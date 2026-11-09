use crate::traits::{
    CancellationToken, DirIterator, IdentitasBerkas, JenisEntri, RawEntry, ScanError,
};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::mem::MaybeUninit;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFileInformationByHandle, GetFileInformationByHandleEx,
    FileIdBothDirectoryInfo, FileIdBothDirectoryRestartInfo, BY_HANDLE_FILE_INFORMATION,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_ID_BOTH_DIR_INFO, FILE_LIST_DIRECTORY, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};

const BUF_SIZE: usize = 64 * 1024; // 64 KB buffer per panggilan GetFileInformationByHandleEx

struct PendingDir {
    id: u32,
    path: PathBuf,
}

pub struct WindowsDirIterator {
    buffer: Vec<u8>,
    pub lintas_mount: bool,
}

impl WindowsDirIterator {
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

    /// Konversi path ke format \\?\ untuk mendukung path panjang > 260 karakter
    fn to_verbatim_wide(path: &Path) -> Vec<u16> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path_str = canonical.to_string_lossy();

        let verbatim_str = if path_str.starts_with(r"\\?\") {
            path_str.to_string()
        } else if let Some(stripped) = path_str.strip_prefix(r"\\") {
            // UNC path: \\server\share -> \\?\UNC\server\share
            format!(r"\\?\UNC\{stripped}")
        } else {
            format!(r"\\?\{path_str}")
        };

        OsStr::new(&verbatim_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Buka direktori untuk enumerasi batch menggunakan FILE_LIST_DIRECTORY & FILE_FLAG_BACKUP_SEMANTICS
    fn open_dir_handle(path: &Path) -> Result<HANDLE, ScanError> {
        let wide_path = Self::to_verbatim_wide(path);
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let err = std::io::Error::last_os_error();
            Err(ScanError::IoError(format!(
                "Gagal membuka direktori Windows {}: {}",
                path.display(),
                err
            )))
        } else {
            Ok(handle)
        }
    }

    /// Ambil serial number volume dari handle direktori
    fn get_volume_serial(handle: HANDLE) -> u64 {
        let mut info: MaybeUninit<BY_HANDLE_FILE_INFORMATION> = MaybeUninit::zeroed();
        let ret = unsafe { GetFileInformationByHandle(handle, info.as_mut_ptr()) };
        if ret != 0 {
            let info = unsafe { info.assume_init() };
            info.dwVolumeSerialNumber as u64
        } else {
            0
        }
    }
}

impl Default for WindowsDirIterator {
    fn default() -> Self {
        Self::new()
    }
}

impl DirIterator for WindowsDirIterator {
    fn read_dir(
        &mut self,
        dir: &Path,
        token: &CancellationToken,
        emit: &mut dyn FnMut(RawEntry<'_>),
    ) -> Result<(), ScanError> {
        let dir_handle = match Self::open_dir_handle(dir) {
            Ok(h) => h,
            Err(_) => return Ok(()),
        };

        let volume_serial = Self::get_volume_serial(dir_handle);
        let mut is_restart = true;
        let mut checked_counter: u32 = 0;

        loop {
            checked_counter += 1;
            if checked_counter & 511 == 0 {
                token.check()?;
            }

            let info_class = if is_restart {
                FileIdBothDirectoryRestartInfo
            } else {
                FileIdBothDirectoryInfo
            };

            let success = unsafe {
                GetFileInformationByHandleEx(
                    dir_handle,
                    info_class,
                    self.buffer.as_mut_ptr() as *mut _,
                    self.buffer.len() as u32,
                )
            };

            if success == 0 {
                let err = unsafe { GetLastError() };
                if err == ERROR_NO_MORE_FILES {
                    break;
                }
                break;
            }

            is_restart = false;
            let mut offset = 0;

            loop {
                let entry = unsafe {
                    &*(self.buffer.as_ptr().add(offset) as *const FILE_ID_BOTH_DIR_INFO)
                };

                let name_len_u16 = (entry.FileNameLength / 2) as usize;
                let name_u16_slice = unsafe {
                    std::slice::from_raw_parts(entry.FileName.as_ptr(), name_len_u16)
                };

                let is_dot = name_u16_slice == [b'.' as u16];
                let is_dotdot = name_u16_slice == [b'.' as u16, b'.' as u16];

                if !is_dot && !is_dotdot {
                    let name_string = String::from_utf16_lossy(name_u16_slice);
                    let name_bytes = name_string.as_bytes();

                    let is_reparse = (entry.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0;
                    let is_dir = (entry.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;

                    let jenis = if is_reparse {
                        JenisEntri::Tautan
                    } else if is_dir {
                        JenisEntri::Direktori
                    } else {
                        JenisEntri::Berkas
                    };

                    let apparent_size = if entry.EndOfFile >= 0 {
                        entry.EndOfFile as u64
                    } else {
                        0
                    };

                    let block_size = if entry.AllocationSize >= 0 {
                        entry.AllocationSize as u64
                    } else {
                        0
                    };

                    let identitas = Some(IdentitasBerkas {
                        volume: volume_serial,
                        berkas: entry.FileId as u128,
                    });

                    emit(RawEntry {
                        induk: 0,
                        nama: name_bytes,
                        jenis,
                        ukuran_asli: apparent_size,
                        ukuran_blok: block_size,
                        jumlah_tautan: 1,
                        identitas,
                        error: None,
                    });
                }

                if entry.NextEntryOffset == 0 {
                    break;
                }
                offset += entry.NextEntryOffset as usize;
                if offset >= self.buffer.len() {
                    break;
                }
            }
        }

        unsafe {
            CloseHandle(dir_handle);
        }

        Ok(())
    }

    fn walk(
        &mut self,
        root: &Path,
        token: &CancellationToken,
        emit: &mut dyn FnMut(RawEntry<'_>),
    ) -> Result<(), ScanError> {
        let root_handle = Self::open_dir_handle(root)?;
        let root_volume = Self::get_volume_serial(root_handle);

        let mut root_info: MaybeUninit<BY_HANDLE_FILE_INFORMATION> = MaybeUninit::zeroed();
        let (root_size, root_blocks, root_nlink, root_file_id) = unsafe {
            if GetFileInformationByHandle(root_handle, root_info.as_mut_ptr()) != 0 {
                let info = root_info.assume_init();
                let size = ((info.nFileSizeHigh as u64) << 32) | (info.nFileSizeLow as u64);
                let id = ((info.nFileIndexHigh as u128) << 32) | (info.nFileIndexLow as u128);
                (size, size, info.nNumberOfLinks, id)
            } else {
                (0, 0, 1, 0)
            }
        };

        unsafe {
            CloseHandle(root_handle);
        }

        let root_canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let root_name_str = root_canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        let root_name_bytes = root_name_str.as_bytes();

        let root_identitas = if root_nlink > 1 {
            Some(IdentitasBerkas {
                volume: root_volume,
                berkas: root_file_id,
            })
        } else {
            None
        };

        emit(RawEntry {
            induk: u32::MAX,
            nama: root_name_bytes,
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

        while let Some(current_dir) = pending_dirs.pop_front() {
            token.check()?;

            let mut sub_dirs = Vec::new();
            let parent_id = current_dir.id;

            self.read_dir(&current_dir.path, token, &mut |mut raw| {
                let entry_id = next_id;
                next_id += 1;
                raw.induk = parent_id;

                let is_dir = raw.jenis == JenisEntri::Direktori;
                let name_string = String::from_utf8_lossy(raw.nama).to_string();

                emit(raw);

                if is_dir {
                    let mut child_path = current_dir.path.clone();
                    child_path.push(name_string);
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
