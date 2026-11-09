use crate::entri::{penanda, RekamanEntri};
use crate::error::ErrorLog;
use crate::gudang::{GudangEntri, GudangNama};
use crate::ids::{IdEntri, IdNama, INDUK_AKAR};
use fast_disk_analyze_platform::IdentitasBerkas;
use rustc_hash::FxHashSet;

/// Hasil kerja traversal dari satu worker thread
pub struct WorkerScanResult {
    pub daftar_entri: Vec<RekamanEntri>,
    pub nama: GudangNama,
    pub tautan: Vec<(usize, IdentitasBerkas)>,
    pub items_scanned: u64,
    pub bytes_scanned: u64,
    pub errors: Vec<(u32, fast_disk_analyze_platform::ScanError)>,
}

pub struct Perakit;

impl Perakit {
    /// Merakit hasil dari banyak worker thread menjadi GudangEntri tunggal dengan:
    /// 1. Merge array entri & remap IdNama
    /// 2. Resolusi parent_token ke final_id
    /// 3. Dedup hardlink efemeral
    /// 4. Pengurutan kontigu anak berdasarkan induk (anak_awal & jumlah_anak)
    /// 5. Agregasi ukuran direktori iteratif bottom-up (zero recursion)
    pub fn rakit_multi_worker(
        results: Vec<WorkerScanResult>,
        total_tokens: u32,
    ) -> (GudangEntri, ErrorLog) {
        let mut central_nama = GudangNama::baru();
        let total_est_entries: usize = results.iter().map(|r| r.daftar_entri.len()).sum();
        let mut merged_entries: Vec<RekamanEntri> = Vec::with_capacity(total_est_entries);
        let mut all_tautan: Vec<(usize, IdentitasBerkas)> = Vec::new();
        let mut error_log = ErrorLog::new();

        // 1. Merge array & remap nama
        for worker_res in results {
            let offset = merged_entries.len();

            // Bangun tabel translasi IdNama untuk worker ini
            let worker_nama_count = worker_res.nama.jumlah();
            let mut name_map = Vec::with_capacity(worker_nama_count);
            for i in 0..worker_nama_count {
                let name_bytes = worker_res.nama.ambil_nama(IdNama(i as u32));
                let central_id = central_nama.simpan_nama(name_bytes);
                name_map.push(central_id);
            }

            for mut entri in worker_res.daftar_entri {
                entri.id_nama = name_map[entri.id_nama as usize].0;
                merged_entries.push(entri);
            }

            for (local_idx, id_berkas) in worker_res.tautan {
                all_tautan.push((offset + local_idx, id_berkas));
            }

            for (token_id, err) in worker_res.errors {
                error_log.catat(IdEntri(token_id), err);
            }
        }

        if merged_entries.is_empty() {
            return (
                GudangEntri {
                    daftar_entri: merged_entries,
                    nama: central_nama,
                },
                error_log,
            );
        }

        let total = merged_entries.len();

        // 2. Resolusi parent_token -> final_id
        let mut token_to_final_id: Vec<u32> = vec![INDUK_AKAR; (total_tokens + 1) as usize];
        for (idx, entri) in merged_entries.iter().enumerate() {
            if entri.adalah_direktori() {
                let my_token = entri.anak_awal;
                if (my_token as usize) < token_to_final_id.len() {
                    token_to_final_id[my_token as usize] = idx as u32;
                }
            }
        }

        for entri in merged_entries.iter_mut() {
            let parent_token = entri.induk;
            if parent_token == INDUK_AKAR {
                entri.induk = INDUK_AKAR;
            } else if (parent_token as usize) < token_to_final_id.len() {
                entri.induk = token_to_final_id[parent_token as usize];
            }
            entri.anak_awal = 0;
        }

        // 3. Dedup hardlink efemeral
        if !all_tautan.is_empty() {
            let mut visited = FxHashSet::default();
            for (idx, id_berkas) in all_tautan {
                if idx < total {
                    if !visited.insert(id_berkas) {
                        // Tautan ganda: tandai flag, nol-kan ukuran agar tidak diagregasi dobel
                        merged_entries[idx].penanda |= penanda::TAUTAN_GANDA;
                        merged_entries[idx].ukuran_asli = 0;
                        merged_entries[idx].ukuran_blok = 0;
                    }
                }
            }
            // visited di-drop di sini untuk membebaskan RAM
        }

        // 4. Pengurutan kontigu berdasarkan induk (Stable Sort)
        // Root (induk == INDUK_AKAR) diprioritaskan di indeks 0
        let mut perm: Vec<u32> = (0..total as u32).collect();
        perm.sort_by_key(|&idx| {
            let p = merged_entries[idx as usize].induk;
            if p == INDUK_AKAR {
                0u64
            } else {
                (p as u64) + 1
            }
        });

        let mut new_pos = vec![0u32; total];
        for (new_idx, &old_idx) in perm.iter().enumerate() {
            new_pos[old_idx as usize] = new_idx as u32;
        }

        let mut sorted_entries = Vec::with_capacity(total);
        for &old_idx in &perm {
            let mut entri = merged_entries[old_idx as usize];
            if entri.induk != INDUK_AKAR && (entri.induk as usize) < total {
                entri.induk = new_pos[entri.induk as usize];
            }
            sorted_entries.push(entri);
        }

        // Hitung anak_awal dan jumlah_anak untuk setiap induk
        let mut count = vec![0u32; total];
        let mut start = vec![0u32; total];
        for (idx, entri) in sorted_entries.iter().enumerate() {
            let p = entri.induk;
            if p != INDUK_AKAR && (p as usize) < total {
                let pu = p as usize;
                if count[pu] == 0 {
                    start[pu] = idx as u32;
                }
                count[pu] += 1;
            }
        }

        for i in 0..total {
            sorted_entries[i].anak_awal = start[i];
            sorted_entries[i].jumlah_anak = count[i];
        }

        // 5. Agregasi iteratif bottom-up tanpa rekursi (urutan terbalik indeks)
        for i in (0..total).rev() {
            let parent = sorted_entries[i].induk;
            if parent != INDUK_AKAR && (parent as usize) < total {
                let u_asli = sorted_entries[i].ukuran_asli;
                let u_blok = sorted_entries[i].ukuran_blok;
                sorted_entries[parent as usize].ukuran_asli += u_asli;
                sorted_entries[parent as usize].ukuran_blok += u_blok;
            }
        }

        error_log.selesaikan_urut();

        (
            GudangEntri {
                daftar_entri: sorted_entries,
                nama: central_nama,
            },
            error_log,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fast_disk_analyze_platform::JenisEntri;

    #[test]
    fn test_perakit_multi_worker_dan_agregasi() {
        // Buat hasil tiruan dari 2 worker
        // Root: token 0 (Folder utama)
        // Worker 1 memindai:
        // - Root (token 0, parent INDUK_AKAR)
        // - Folder A (token 1, parent token 0)
        // - Berkas A1 (parent token 1, ukuran 100)
        let mut g1 = GudangEntri::baru();
        let name_root = g1.nama.simpan_nama(b"root");
        let name_foldera = g1.nama.simpan_nama(b"folder_a");
        let name_filea1 = g1.nama.simpan_nama(b"file_a1.bin");

        g1.tambah_entri(RekamanEntri {
            ukuran_asli: 0,
            ukuran_blok: 0,
            induk: INDUK_AKAR,
            id_nama: name_root.0,
            anak_awal: 0, // my_token = 0
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Direktori as u8,
            penanda: 0,
            _padding: 0,
        });

        g1.tambah_entri(RekamanEntri {
            ukuran_asli: 0,
            ukuran_blok: 0,
            induk: 0, // parent_token = 0
            id_nama: name_foldera.0,
            anak_awal: 1, // my_token = 1
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Direktori as u8,
            penanda: 0,
            _padding: 0,
        });

        g1.tambah_entri(RekamanEntri {
            ukuran_asli: 100,
            ukuran_blok: 128,
            induk: 1, // parent_token = 1
            id_nama: name_filea1.0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Berkas as u8,
            penanda: 0,
            _padding: 0,
        });

        // Worker 2 memindai:
        // - Berkas R1 di Root (parent token 0, ukuran 50)
        // - Berkas A2 di Folder A (parent token 1, ukuran 200)
        let mut g2 = GudangEntri::baru();
        let name_filer1 = g2.nama.simpan_nama(b"file_r1.bin");
        let name_filea2 = g2.nama.simpan_nama(b"file_a2.bin");

        g2.tambah_entri(RekamanEntri {
            ukuran_asli: 50,
            ukuran_blok: 64,
            induk: 0, // parent_token = 0
            id_nama: name_filer1.0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Berkas as u8,
            penanda: 0,
            _padding: 0,
        });

        g2.tambah_entri(RekamanEntri {
            ukuran_asli: 200,
            ukuran_blok: 256,
            induk: 1, // parent_token = 1
            id_nama: name_filea2.0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Berkas as u8,
            penanda: 0,
            _padding: 0,
        });

        let results = vec![
            WorkerScanResult {
                daftar_entri: g1.daftar_entri,
                nama: g1.nama,
                tautan: Vec::new(),
                items_scanned: 3,
                bytes_scanned: 100,
                errors: Vec::new(),
            },
            WorkerScanResult {
                daftar_entri: g2.daftar_entri,
                nama: g2.nama,
                tautan: Vec::new(),
                items_scanned: 2,
                bytes_scanned: 250,
                errors: Vec::new(),
            },
        ];

        let (gudang, _) = Perakit::rakit_multi_worker(results, 1);
        assert_eq!(gudang.jumlah(), 5);

        // Root ada di index 0
        let root = &gudang.daftar_entri[0];
        assert_eq!(root.induk, INDUK_AKAR);
        assert_eq!(root.ukuran_asli, 350); // 100 + 200 + 50
        assert_eq!(root.ukuran_blok, 448); // 128 + 256 + 64
        assert_eq!(root.jumlah_anak, 2);   // folder_a dan file_r1

        // Anak root kontigu di root.anak()
        let root_children = &gudang.daftar_entri[root.anak()];
        assert_eq!(root_children.len(), 2);
    }
}
