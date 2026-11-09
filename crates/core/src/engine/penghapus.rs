use crate::ids::IdEntri;
use crate::model::PohonSistemBerkas;
use fast_disk_analyze_platform::{FsError, FsOps};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ModeHapus {
    Permanen,
    Sampah,
}

#[derive(Clone, Debug, Default)]
pub struct HasilPenghapusan {
    pub terhapus: Vec<IdEntri>,
    pub gagal: Vec<(IdEntri, FsError)>,
    pub dibebaskan_asli: u64,
    pub dibebaskan_blok: u64,
}

pub struct Penghapus;

impl Penghapus {
    pub fn new() -> Self {
        Self
    }

    /// Mengeksekusi operasi penghapusan pada berkas/folder melalui FsOps
    pub fn eksekusi(
        pohon: &PohonSistemBerkas,
        targets: &[IdEntri],
        mode: ModeHapus,
        ops: &dyn FsOps,
    ) -> HasilPenghapusan {
        let mut terhapus = Vec::with_capacity(targets.len());
        let mut gagal = Vec::new();
        let mut dibebaskan_asli = 0u64;
        let mut dibebaskan_blok = 0u64;

        for &id in targets {
            let verbatim = pohon.fs_path_verbatim(id);
            let res = match mode {
                ModeHapus::Permanen => ops.delete_permanent(&verbatim),
                ModeHapus::Sampah => ops.move_to_trash(&verbatim).map(|_| ()),
            };

            match res {
                Ok(()) => {
                    terhapus.push(id);
                    if (id.0 as usize) < pohon.gudang.jumlah() {
                        let entri = pohon.gudang.ambil(id);
                        dibebaskan_asli += entri.ukuran_asli;

                        // Nlink-aware: kurangi ukuran blok jika tautan tunggal atau link representatif
                        if entri.jumlah_tautan <= 1 || (entri.penanda & crate::entri::penanda::TAUTAN_GANDA == 0) {
                            dibebaskan_blok += entri.ukuran_blok;
                        }
                    }
                }
                Err(err) => {
                    gagal.push((id, err));
                }
            }
        }

        HasilPenghapusan {
            terhapus,
            gagal,
            dibebaskan_asli,
            dibebaskan_blok,
        }
    }
}

impl Default for Penghapus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entri::RekamanEntri;
    use crate::error::ErrorLog;
    use crate::gudang::GudangEntri;
    use crate::ids::INDUK_AKAR;
    use fast_disk_analyze_platform::{TrashOutcome, VerbatimPath};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockFsOps {
        pub fail_on_delete: AtomicBool,
    }

    impl FsOps for MockFsOps {
        fn delete_permanent(&self, _path: &VerbatimPath) -> Result<(), FsError> {
            if self.fail_on_delete.load(Ordering::SeqCst) {
                Err(FsError::PermissionDenied)
            } else {
                Ok(())
            }
        }
        fn move_to_trash(&self, _path: &VerbatimPath) -> Result<TrashOutcome, FsError> {
            if self.fail_on_delete.load(Ordering::SeqCst) {
                Err(FsError::PermissionDenied)
            } else {
                Ok(TrashOutcome::Success)
            }
        }
        fn path_is_cross_fs(&self, _path: &Path, _trash_root: &Path) -> bool {
            false
        }
        fn open_default(&self, _path: &Path) -> Result<(), FsError> {
            Ok(())
        }
        fn reveal_in_file_manager(&self, _path: &Path) -> Result<(), FsError> {
            Ok(())
        }
    }

    #[test]
    fn test_penghapus_berhasil_dan_gagal() {
        let mut gudang = GudangEntri::baru();
        let name_root = gudang.nama.simpan_nama(b"/");
        let name_file = gudang.nama.simpan_nama(b"test.txt");

        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 1000,
            ukuran_blok: 1024,
            induk: INDUK_AKAR,
            id_nama: name_root.0,
            anak_awal: 1,
            jumlah_anak: 1,
            jumlah_tautan: 1,
            jenis: 1,
            penanda: 0,
            _padding: 0,
        });

        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 1000,
            ukuran_blok: 1024,
            induk: 0,
            id_nama: name_file.0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: 0,
            penanda: 0,
            _padding: 0,
        });

        let pohon = PohonSistemBerkas::baru(PathBuf::from("/mock"), gudang, IdEntri(0), ErrorLog::new());
        let ops = MockFsOps {
            fail_on_delete: AtomicBool::new(false),
        };

        // Test sukses hapus
        let hasil = Penghapus::eksekusi(&pohon, &[IdEntri(1)], ModeHapus::Permanen, &ops);
        assert_eq!(hasil.terhapus.len(), 1);
        assert_eq!(hasil.gagal.len(), 0);
        assert_eq!(hasil.dibebaskan_asli, 1000);
        assert_eq!(hasil.dibebaskan_blok, 1024);

        // Test gagal hapus (permission denied)
        ops.fail_on_delete.store(true, Ordering::SeqCst);
        let hasil_gagal = Penghapus::eksekusi(&pohon, &[IdEntri(1)], ModeHapus::Permanen, &ops);
        assert_eq!(hasil_gagal.terhapus.len(), 0);
        assert_eq!(hasil_gagal.gagal.len(), 1);
        assert_eq!(hasil_gagal.gagal[0].1, FsError::PermissionDenied);
    }
}
