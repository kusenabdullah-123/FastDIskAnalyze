use crate::entri::{penanda, RekamanEntri};
use crate::gudang::{GudangEntri, GudangNama};
use crate::ids::INDUK_AKAR;
use fast_disk_analyze_platform::IdentitasBerkas;
use rustc_hash::FxHashSet;

pub struct Perakit;

impl Perakit {
    /// Merakit daftar entri mentah hasil traversal worker menjadi gudang kontigu
    /// dengan agregasi ukuran direktori iteratif dan dedup hardlink.
    pub fn rakit(
        mut daftar_entri: Vec<RekamanEntri>,
        nama: GudangNama,
        tautan_list: Vec<(usize, IdentitasBerkas)>, // (indeks di daftar_entri, identitas)
    ) -> GudangEntri {
        if daftar_entri.is_empty() {
            return GudangEntri {
                daftar_entri,
                nama,
            };
        }

        // 1. Dedup hardlink: tandai kemunculan kedua dan seterusnya
        if !tautan_list.is_empty() {
            let mut visited = FxHashSet::default();
            for (idx, id_berkas) in tautan_list {
                if idx < daftar_entri.len() {
                    if !visited.insert(id_berkas) {
                        // Tautan ganda: tandai flag, ukuran di-nol-kan agar tidak diagregasi dobel
                        daftar_entri[idx].penanda |= penanda::TAUTAN_GANDA;
                        daftar_entri[idx].ukuran_asli = 0;
                        daftar_entri[idx].ukuran_blok = 0;
                    }
                }
            }
        }

        // 2. Hubungkan anak_awal dan jumlah_anak:
        // Jika entri sudah diurutkan atau disusun menurut induk, kita isi range anak
        let total = daftar_entri.len();
        let mut count_map: Vec<u32> = vec![0; total];

        for i in 0..total {
            let parent = daftar_entri[i].induk;
            if parent != INDUK_AKAR && (parent as usize) < total {
                count_map[parent as usize] += 1;
            }
        }

        for i in 0..total {
            daftar_entri[i].jumlah_anak = count_map[i];
        }

        // 3. Agregasi iteratif bottom-up tanpa rekursi (urutan terbalik indeks)
        for i in (0..total).rev() {
            let parent = daftar_entri[i].induk;
            if parent != INDUK_AKAR && (parent as usize) < total {
                let u_asli = daftar_entri[i].ukuran_asli;
                let u_blok = daftar_entri[i].ukuran_blok;
                daftar_entri[parent as usize].ukuran_asli += u_asli;
                daftar_entri[parent as usize].ukuran_blok += u_blok;
            }
        }

        GudangEntri {
            daftar_entri,
            nama,
        }
    }
}
