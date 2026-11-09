use crate::traits::{FsError, FsOps, TrashOutcome, VerbatimPath};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

pub struct LinuxFsOps;

impl LinuxFsOps {
    pub fn new() -> Self {
        Self
    }

    /// Mendapatkan lokasi direktori XDG Trash untuk pengguna saat ini.
    pub fn get_user_trash_dir() -> PathBuf {
        if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
            if !xdg_data.is_empty() {
                return PathBuf::from(xdg_data).join("Trash");
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".local/share/Trash");
        }

        PathBuf::from("/tmp/Trash")
    }
}

impl Default for LinuxFsOps {
    fn default() -> Self {
        Self::new()
    }
}

impl FsOps for LinuxFsOps {
    fn delete_permanent(&self, path: &VerbatimPath) -> Result<(), FsError> {
        let p = path.as_path();
        if !p.exists() && p.symlink_metadata().is_err() {
            return Err(FsError::NotFound);
        }

        let metadata = p.symlink_metadata().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FsError::NotFound,
            std::io::ErrorKind::PermissionDenied => FsError::PermissionDenied,
            _ => FsError::IoError(e.to_string()),
        })?;

        if metadata.is_dir() {
            fs::remove_dir_all(p).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FsError::NotFound,
                std::io::ErrorKind::PermissionDenied => FsError::PermissionDenied,
                _ => FsError::IoError(e.to_string()),
            })?;
        } else {
            fs::remove_file(p).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FsError::NotFound,
                std::io::ErrorKind::PermissionDenied => FsError::PermissionDenied,
                _ => FsError::IoError(e.to_string()),
            })?;
        }

        Ok(())
    }

    fn move_to_trash(&self, path: &VerbatimPath) -> Result<TrashOutcome, FsError> {
        let p = path.as_path();
        if !p.exists() && p.symlink_metadata().is_err() {
            return Err(FsError::NotFound);
        }

        let trash_dir = Self::get_user_trash_dir();
        let files_dir = trash_dir.join("files");
        let info_dir = trash_dir.join("info");

        fs::create_dir_all(&files_dir).map_err(|e| FsError::IoError(e.to_string()))?;
        fs::create_dir_all(&info_dir).map_err(|e| FsError::IoError(e.to_string()))?;

        let file_name = p
            .file_name()
            .ok_or_else(|| FsError::IoError("Nama berkas tidak valid".to_string()))?;

        let target_file = files_dir.join(file_name);
        let info_file = info_dir.join(format!("{}.trashinfo", file_name.to_string_lossy()));

        // Tulis .trashinfo sesuai spesifikasi XDG Trash
        let now = SystemTime::now();
        let datetime_str = match now.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => format!("{}-01-01T00:00:00", 1970 + d.as_secs() / 31536000),
            Err(_) => "1970-01-01T00:00:00".to_string(),
        };

        let canonical_orig = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        let info_content = format!(
            "[Trash Info]\nPath={}\nDeletionDate={}\n",
            canonical_orig.display(),
            datetime_str
        );

        fs::write(&info_file, info_content).map_err(|e| FsError::IoError(e.to_string()))?;

        // Coba rename atomik
        if let Err(err) = fs::rename(p, &target_file) {
            // Jika lintas partisi (EXDEV = os error 18), fallback
            if err.raw_os_error() == Some(libc::EXDEV) {
                let _ = fs::remove_file(&info_file);
                return Err(FsError::CrossDevice);
            }
            let _ = fs::remove_file(&info_file);
            return Err(FsError::IoError(err.to_string()));
        }

        Ok(TrashOutcome::Success)
    }

    fn path_is_cross_fs(&self, path: &Path, trash_root: &Path) -> bool {
        let meta_a = match path.metadata() {
            Ok(m) => m,
            Err(_) => return false,
        };
        let meta_b = match trash_root.metadata() {
            Ok(m) => m,
            Err(_) => return false,
        };

        meta_a.dev() != meta_b.dev()
    }

    fn open_default(&self, path: &Path) -> Result<(), FsError> {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| FsError::IoError(e.to_string()))?;
        Ok(())
    }

    fn reveal_in_file_manager(&self, path: &Path) -> Result<(), FsError> {
        let parent = path.parent().unwrap_or(path);
        Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| FsError::IoError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_permanent() {
        let test_file = PathBuf::from("target/test_delete_file.txt");
        fs::write(&test_file, b"test content").unwrap();
        assert!(test_file.exists());

        let ops = LinuxFsOps::new();
        let res = ops.delete_permanent(&VerbatimPath(test_file.clone()));
        assert!(res.is_ok());
        assert!(!test_file.exists());
    }
}
