use crate::traits::{MediumKind, StorageProbe};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Entri mount yang diekstrak dari /proc/mounts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountEntry {
    pub device: String,
    pub mount_point: PathBuf,
    pub fs_type: String,
}

pub struct LinuxStorageProbe;

impl LinuxStorageProbe {
    pub fn new() -> Self {
        Self
    }

    /// Melakukan unescape urutan oktal standar /proc/mounts (mis. `\040` -> spasi, `\011` -> tab, `\134` -> `\`).
    pub fn unescape_octal(input: &str) -> String {
        let bytes = input.as_bytes();
        let mut result = Vec::with_capacity(bytes.len());
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'\\' && i + 3 < bytes.len() {
                let d1 = bytes[i + 1];
                let d2 = bytes[i + 2];
                let d3 = bytes[i + 3];

                if (b'0'..=b'7').contains(&d1)
                    && (b'0'..=b'7').contains(&d2)
                    && (b'0'..=b'7').contains(&d3)
                {
                    let octal_val = (d1 - b'0') * 64 + (d2 - b'0') * 8 + (d3 - b'0');
                    result.push(octal_val);
                    i += 4;
                    continue;
                }
            }
            result.push(bytes[i]);
            i += 1;
        }

        String::from_utf8_lossy(&result).into_owned()
    }

    /// Parsing `/proc/mounts` dan menemukan entri mount dengan prefix terpanjang yang cocok dengan `target_path`.
    pub fn parse_mounts_matching<R: BufRead>(reader: R, target_path: &Path) -> Option<MountEntry> {
        let canonical_target = target_path.canonicalize().unwrap_or_else(|_| target_path.to_path_buf());
        let mut best_match: Option<MountEntry> = None;
        let mut longest_prefix_len = 0;

        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => continue,
            };

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }

            let device = Self::unescape_octal(parts[0]);
            let mount_point_str = Self::unescape_octal(parts[1]);
            let fs_type = parts[2].to_string();
            let mount_point = PathBuf::from(&mount_point_str);

            if canonical_target.starts_with(&mount_point) {
                let prefix_len = mount_point.as_os_str().len();
                if prefix_len >= longest_prefix_len {
                    longest_prefix_len = prefix_len;
                    best_match = Some(MountEntry {
                        device,
                        mount_point,
                        fs_type,
                    });
                }
            }
        }

        best_match
    }

    /// Menemukan entri mount untuk path dari `/proc/mounts` sistem saat ini.
    pub fn find_mount_entry(target_path: &Path) -> Option<MountEntry> {
        let file = File::open("/proc/mounts").ok()?;
        Self::parse_mounts_matching(BufReader::new(file), target_path)
    }

    /// Memeriksa apakah tipe sistem berkas adalah filesystem jaringan.
    pub fn is_network_fs(fs_type: &str, device: &str) -> bool {
        let fs = fs_type.to_ascii_lowercase();
        if matches!(
            fs.as_str(),
            "nfs"
                | "nfs4"
                | "cifs"
                | "smbfs"
                | "sshfs"
                | "fuse.sshfs"
                | "davfs"
                | "fuse.davfs"
                | "glusterfs"
                | "lustre"
                | "ceph"
        ) {
            return true;
        }

        // Jalur UNC Windows atau host:/share di Linux
        if device.starts_with("//") || (device.contains(':') && !device.starts_with("/dev/")) {
            return true;
        }

        false
    }

    /// Memeriksa sysfs untuk menentukan tipe medium dari nama perangkat blok (mis. `sda1`, `nvme0n1p2`, `dm-0`).
    pub fn probe_device_sysfs(dev_name: &str) -> MediumKind {
        // 1. Cek apakah removable
        let removable_paths = [
            format!("/sys/class/block/{dev_name}/removable"),
            format!("/sys/class/block/{dev_name}/../removable"),
        ];

        for p in &removable_paths {
            if let Ok(content) = std::fs::read_to_string(p) {
                if content.trim() == "1" {
                    return MediumKind::Removable;
                }
            }
        }

        // 2. Jika nama perangkat mengandung 'nvme'
        if dev_name.contains("nvme") {
            return MediumKind::NvmeSsd;
        }

        // 3. Jika device-mapper (LVM / LUKS / dm-*), periksa slave devices
        let slaves_dir = format!("/sys/class/block/{dev_name}/slaves");
        if let Ok(entries) = std::fs::read_dir(&slaves_dir) {
            let mut found_any = false;
            let mut all_nvme = true;
            let mut has_rotational = false;

            for entry_res in entries.flatten() {
                found_any = true;
                let slave_name = entry_res.file_name().to_string_lossy().to_string();
                let slave_medium = Self::probe_device_sysfs(&slave_name);

                if slave_medium == MediumKind::Hdd {
                    has_rotational = true;
                }
                if slave_medium != MediumKind::NvmeSsd {
                    all_nvme = false;
                }
            }

            if found_any {
                if has_rotational {
                    return MediumKind::Hdd;
                }
                if all_nvme {
                    return MediumKind::NvmeSsd;
                }
                return MediumKind::SataSsd;
            }
        }

        // 4. Periksa queue/rotational
        let rotational_paths = [
            format!("/sys/class/block/{dev_name}/queue/rotational"),
            format!("/sys/class/block/{dev_name}/../queue/rotational"),
        ];

        for p in &rotational_paths {
            if let Ok(content) = std::fs::read_to_string(p) {
                match content.trim() {
                    "1" => return MediumKind::Hdd,
                    "0" => return MediumKind::SataSsd,
                    _ => {}
                }
            }
        }

        MediumKind::Unknown
    }
}

