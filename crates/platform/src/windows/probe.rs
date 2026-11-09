use crate::traits::{InformasiDrive, MediumKind, StorageProbe};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW, GetVolumeInformationW,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::WindowsProgramming::{
    DRIVE_CDROM, DRIVE_FIXED, DRIVE_RAMDISK, DRIVE_REMOVABLE, DRIVE_REMOTE,
};
use windows_sys::Win32::System::Ioctl::{
    PropertyStandardQuery, StorageDeviceProperty, StorageDeviceSeekPenaltyProperty,
    DEVICE_SEEK_PENALTY_DESCRIPTOR, IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_DEVICE_DESCRIPTOR,
    STORAGE_PROPERTY_QUERY,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

pub struct WindowsStorageProbe;

impl WindowsStorageProbe {
    pub fn new() -> Self {
        Self
    }

    /// Konversi &Path atau string menjadi vektor UTF-16 null-terminated
    fn to_wide_null(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Ambil prefix drive (mis. `C:\` atau `\\?\C:\`) dari path
    fn get_drive_root(path: &Path) -> Option<Vec<u16>> {
        let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path_str = abs_path.to_str()?;

        // Cek pola drive letter: mis. C:\
        let drive_prefix = if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
            stripped.get(0..3)? // "C:\"
        } else {
            path_str.get(0..3)?
        };

        if drive_prefix.chars().nth(1) == Some(':') {
            let mut wide: Vec<u16> = OsStr::new(drive_prefix).encode_wide().collect();
            wide.push(0);
            Some(wide)
        } else {
            None
        }
    }

    /// Buka handle ke volume fisik (mis. `\\.\C:`)
    fn open_volume_handle(path: &Path) -> Option<windows_sys::Win32::Foundation::HANDLE> {
        let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path_str = abs_path.to_str()?;

        let drive_letter = if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
            stripped.chars().next()?
        } else {
            path_str.chars().next()?
        };

        if !drive_letter.is_ascii_alphabetic() {
            return None;
        }

        let volume_path = format!(r"\\.\{}:", drive_letter.to_ascii_uppercase());
        let wide_volume = Self::to_wide_null(OsStr::new(&volume_path));

        let handle = unsafe {
            CreateFileW(
                wide_volume.as_ptr(),
                0, // Tanpa akses baca/tulis khusus, hanya query metadata
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(handle)
        }
    }
}

impl Default for WindowsStorageProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageProbe for WindowsStorageProbe {
    fn probe(&self, path: &Path) -> MediumKind {
        // 1. Cek GetDriveTypeW terlebih dahulu
        if let Some(drive_root) = Self::get_drive_root(path) {
            let drive_type = unsafe { GetDriveTypeW(drive_root.as_ptr()) };
            match drive_type {
                DRIVE_REMOTE => return MediumKind::Network,
                DRIVE_REMOVABLE | DRIVE_CDROM => return MediumKind::Removable,
                DRIVE_RAMDISK => return MediumKind::SataSsd,
                DRIVE_FIXED => {
                    // Drive lokal fixed, lanjutkan query DeviceIoControl
                }
                _ => return MediumKind::Unknown,
            }
        }

        // 2. Buka handle volume untuk DeviceIoControl
        let handle = match Self::open_volume_handle(path) {
            Some(h) => h,
            None => return MediumKind::Unknown,
        };

        // Query A: BusType (NVMe vs SATA vs USB)
        let mut query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };

        let mut desc: STORAGE_DEVICE_DESCRIPTOR = unsafe { std::mem::zeroed() };
        let mut bytes_returned: u32 = 0;

        let ret = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                &mut query as *mut _ as *mut _,
                std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                &mut desc as *mut _ as *mut _,
                std::mem::size_of::<STORAGE_DEVICE_DESCRIPTOR>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        let is_nvme = if ret != 0 {
            // BusTypeNvme = 17 di Windows DDK
            desc.BusType == 17
        } else {
            false
        };

        // Query B: SeekPenalty (HDD vs SSD)
        let mut seek_query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceSeekPenaltyProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };

        let mut seek_desc: DEVICE_SEEK_PENALTY_DESCRIPTOR = unsafe { std::mem::zeroed() };
        let mut bytes_returned2: u32 = 0;

        let ret_seek = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                &mut seek_query as *mut _ as *mut _,
                std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                &mut seek_desc as *mut _ as *mut _,
                std::mem::size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32,
                &mut bytes_returned2,
                std::ptr::null_mut(),
            )
        };

        unsafe {
            CloseHandle(handle);
        }

        if ret_seek != 0 {
            if seek_desc.IncursSeekPenalty != 0 {
                // Ada seek penalty => piringan berputar (HDD)
                return MediumKind::Hdd;
            } else {
                // Tanpa seek penalty => Solid State Drive
                if is_nvme {
                    return MediumKind::NvmeSsd;
                } else {
                    return MediumKind::SataSsd;
                }
            }
        }

        if is_nvme {
            MediumKind::NvmeSsd
        } else {
            MediumKind::Unknown
        }
    }

    fn list_drives(&self) -> Vec<InformasiDrive> {
        let mut drives = Vec::new();

        // 1. Profil Pengguna Windows (C:\Users\<Name>)
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let profile_path = PathBuf::from(&user_profile);
            if profile_path.exists() {
                let wide_path = Self::to_wide_null(OsStr::new(&user_profile));
                let mut free_bytes_avail: u64 = 0;
                let mut total_bytes: u64 = 0;
                let mut total_free: u64 = 0;

                let ok = unsafe {
                    GetDiskFreeSpaceExW(
                        wide_path.as_ptr(),
                        &mut free_bytes_avail,
                        &mut total_bytes,
                        &mut total_free,
                    )
                };

                if ok != 0 && total_bytes > 0 {
                    let medium = self.probe(&profile_path);
                    drives.push(InformasiDrive {
                        id: format!("user-profile:{}", user_profile),
                        mount_point: profile_path,
                        label: "Home Pengguna (~/)".to_string(),
                        fs_type: "directory".to_string(),
                        total_bytes,
                        available_bytes: free_bytes_avail,
                        medium,
                    });
                }
            }
        }
        let mut buffer = [0u16; 256];
        let len = unsafe { GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr()) };
        if len == 0 || len as usize >= buffer.len() {
            return drives;
        }

        let mut offset = 0;
        while offset < len as usize && buffer[offset] != 0 {
            let mut end = offset;
            while end < len as usize && buffer[end] != 0 {
                end += 1;
            }

            let drive_slice = &buffer[offset..end];
            let drive_ptr = buffer[offset..].as_ptr();
            let drive_str = String::from_utf16_lossy(drive_slice);
            let mount_point = PathBuf::from(&drive_str);

            offset = end + 1;

            let drive_type = unsafe { GetDriveTypeW(drive_ptr) };
            if drive_type != DRIVE_FIXED && drive_type != DRIVE_REMOVABLE {
                continue;
            }

            let mut free_bytes_avail: u64 = 0;
            let mut total_bytes: u64 = 0;
            let mut total_free: u64 = 0;

            let ok = unsafe {
                GetDiskFreeSpaceExW(
                    drive_ptr,
                    &mut free_bytes_avail,
                    &mut total_bytes,
                    &mut total_free,
                )
            };

            if ok == 0 || total_bytes == 0 {
                continue;
            }

            let mut vol_name = [0u16; 260];
            let mut fs_name = [0u16; 260];
            let mut serial = 0u32;
            let mut max_comp = 0u32;
            let mut flags = 0u32;

            unsafe {
                GetVolumeInformationW(
                    drive_ptr,
                    vol_name.as_mut_ptr(),
                    vol_name.len() as u32,
                    &mut serial,
                    &mut max_comp,
                    &mut flags,
                    fs_name.as_mut_ptr(),
                    fs_name.len() as u32,
                );
            }

            let vol_name_str = String::from_utf16_lossy(&vol_name)
                .trim_matches('\0')
                .to_string();
            let fs_name_str = String::from_utf16_lossy(&fs_name)
                .trim_matches('\0')
                .to_string();

            let label = if vol_name_str.is_empty() {
                format!("Local Disk ({})", drive_str.trim_end_matches('\\'))
            } else {
                format!("{} ({})", vol_name_str, drive_str.trim_end_matches('\\'))
            };

            let medium = self.probe(&mount_point);

            drives.push(InformasiDrive {
                id: drive_str,
                mount_point,
                label,
                fs_type: if fs_name_str.is_empty() {
                    "NTFS".to_string()
                } else {
                    fs_name_str
                },
                total_bytes,
                available_bytes: free_bytes_avail,
                medium,
            });
        }

        drives
    }
}
