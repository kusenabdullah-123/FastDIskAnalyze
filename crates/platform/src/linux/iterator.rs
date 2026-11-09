use crate::traits::{
    CancellationToken, DirIterator, EntryError, IdentitasBerkas, JenisEntri, RawEntry, ScanError,
};
use std::collections::VecDeque;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const BUF_SIZE: usize = 64 * 1024; // 64 KB buffer per syscall getdents64

struct PendingDir {
    id: u32,
    path: PathBuf,
}

pub struct LinuxDirIterator {
    buffer: Vec<u8>,
    pub lintas_mount: bool,
}

impl LinuxDirIterator {
    pub fn new() -> Self {
        Self {
            buffer: vec![0u8; BUF_SIZE],
            lintas_mount: false,
        }
    }

    pub fn with_lintas_mount(lintas_mount: bool) -> Self {
        Self {
            buffer: vec![0u8; BUF_SIZE],
            lintas_mount,
        }
    }

    /// Mendapatkan stx_dev dari sebuah path
    pub fn get_device_id(path: &Path) -> Result<u64, ScanError> {
        let path_bytes = path.as_os_str().as_bytes();
        let path_cstr = CString::new(path_bytes)
            .map_err(|e| ScanError::Internal(format!("Invalid path CString: {e}")))?;

        let mut stx: MaybeUninit<libc::statx> = MaybeUninit::zeroed();
        let ret = unsafe {
            libc::statx(
                libc::AT_FDCWD,
                path_cstr.as_ptr(),
                libc::AT_SYMLINK_NOFOLLOW | libc::AT_STATX_DONT_SYNC | libc::AT_NO_AUTOMOUNT,
                libc::STATX_TYPE,
                stx.as_mut_ptr(),
            )
        };

        if ret != 0 {
            let err = std::io::Error::last_os_error();
            return Err(ScanError::IoError(format!(
                "Gagal statx direktori root {}: {}",
                path.display(),
                err
            )));
        }

        let stx = unsafe { stx.assume_init() };
        Ok(((stx.stx_dev_major as u64) << 32) | (stx.stx_dev_minor as u64))
    }
}

impl Default for LinuxDirIterator {
    fn default() -> Self {
        Self::new()
    }
}

