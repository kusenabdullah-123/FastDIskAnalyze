use crate::traits::{FsError, FsOps, TrashOutcome, VerbatimPath};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows_sys::Win32::Storage::FileSystem::{DeleteFileW, RemoveDirectoryW};
use windows_sys::Win32::UI::Shell::{
    SHFileOperationW, ShellExecuteW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT,
    FO_DELETE, SHFILEOPSTRUCTW,
};

pub struct WindowsFsOps;

impl WindowsFsOps {
    pub fn new() -> Self {
        Self
    }

    /// Konversi path ke format \\?\ null-terminated untuk Win32 API
    fn to_verbatim_wide(path: &Path) -> Vec<u16> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path_str = canonical.to_string_lossy();

        let verbatim_str = if path_str.starts_with(r"\\?\") {
            path_str.to_string()
        } else if let Some(stripped) = path_str.strip_prefix(r"\\") {
            format!(r"\\?\UNC\{stripped}")
        } else {
            format!(r"\\?\{path_str}")
        };

        OsStr::new(&verbatim_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Konversi path ke format double-null-terminated untuk SHFileOperationW
    fn to_double_null_wide(path: &Path) -> Vec<u16> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path_str = canonical.to_string_lossy();

        let normal_str = if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
            stripped.to_string()
        } else {
            path_str.to_string()
        };

        let mut wide: Vec<u16> = OsStr::new(&normal_str).encode_wide().collect();
        wide.push(0);
        wide.push(0); // double-null
        wide
    }

    /// Dapatkan huruf drive (mis. 'C') dari sebuah path
    fn get_drive_letter(path: &Path) -> Option<char> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path_str = canonical.to_string_lossy();

        let s = if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
            stripped
        } else {
            &path_str
        };

        let mut chars = s.chars();
        let letter = chars.next()?;
        if chars.next() == Some(':') {
            Some(letter.to_ascii_uppercase())
        } else {
            None
        }
    }
}

impl Default for WindowsFsOps {
    fn default() -> Self {
        Self::new()
    }
}

impl FsOps for WindowsFsOps {
    fn delete_permanent(&self, path: &VerbatimPath) -> Result<(), FsError> {
        let p = path.as_path();
        if !p.exists() {
            return Err(FsError::NotFound);
        }

        let is_dir = p.is_dir();
        let wide_path = Self::to_verbatim_wide(p);

        let success = unsafe {
            if is_dir {
                // Untuk direktori di Windows, gunakan RemoveDirectoryW
                RemoveDirectoryW(wide_path.as_ptr())
            } else {
                DeleteFileW(wide_path.as_ptr())
            }
        };

        if success == 0 {
            let err = std::io::Error::last_os_error();
            Err(FsError::IoError(format!("Gagal menghapus {}: {}", p.display(), err)))
        } else {
            Ok(())
        }
    }

    fn move_to_trash(&self, path: &VerbatimPath) -> Result<TrashOutcome, FsError> {
        let p = path.as_path();
        if !p.exists() {
            return Err(FsError::NotFound);
        }

        let mut double_null = Self::to_double_null_wide(p);

        let mut file_op = SHFILEOPSTRUCTW {
            hwnd: std::ptr::null_mut(),
            wFunc: FO_DELETE,
            pFrom: double_null.as_mut_ptr(),
            pTo: std::ptr::null(),
            fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI) as u16,
            fAnyOperationsAborted: 0,
            hNameMappings: std::ptr::null_mut(),
            lpszProgressTitle: std::ptr::null(),
        };

        let ret = unsafe { SHFileOperationW(&mut file_op) };

        if ret != 0 {
            Err(FsError::IoError(format!(
                "SHFileOperationW gagal memindahkan {} ke Recycle Bin (kode error {})",
                p.display(),
                ret
            )))
        } else if file_op.fAnyOperationsAborted != 0 {
            Err(FsError::IoError("Operasi pemindahan ke Recycle Bin dibatalkan".to_string()))
        } else {
            Ok(TrashOutcome::Success)
        }
    }

    fn path_is_cross_fs(&self, path: &Path, trash_root: &Path) -> bool {
        let drive_a = Self::get_drive_letter(path);
        let drive_b = Self::get_drive_letter(trash_root);

        match (drive_a, drive_b) {
            (Some(a), Some(b)) => a != b,
            _ => true,
        }
    }

    fn open_default(&self, path: &Path) -> Result<(), FsError> {
        let wide_path = Self::to_verbatim_wide(path);
        let op: Vec<u16> = OsStr::new("open").encode_wide().chain(std::iter::once(0)).collect();

        let ret = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                op.as_ptr(),
                wide_path.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            )
        };

        if (ret as isize) <= 32 {
            Err(FsError::IoError("Gagal membuka berkas dengan program default".to_string()))
        } else {
            Ok(())
        }
    }

    fn reveal_in_file_manager(&self, path: &Path) -> Result<(), FsError> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let arg = format!("/select,\"{}\"", canonical.display());
        let wide_arg: Vec<u16> = OsStr::new(&arg).encode_wide().chain(std::iter::once(0)).collect();
        let explorer: Vec<u16> = OsStr::new("explorer.exe").encode_wide().chain(std::iter::once(0)).collect();

        let ret = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                std::ptr::null(),
                explorer.as_ptr(),
                wide_arg.as_ptr(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            )
        };

        if (ret as isize) <= 32 {
            Err(FsError::IoError("Gagal menampilkan berkas di File Explorer".to_string()))
        } else {
            Ok(())
        }
    }
}
