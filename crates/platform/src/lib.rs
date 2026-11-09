pub mod traits;
pub use traits::*;

#[cfg(unix)]
pub mod linux;

#[cfg(windows)]
pub mod windows;

/// Buat probe default untuk platform saat ini.
pub fn default_probe() -> Box<dyn StorageProbe> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsStorageProbe::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxStorageProbe::new())
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        struct FallbackProbe;
        impl StorageProbe for FallbackProbe {
            fn probe(&self, _path: &std::path::Path) -> MediumKind {
                MediumKind::Unknown
            }
        }
        Box::new(FallbackProbe)
    }
}

/// Buat iterator direktori default untuk platform saat ini.
pub fn default_dir_iterator() -> Box<dyn DirIterator> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsDirIterator::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxDirIterator::new())
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        struct FallbackIterator;
        impl DirIterator for FallbackIterator {
            fn walk(
                &mut self,
                _root: &std::path::Path,
                token: &CancellationToken,
                _emit: &mut dyn FnMut(RawEntry<'_>),
            ) -> Result<(), ScanError> {
                token.check()?;
                Ok(())
            }
        }
        Box::new(FallbackIterator)
    }
}

/// Buat FsOps default untuk platform saat ini.
pub fn default_fs_ops() -> Box<dyn FsOps> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsFsOps::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxFsOps::new())
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        struct FallbackFsOps;
        impl FsOps for FallbackFsOps {
            fn delete_permanent(&self, _path: &VerbatimPath) -> Result<(), FsError> {
                Ok(())
            }
            fn move_to_trash(&self, _path: &VerbatimPath) -> Result<TrashOutcome, FsError> {
                Ok(TrashOutcome::Success)
            }
            fn path_is_cross_fs(&self, _path: &std::path::Path, _trash_root: &std::path::Path) -> bool {
                false
            }
            fn open_default(&self, _path: &std::path::Path) -> Result<(), FsError> {
                Ok(())
            }
            fn reveal_in_file_manager(&self, _path: &std::path::Path) -> Result<(), FsError> {
                Ok(())
            }
        }
        Box::new(FallbackFsOps)
    }
}