impl DirIterator for LinuxDirIterator {
    fn read_dir(
        &mut self,
        dir: &Path,
        token: &CancellationToken,
        emit: &mut dyn FnMut(RawEntry<'_>),
    ) -> Result<(), ScanError> {
        let dir_cstr = match CString::new(dir.as_os_str().as_bytes()) {
            Ok(c) => c,
            Err(_) => return Ok(()),
        };

        // Buka direktori dengan O_NOFOLLOW untuk keamanan dari symlink loop
        let dir_fd = unsafe {
            libc::open(
                dir_cstr.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };

        if dir_fd < 0 {
            return Ok(());
        }

        let mut checked_counter: u32 = 0;

        loop {
            checked_counter += 1;
            if checked_counter & 511 == 0 {
                token.check()?;
            }

            let nread = unsafe {
                libc::syscall(
                    libc::SYS_getdents64,
                    dir_fd,
                    self.buffer.as_mut_ptr(),
                    self.buffer.len(),
                )
            };

            if nread <= 0 {
                break;
            }

            let mut offset = 0;
            let nread = nread as usize;

            while offset < nread {
                let d_reclen = unsafe {
                    let ptr = self.buffer.as_ptr().add(offset + 16) as *const u16;
                    ptr.read_unaligned()
                } as usize;

                let d_type = self.buffer[offset + 18];
                let name_start = offset + 19;

                let mut name_end = name_start;
                while name_end < offset + d_reclen && self.buffer[name_end] != 0 {
                    name_end += 1;
                }

                let name_bytes = &self.buffer[name_start..name_end];

                if name_bytes == b"." || name_bytes == b".." {
                    offset += d_reclen;
                    continue;
                }

                let name_c_ptr = unsafe { self.buffer.as_ptr().add(name_start) as *const libc::c_char };
                let mut stx: MaybeUninit<libc::statx> = MaybeUninit::zeroed();

                let stat_ret = unsafe {
                    libc::statx(
                        dir_fd,
                        name_c_ptr,
                        libc::AT_SYMLINK_NOFOLLOW | libc::AT_STATX_DONT_SYNC | libc::AT_NO_AUTOMOUNT,
                        libc::STATX_TYPE
                            | libc::STATX_SIZE
                            | libc::STATX_BLOCKS
                            | libc::STATX_NLINK
                            | libc::STATX_INO,
                        stx.as_mut_ptr(),
                    )
                };

                let (jenis, ukuran_asli, ukuran_blok, nlink, dev, ino, err) = if stat_ret == 0 {
                    let s = unsafe { stx.assume_init() };
                    let mode = s.stx_mode as u32;
                    let jenis = if (mode & libc::S_IFMT as u32) == libc::S_IFDIR as u32 {
                        JenisEntri::Direktori
                    } else if (mode & libc::S_IFMT as u32) == libc::S_IFLNK as u32 {
                        JenisEntri::Tautan
                    } else if (mode & libc::S_IFMT as u32) == libc::S_IFREG as u32 {
                        JenisEntri::Berkas
                    } else {
                        JenisEntri::Lainnya
                    };

                    let dev = ((s.stx_dev_major as u64) << 32) | (s.stx_dev_minor as u64);
                    (
                        jenis,
                        s.stx_size,
                        s.stx_blocks * 512,
                        s.stx_nlink,
                        dev,
                        s.stx_ino as u128,
                        None,
                    )
                } else {
                    let j = match d_type {
                        libc::DT_DIR => JenisEntri::Direktori,
                        libc::DT_LNK => JenisEntri::Tautan,
                        libc::DT_REG => JenisEntri::Berkas,
                        _ => JenisEntri::Lainnya,
                    };
                    (j, 0, 0, 1, 0, 0, Some(EntryError::PermissionDenied))
                };

                let identitas = if nlink > 1 {
                    Some(IdentitasBerkas {
                        volume: dev,
                        berkas: ino,
                    })
                } else {
                    None
                };

                emit(RawEntry {
                    induk: 0,
                    nama: name_bytes,
                    jenis,
                    ukuran_asli,
                    ukuran_blok,
                    jumlah_tautan: nlink,
                    identitas,
                    error: err,
                });

                offset += d_reclen;
            }
        }

        unsafe {
            libc::close(dir_fd);
        }

        Ok(())
    }

    fn walk(
        &mut self,
        root: &Path,
        token: &CancellationToken,
        emit: &mut dyn FnMut(RawEntry<'_>),
    ) -> Result<(), ScanError> {
        let root_canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let root_dev = Self::get_device_id(&root_canonical)?;

        let mut root_stx: MaybeUninit<libc::statx> = MaybeUninit::zeroed();
        let root_cstr = CString::new(root_canonical.as_os_str().as_bytes())
            .map_err(|e| ScanError::Internal(format!("Invalid root CString: {e}")))?;

        let ret = unsafe {
            libc::statx(
                libc::AT_FDCWD,
                root_cstr.as_ptr(),
                libc::AT_SYMLINK_NOFOLLOW | libc::AT_STATX_DONT_SYNC | libc::AT_NO_AUTOMOUNT,
                libc::STATX_TYPE | libc::STATX_SIZE | libc::STATX_BLOCKS | libc::STATX_NLINK | libc::STATX_INO,
                root_stx.as_mut_ptr(),
            )
        };

        let (root_size, root_blocks, root_nlink, root_ino) = if ret == 0 {
            let s = unsafe { root_stx.assume_init() };
            (s.stx_size, s.stx_blocks * 512, s.stx_nlink, s.stx_ino as u128)
        } else {
            (0, 0, 1, 0)
        };

        let root_name = root_canonical
            .file_name()
            .map(|n| n.as_bytes())
            .unwrap_or(b"/");

        let root_identitas = if root_nlink > 1 {
            Some(IdentitasBerkas {
                volume: root_dev,
                berkas: root_ino,
            })
        } else {
            None
        };

        emit(RawEntry {
            induk: u32::MAX, // INDUK_AKAR
            nama: root_name,
            jenis: JenisEntri::Direktori,
            ukuran_asli: root_size,
            ukuran_blok: root_blocks,
            jumlah_tautan: root_nlink,
            identitas: root_identitas,
            error: None,
        });

        let mut next_id: u32 = 1;
        let mut pending_dirs: VecDeque<PendingDir> = VecDeque::new();
        pending_dirs.push_back(PendingDir {
            id: 0,
            path: root_canonical,
        });

        let lintas_mount = self.lintas_mount;

        while let Some(current_dir) = pending_dirs.pop_front() {
            token.check()?;

            let mut sub_dirs = Vec::new();
            let parent_id = current_dir.id;

            self.read_dir(&current_dir.path, token, &mut |mut raw| {
                let entry_id = next_id;
                next_id += 1;
                raw.induk = parent_id;

                let is_dir = raw.jenis == JenisEntri::Direktori;
                let dev = raw.identitas.map(|id| id.volume).unwrap_or(root_dev);
                let name_bytes = raw.nama;

                emit(raw);

                if is_dir && (lintas_mount || dev == root_dev) {
                    let mut child_path = current_dir.path.clone();
                    #[cfg(unix)]
                    {
                        use std::ffi::OsStr;
                        child_path.push(OsStr::from_bytes(name_bytes));
                    }
                    #[cfg(not(unix))]
                    {
                        let s = String::from_utf8_lossy(name_bytes);
                        child_path.push(s.as_ref());
                    }

                    sub_dirs.push(PendingDir {
                        id: entry_id,
                        path: child_path,
                    });
                }
            })?;

            for sd in sub_dirs {
                pending_dirs.push_back(sd);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_dir_iterator_walk_local_dir() {
        let mut iterator = LinuxDirIterator::new();
        let token = CancellationToken::new();
        let mut count = 0;
        let mut found_lib = false;

        let res = iterator.walk(Path::new("src"), &token, &mut |raw| {
            count += 1;
            if raw.nama == b"lib.rs" {
                found_lib = true;
                assert_eq!(raw.jenis, JenisEntri::Berkas);
                assert!(raw.ukuran_asli > 0);
            }
        });

        assert!(res.is_ok());
        assert!(count >= 2);
        assert!(found_lib);
    }

    #[test]
    fn test_symlink_loop_safety() {
        use std::fs;
        use std::os::unix::fs::symlink;

        let test_dir = PathBuf::from("target/test_symlink_dir");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let sub_dir = test_dir.join("sub");
        fs::create_dir_all(&sub_dir).unwrap();

        let loop_link = sub_dir.join("loop_link");
        let _ = symlink(&test_dir, &loop_link);

        let mut iterator = LinuxDirIterator::new();
        let token = CancellationToken::new();
        let loop_traversed = false;

        let res = iterator.walk(&test_dir, &token, &mut |raw| {
            if raw.nama == b"loop_link" {
                assert_eq!(raw.jenis, JenisEntri::Tautan);
            }
        });

        assert!(res.is_ok());
        assert!(!loop_traversed);

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cancellation() {
        let mut iterator = LinuxDirIterator::new();
        let token = CancellationToken::new();
        token.cancel();

        let res = iterator.walk(Path::new("."), &token, &mut |_| {});
        assert!(matches!(res, Err(ScanError::Cancelled)));
    }
}