impl Default for LinuxStorageProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageProbe for LinuxStorageProbe {
    fn probe(&self, path: &Path) -> MediumKind {
        let mount_entry = match Self::find_mount_entry(path) {
            Some(entry) => entry,
            None => return MediumKind::Unknown,
        };

        // 1. Cek filesystem jaringan
        if Self::is_network_fs(&mount_entry.fs_type, &mount_entry.device) {
            return MediumKind::Network;
        }

        // 2. Ekstrak nama perangkat dari path /dev/
        if let Some(dev_name) = mount_entry.device.strip_prefix("/dev/") {
            let kind = Self::probe_device_sysfs(dev_name);
            if kind != MediumKind::Unknown {
                return kind;
            }
        }

        // 3. Fallback jika tmpfs/ramfs atau sistem berkas virtual
        if mount_entry.fs_type == "tmpfs" || mount_entry.fs_type == "ramfs" {
            return MediumKind::SataSsd;
        }

        MediumKind::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_unescape_octal() {
        assert_eq!(LinuxStorageProbe::unescape_octal(r"/home/user/Data\040Kerja"), "/home/user/Data Kerja");
        assert_eq!(LinuxStorageProbe::unescape_octal(r"/mnt/tab\011dir"), "/mnt/tab\tdir");
        assert_eq!(LinuxStorageProbe::unescape_octal(r"/mnt/slash\134dir"), "/mnt/slash\\dir");
        assert_eq!(LinuxStorageProbe::unescape_octal(r"/mnt/biasa"), "/mnt/biasa");
    }

    #[test]
    fn test_parse_mounts_matching_longest_prefix() {
        let fake_mounts = r#"
/dev/sda1 / ext4 rw,relatime 0 0
/dev/sdb1 /home ext4 rw,relatime 0 0
/dev/nvme0n1p1 /home/kusena/rust btrfs rw,relatime 0 0
192.168.1.50:/share /mnt/nfs nfs4 rw,relatime 0 0
"#;

        let entry = LinuxStorageProbe::parse_mounts_matching(
            Cursor::new(fake_mounts),
            Path::new("/home/kusena/rust/disk-analize"),
        )
        .expect("Harus menemukan mount entry");

        assert_eq!(entry.device, "/dev/nvme0n1p1");
        assert_eq!(entry.mount_point, PathBuf::from("/home/kusena/rust"));
        assert_eq!(entry.fs_type, "btrfs");
    }

    #[test]
    fn test_network_fs_detection() {
        assert!(LinuxStorageProbe::is_network_fs("nfs", "/dev/null"));
        assert!(LinuxStorageProbe::is_network_fs("nfs4", "server:/path"));
        assert!(LinuxStorageProbe::is_network_fs("cifs", "//192.168.1.1/share"));
        assert!(!LinuxStorageProbe::is_network_fs("ext4", "/dev/sda1"));
        assert!(!LinuxStorageProbe::is_network_fs("btrfs", "/dev/nvme0n1p2"));
    }
}
