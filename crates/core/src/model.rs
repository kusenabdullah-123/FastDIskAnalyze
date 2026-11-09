use crate::entri::RekamanEntri;
use crate::error::ErrorLog;
use crate::gudang::GudangEntri;
use crate::ids::{IdEntri, IdNama, INDUK_AKAR};
use crate::view::{SizeMode, SortKey, ViewCache};
use fast_disk_analyze_platform::{ScanError, VerbatimPath};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

pub struct DisplayPath(pub String);

impl std::fmt::Display for DisplayPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct PohonSistemBerkas {
    pub root_path: PathBuf,
    pub gudang: GudangEntri,
    pub root: IdEntri,
    pub errors: ErrorLog,
    view_cache: std::sync::RwLock<ViewCache>,
}

impl PohonSistemBerkas {
    pub fn baru(root_path: PathBuf, gudang: GudangEntri, root: IdEntri, errors: ErrorLog) -> Self {
        Self {
            root_path,
            gudang,
            root,
            errors,
            view_cache: std::sync::RwLock::new(ViewCache::baru()),
        }
    }

    /// Mendapatkan slice anak-anak dari entri yang kontigu di memori, O(1).
    pub fn children(&self, id: IdEntri) -> &[RekamanEntri] {
        let entri = self.gudang.ambil(id);
        let jangkauan = entri.anak();
        if jangkauan.end <= self.gudang.jumlah() {
            &self.gudang.daftar_entri[jangkauan]
        } else {
            &[]
        }
    }

    /// Mendapatkan slice IdEntri untuk anak-anak direktori, O(K).
    pub fn children_ids(&self, id: IdEntri) -> Vec<IdEntri> {
        let entri = self.gudang.ambil(id);
        (entri.anak_awal..entri.anak_awal + entri.jumlah_anak)
            .map(IdEntri)
            .filter(|&child_id| !self.gudang.ambil(child_id).adalah_yatim())
            .collect()
    }

    pub fn adalah_direktori(&self, id: IdEntri) -> bool {
        if (id.0 as usize) < self.gudang.jumlah() {
            self.gudang.ambil(id).adalah_direktori()
        } else {
            false
        }
    }

    pub fn jumlah_anak(&self, id: IdEntri) -> u32 {
        if (id.0 as usize) < self.gudang.jumlah() {
            self.children_ids(id).len() as u32
        } else {
            0
        }
    }

    /// Mengambil daftar anak yang diurutkan dengan dukungan cache O(1) pada cache-hit.
    pub fn sorted(&self, id: IdEntri, key: SortKey) -> Arc<[IdEntri]> {
        if let Ok(cache) = self.view_cache.read() {
            if let Some(cached) = cache.sort_cache.get(&(id, key)) {
                return cached.clone();
            }
        }

        let mut ids = self.children_ids(id);
        match key {
            SortKey::UkuranMenurun => {
                ids.sort_by(|&a, &b| {
                    let ea = self.gudang.ambil(a);
                    let eb = self.gudang.ambil(b);
                    eb.ukuran_blok.cmp(&ea.ukuran_blok)
                });
            }
            SortKey::UkuranMenaik => {
                ids.sort_by(|&a, &b| {
                    let ea = self.gudang.ambil(a);
                    let eb = self.gudang.ambil(b);
                    ea.ukuran_blok.cmp(&eb.ukuran_blok)
                });
            }
            SortKey::NamaAZ => {
                ids.sort_by(|&a, &b| {
                    let na = self.gudang.nama.ambil_nama(IdNama(self.gudang.ambil(a).id_nama));
                    let nb = self.gudang.nama.ambil_nama(IdNama(self.gudang.ambil(b).id_nama));
                    na.cmp(nb)
                });
            }
            SortKey::NamaZA => {
                ids.sort_by(|&a, &b| {
                    let na = self.gudang.nama.ambil_nama(IdNama(self.gudang.ambil(a).id_nama));
                    let nb = self.gudang.nama.ambil_nama(IdNama(self.gudang.ambil(b).id_nama));
                    nb.cmp(na)
                });
            }
            SortKey::JumlahAnakMenurun => {
                ids.sort_by(|&a, &b| {
                    let ea = self.gudang.ambil(a);
                    let eb = self.gudang.ambil(b);
                    eb.jumlah_anak.cmp(&ea.jumlah_anak)
                });
            }
            SortKey::JumlahAnakMenaik => {
                ids.sort_by(|&a, &b| {
                    let ea = self.gudang.ambil(a);
                    let eb = self.gudang.ambil(b);
                    ea.jumlah_anak.cmp(&eb.jumlah_anak)
                });
            }
        }

        let arc: Arc<[IdEntri]> = ids.into();
        if let Ok(mut cache) = self.view_cache.write() {
            cache.simpan_sort(id, key, arc.clone());
        }
        arc
    }

    /// Pencarian substring case-insensitive langsung pada buffer byte mentah (zero heap allocation per berkas).
    pub fn filtered(&self, id: IdEntri, q: &str) -> Arc<[IdEntri]> {
        let cache_key = (id, q.to_string());
        if let Ok(mut cache) = self.view_cache.write() {
            if let Some(cached) = cache.filter_cache.get(&cache_key) {
                return cached.clone();
            }
        }

        let q_bytes = q.as_bytes();
        let ids = self.children_ids(id);

        let hasil: Vec<IdEntri> = ids
            .into_iter()
            .filter(|&child_id| {
                if q_bytes.is_empty() {
                    return true;
                }
                let nama_bytes = self.gudang.nama.ambil_nama(IdNama(self.gudang.ambil(child_id).id_nama));
                if nama_bytes.len() < q_bytes.len() {
                    return false;
                }
                // Case-insensitive byte matching tanpa alokasi String
                nama_bytes.windows(q_bytes.len()).any(|w| {
                    w.iter().zip(q_bytes.iter()).all(|(&b, &q)| {
                        b.to_ascii_lowercase() == q.to_ascii_lowercase()
                    })
                })
            })
            .collect();

        let arc: Arc<[IdEntri]> = hasil.into();
        if let Ok(mut cache) = self.view_cache.write() {
            cache.filter_cache.put(cache_key, arc.clone());
        }
        arc
    }

    /// Rantai leluhur dari Root hingga node target (untuk breadcrumb navigation di GUI).
    pub fn ancestor_chain(&self, id: IdEntri) -> Vec<IdEntri> {
        let mut chain = Vec::new();
        let mut curr = id.0;
        while curr != INDUK_AKAR && (curr as usize) < self.gudang.jumlah() {
            chain.push(IdEntri(curr));
            curr = self.gudang.ambil(IdEntri(curr)).induk;
        }
        chain.reverse();
        chain
    }

    /// Kedalaman direktori dari Root (Root = 0).
    pub fn depth(&self, id: IdEntri) -> usize {
        let mut d = 0;
        let mut curr = self.gudang.ambil(id).induk;
        while curr != INDUK_AKAR && (curr as usize) < self.gudang.jumlah() {
            d += 1;
            curr = self.gudang.ambil(IdEntri(curr)).induk;
        }
        d
    }

    /// Menghitung persentase ukuran disk anak terhadap induknya dalam O(1).
    pub fn size_percentage(&self, child_id: IdEntri) -> f64 {
        let child = self.gudang.ambil(child_id);
        if child.induk == INDUK_AKAR || (child.induk as usize) >= self.gudang.jumlah() {
            return 100.0;
        }
        let parent = self.gudang.ambil(IdEntri(child.induk));
        if parent.ukuran_blok > 0 {
            (child.ukuran_blok as f64 / parent.ukuran_blok as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn nama(&self, id: IdEntri) -> Cow<'_, str> {
        let entri = self.gudang.ambil(id);
        self.gudang.nama.nama_tampil(IdNama(entri.id_nama))
    }

    pub fn nama_raw(&self, id: IdEntri) -> &[u8] {
        let entri = self.gudang.ambil(id);
        self.gudang.nama.ambil_nama(IdNama(entri.id_nama))
    }

    pub fn display_path(&self, id: IdEntri) -> DisplayPath {
        let mut segments = Vec::new();
        let mut curr = id.0;

        while curr != INDUK_AKAR && (curr as usize) < self.gudang.jumlah() {
            let entri = self.gudang.ambil(IdEntri(curr));
            segments.push(self.gudang.nama.nama_tampil(IdNama(entri.id_nama)));
            curr = entri.induk;
        }

        segments.reverse();
        if segments.is_empty() {
            DisplayPath(String::from("/"))
        } else {
            DisplayPath(segments.join("/"))
        }
    }

    pub fn fs_path_verbatim(&self, id: IdEntri) -> VerbatimPath {
        if id == self.root {
            return VerbatimPath(self.root_path.clone());
        }

        let mut parts = Vec::new();
        let mut curr = id.0;

        while curr != INDUK_AKAR && (curr as usize) < self.gudang.jumlah() {
            if IdEntri(curr) == self.root {
                break;
            }
            let entri = self.gudang.ambil(IdEntri(curr));
            parts.push(self.gudang.nama.ambil_nama(IdNama(entri.id_nama)));
            curr = entri.induk;
        }

        parts.reverse();
        let mut path_buf = self.root_path.clone();
        for part in parts {
            #[cfg(unix)]
            {
                use std::ffi::OsStr;
                use std::os::unix::ffi::OsStrExt;
                path_buf.push(OsStr::from_bytes(part));
            }
            #[cfg(not(unix))]
            {
                let s = String::from_utf8_lossy(part);
                path_buf.push(s.as_ref());
            }
        }

        VerbatimPath(path_buf)
    }

    pub fn effective_size(&self, id: IdEntri, mode: SizeMode) -> u64 {
        let entri = self.gudang.ambil(id);
        match mode {
            SizeMode::UkuranAsli => entri.ukuran_asli,
            SizeMode::UkuranBlok => entri.ukuran_blok,
        }
    }

    pub fn errors_for(&self, id: IdEntri) -> Vec<&ScanError> {
        self.errors.untuk_entri(id)
    }

    pub fn invalidasi_cache(&self) {
        if let Ok(mut cache) = self.view_cache.write() {
            cache.invalidasi_semua();
        }
    }

    pub fn invalidasi_rantai_leluhur(&self, id: IdEntri) {
        let chain = self.ancestor_chain(id);
        if let Ok(mut cache) = self.view_cache.write() {
            cache.invalidasi_banyak(&chain);
        }
    }

    /// Menghitung rasio entri yatim terhadap total entri di dalam arena gudang.
    pub fn rasio_yatim(&self) -> f32 {
        let total = self.gudang.jumlah();
        if total == 0 {
            return 0.0;
        }
        let yatim_count = self.gudang.daftar_entri.iter().filter(|e| e.adalah_yatim()).count();
        yatim_count as f32 / total as f32
    }

    /// Menerapkan hasil operasi penghapusan berkas/folder ke domain model:
    /// Menandai node dan seluruh keturunannya sebagai YATIM, mengurangi ukuran agregat
    /// pada rantai leluhur secara O(D), dan mencatat kegagalan ke ErrorLog.
    pub fn terapkan_penghapusan(&mut self, hasil: &crate::engine::penghapus::HasilPenghapusan) {
        for &id in &hasil.terhapus {
            if (id.0 as usize) >= self.gudang.jumlah() {
                continue;
            }

            let (delta_asli, delta_blok, curr_induk) = {
                let entri = self.gudang.ambil_mut(id);
                if entri.adalah_yatim() {
                    continue;
                }
                let delta_asli = entri.ukuran_asli;
                let delta_blok = if entri.jumlah_tautan <= 1 || (entri.penanda & crate::entri::penanda::TAUTAN_GANDA == 0) {
                    entri.ukuran_blok
                } else {
                    0
                };
                entri.penanda |= crate::entri::penanda::YATIM;
                (delta_asli, delta_blok, entri.induk)
            };

            // Tandai semua keturunan (jika direktori) sebagai yatim juga
            let mut stack = vec![id];
            while let Some(parent_id) = stack.pop() {
                let parent_entry = *self.gudang.ambil(parent_id);
                for child_idx in parent_entry.anak_awal..parent_entry.anak_awal + parent_entry.jumlah_anak {
                    if (child_idx as usize) < self.gudang.jumlah() {
                        let child = self.gudang.ambil_mut(IdEntri(child_idx));
                        if !child.adalah_yatim() {
                            child.penanda |= crate::entri::penanda::YATIM;
                            if child.adalah_direktori() {
                                stack.push(IdEntri(child_idx));
                            }
                        }
                    }
                }
            }

            // Propagasi pengurangan ukuran ke atas (rantai leluhur) O(D)
            let mut p = curr_induk;
            while p != INDUK_AKAR && (p as usize) < self.gudang.jumlah() {
                let ancestor = self.gudang.ambil_mut(IdEntri(p));
                ancestor.ukuran_asli = ancestor.ukuran_asli.saturating_sub(delta_asli);
                ancestor.ukuran_blok = ancestor.ukuran_blok.saturating_sub(delta_blok);
                p = ancestor.induk;
            }
        }

        // Catat error kegagalan penghapusan
        for (id, err) in &hasil.gagal {
            let p = self.fs_path_verbatim(*id);
            self.errors.catat(*id, err.to_scan_error(p.0));
        }

        self.invalidasi_cache();

        if self.rasio_yatim() > 0.25 && self.gudang.jumlah() > 1000 {
            self.kompaksi();
        }
    }

    /// Menerapkan pemindaian ulang subtree ke dalam domain model secara append-only:
    /// - Menandai anak lama target_id sebagai YATIM
    /// - Meng-append anak baru dari subtree_gudang ke akhir self.gudang
    /// - Menghitung delta ukuran dan mempropagasi ke seluruh rantai leluhur target_id O(D)
    pub fn terapkan_subtree(&mut self, target_id: IdEntri, subtree_gudang: GudangEntri, subtree_root: IdEntri) {
        if (target_id.0 as usize) >= self.gudang.jumlah() {
            return;
        }

        // 1. Tandai anak-anak dan keturunan lama dari target_id sebagai YATIM
        let mut stack = vec![target_id];
        while let Some(parent_id) = stack.pop() {
            let parent_entry = *self.gudang.ambil(parent_id);
            for child_idx in parent_entry.anak_awal..parent_entry.anak_awal + parent_entry.jumlah_anak {
                if (child_idx as usize) < self.gudang.jumlah() {
                    let child = self.gudang.ambil_mut(IdEntri(child_idx));
                    if !child.adalah_yatim() {
                        child.penanda |= crate::entri::penanda::YATIM;
                        if child.adalah_direktori() {
                            stack.push(IdEntri(child_idx));
                        }
                    }
                }
            }
        }

        let old_asli = self.gudang.ambil(target_id).ukuran_asli;
        let old_blok = self.gudang.ambil(target_id).ukuran_blok;

        let new_root_entry = *subtree_gudang.ambil(subtree_root);
        let new_asli = new_root_entry.ukuran_asli;
        let new_blok = new_root_entry.ukuran_blok;

        // 2. Map IdNama dari subtree_gudang ke self.gudang.nama
        let mut nama_map = Vec::with_capacity(subtree_gudang.nama.jumlah());
        for i in 0..subtree_gudang.nama.jumlah() {
            let bytes = subtree_gudang.nama.ambil_nama(IdNama(i as u32));
            let new_id_nama = self.gudang.nama.simpan_nama(bytes);
            nama_map.push(new_id_nama.0);
        }

        // 3. Append entri dari subtree_gudang (kecuali root itu sendiri, karena target_id sudah ada)
        let base_idx = self.gudang.jumlah() as u32;
        let num_subtree_entries = subtree_gudang.jumlah();

        let mut id_map = vec![0u32; num_subtree_entries];
        id_map[subtree_root.0 as usize] = target_id.0;

        let mut offset = 0u32;
        for i in 0..num_subtree_entries {
            if i == subtree_root.0 as usize {
                continue;
            }
            id_map[i] = base_idx + offset;
            offset += 1;
        }

        for i in 0..num_subtree_entries {
            if i == subtree_root.0 as usize {
                continue;
            }
            let mut e = *subtree_gudang.ambil(IdEntri(i as u32));
            if (e.id_nama as usize) < nama_map.len() {
                e.id_nama = nama_map[e.id_nama as usize];
            }
            if (e.induk as usize) < id_map.len() {
                e.induk = id_map[e.induk as usize];
            }
            if e.jumlah_anak > 0 && (e.anak_awal as usize) < id_map.len() {
                e.anak_awal = id_map[e.anak_awal as usize];
            }
            self.gudang.tambah_entri(e);
        }

        // 4. Update target_id dengan anak baru dan ukuran baru
        {
            let target_entry = self.gudang.ambil_mut(target_id);
            target_entry.ukuran_asli = new_asli;
            target_entry.ukuran_blok = new_blok;
            if new_root_entry.jumlah_anak > 0 && (new_root_entry.anak_awal as usize) < id_map.len() {
                target_entry.anak_awal = id_map[new_root_entry.anak_awal as usize];
                target_entry.jumlah_anak = new_root_entry.jumlah_anak;
            } else {
                target_entry.anak_awal = 0;
                target_entry.jumlah_anak = 0;
            }
        }

        // 5. Propagasi delta ke seluruh rantai leluhur
        let delta_asli = new_asli as i64 - old_asli as i64;
        let delta_blok = new_blok as i64 - old_blok as i64;
        let mut p = self.gudang.ambil(target_id).induk;
        while p != INDUK_AKAR && (p as usize) < self.gudang.jumlah() {
            let ancestor = self.gudang.ambil_mut(IdEntri(p));
            ancestor.ukuran_asli = (ancestor.ukuran_asli as i64 + delta_asli).max(0) as u64;
            ancestor.ukuran_blok = (ancestor.ukuran_blok as i64 + delta_blok).max(0) as u64;
            p = ancestor.induk;
        }

        self.invalidasi_cache();

        if self.rasio_yatim() > 0.25 && self.gudang.jumlah() > 1000 {
            self.kompaksi();
        }
    }

    /// Kompaksi linier: membangun kembali arena untuk membuang semua entri YATIM
    /// dan memulihkan kontiguitas memori ketika rasio yatim > 25%.
    pub fn kompaksi(&mut self) {
        let total = self.gudang.jumlah();
        if total == 0 {
            return;
        }

        let mut queue = std::collections::VecDeque::new();
        if !self.gudang.ambil(self.root).adalah_yatim() {
            queue.push_back(self.root);
        }

        let mut old_to_new = vec![u32::MAX; total];
        let mut ordered_old_ids = Vec::with_capacity(total);

        // 1. Kumpulkan semua node aktif level demi level (BFS) agar anak selalu kontigu
        while let Some(curr) = queue.pop_front() {
            let new_id = ordered_old_ids.len() as u32;
            old_to_new[curr.0 as usize] = new_id;
            ordered_old_ids.push(curr);

            let entri = self.gudang.ambil(curr);
            for child_idx in entri.anak_awal..entri.anak_awal + entri.jumlah_anak {
                if (child_idx as usize) < total {
                    let child = self.gudang.ambil(IdEntri(child_idx));
                    if !child.adalah_yatim() {
                        queue.push_back(IdEntri(child_idx));
                    }
                }
            }
        }

        // 2. Susun entri baru dengan remapping nama, induk, dan anak_awal
        let mut new_nama = crate::gudang::GudangNama::baru();
        let mut nama_map = vec![u32::MAX; self.gudang.nama.jumlah()];
        let mut new_entries: Vec<RekamanEntri> = Vec::with_capacity(ordered_old_ids.len());

        for &old_id in &ordered_old_ids {
            let mut e = *self.gudang.ambil(old_id);

            // Remap nama
            let old_id_nama = e.id_nama as usize;
            if old_id_nama < nama_map.len() {
                if nama_map[old_id_nama] == u32::MAX {
                    let raw = self.gudang.nama.ambil_nama(IdNama(e.id_nama));
                    nama_map[old_id_nama] = new_nama.simpan_nama(raw).0;
                }
                e.id_nama = nama_map[old_id_nama];
            }

            // Remap induk
            if e.induk != INDUK_AKAR && (e.induk as usize) < old_to_new.len() {
                e.induk = old_to_new[e.induk as usize];
            }

            // Remap anak: cari anak aktif pertama
            let mut active_children_count = 0u32;
            let mut first_new_child_idx = 0u32;
            for child_idx in e.anak_awal..e.anak_awal + e.jumlah_anak {
                if (child_idx as usize) < total && !self.gudang.ambil(IdEntri(child_idx)).adalah_yatim() {
                    if active_children_count == 0 {
                        first_new_child_idx = old_to_new[child_idx as usize];
                    }
                    active_children_count += 1;
                }
            }
            e.anak_awal = first_new_child_idx;
            e.jumlah_anak = active_children_count;

            new_entries.push(e);
        }

        let mut compact_gudang = GudangEntri::dengan_kapasitas(new_entries.len());
        compact_gudang.daftar_entri = new_entries;
        compact_gudang.nama = new_nama;
        self.gudang = compact_gudang;

        if (self.root.0 as usize) < old_to_new.len() && old_to_new[self.root.0 as usize] != u32::MAX {
            self.root = IdEntri(old_to_new[self.root.0 as usize]);
        }

        self.invalidasi_cache();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::penghapus::HasilPenghapusan;
    use fast_disk_analyze_platform::JenisEntri;

    #[test]
    fn test_terapkan_penghapusan_dan_propagasi_delta() {
        let mut gudang = GudangEntri::baru();
        let id_root_nama = gudang.nama.simpan_nama(b"root");
        let id_dira_nama = gudang.nama.simpan_nama(b"dir_a");
        let id_fileb_nama = gudang.nama.simpan_nama(b"file_b.txt");
        let id_filec_nama = gudang.nama.simpan_nama(b"file_c.txt");

        // Id 0: Root (size 500)
        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 500,
            ukuran_blok: 500,
            induk: INDUK_AKAR,
            id_nama: id_root_nama.0,
            anak_awal: 1,
            jumlah_anak: 2,
            jumlah_tautan: 1,
            jenis: JenisEntri::Direktori as u8,
            penanda: 0,
            _padding: 0,
        });

        // Id 1: dir_a (size 300)
        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 300,
            ukuran_blok: 300,
            induk: 0,
            id_nama: id_dira_nama.0,
            anak_awal: 3,
            jumlah_anak: 1,
            jumlah_tautan: 1,
            jenis: JenisEntri::Direktori as u8,
            penanda: 0,
            _padding: 0,
        });

        // Id 2: file_b.txt (size 200)
        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 200,
            ukuran_blok: 200,
            induk: 0,
            id_nama: id_fileb_nama.0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Berkas as u8,
            penanda: 0,
            _padding: 0,
        });

        // Id 3: file_c.txt (size 300)
        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 300,
            ukuran_blok: 300,
            induk: 1,
            id_nama: id_filec_nama.0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Berkas as u8,
            penanda: 0,
            _padding: 0,
        });

        let mut pohon = PohonSistemBerkas::baru(
            PathBuf::from("/test"),
            gudang,
            IdEntri(0),
            ErrorLog::new(),
        );

        assert_eq!(pohon.children_ids(IdEntri(0)).len(), 2);
        assert_eq!(pohon.gudang.ambil(IdEntri(0)).ukuran_blok, 500);

        // Hapus file_b.txt (id 2)
        let hasil = HasilPenghapusan {
            terhapus: vec![IdEntri(2)],
            gagal: vec![],
            dibebaskan_asli: 200,
            dibebaskan_blok: 200,
        };

        pohon.terapkan_penghapusan(&hasil);

        // Verifikasi file_b tidak lagi muncul di anak root
        let children = pohon.children_ids(IdEntri(0));
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], IdEntri(1));

        // Verifikasi ukuran root berkurang 200 -> 300
        assert_eq!(pohon.gudang.ambil(IdEntri(0)).ukuran_blok, 300);
        assert_eq!(pohon.gudang.ambil(IdEntri(0)).ukuran_asli, 300);

        // Verifikasi rasio yatim = 1/4 = 0.25
        assert_eq!(pohon.rasio_yatim(), 0.25);

        // Jalankan kompaksi
        pohon.kompaksi();
        assert_eq!(pohon.gudang.jumlah(), 3);
        assert_eq!(pohon.rasio_yatim(), 0.0);
        assert_eq!(pohon.children_ids(IdEntri(0)).len(), 1);
    }

    #[test]
    fn test_terapkan_subtree_dan_propagasi_delta() {
        let mut gudang = GudangEntri::baru();
        let id_root_nama = gudang.nama.simpan_nama(b"root");
        let id_sub_nama = gudang.nama.simpan_nama(b"sub");
        let id_old_nama = gudang.nama.simpan_nama(b"old.txt");

        // Root (size 100)
        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 100,
            ukuran_blok: 100,
            induk: INDUK_AKAR,
            id_nama: id_root_nama.0,
            anak_awal: 1,
            jumlah_anak: 1,
            jumlah_tautan: 1,
            jenis: JenisEntri::Direktori as u8,
            penanda: 0,
            _padding: 0,
        });

        // Sub (size 100)
        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 100,
            ukuran_blok: 100,
            induk: 0,
            id_nama: id_sub_nama.0,
            anak_awal: 2,
            jumlah_anak: 1,
            jumlah_tautan: 1,
            jenis: JenisEntri::Direktori as u8,
            penanda: 0,
            _padding: 0,
        });

        // old.txt (size 100)
        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 100,
            ukuran_blok: 100,
            induk: 1,
            id_nama: id_old_nama.0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Berkas as u8,
            penanda: 0,
            _padding: 0,
        });

        let mut pohon = PohonSistemBerkas::baru(
            PathBuf::from("/test"),
            gudang,
            IdEntri(0),
            ErrorLog::new(),
        );

        // Buat subtree baru untuk "sub" yang sekarang berisi file baru berukuran 500
        let mut sub_gudang = GudangEntri::baru();
        let id_sub_new_nama = sub_gudang.nama.simpan_nama(b"sub");
        let id_new_file_nama = sub_gudang.nama.simpan_nama(b"new.txt");

        // Subtree root (sub, size 500)
        sub_gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 500,
            ukuran_blok: 500,
            induk: INDUK_AKAR,
            id_nama: id_sub_new_nama.0,
            anak_awal: 1,
            jumlah_anak: 1,
            jumlah_tautan: 1,
            jenis: JenisEntri::Direktori as u8,
            penanda: 0,
            _padding: 0,
        });

        // new.txt (size 500)
        sub_gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 500,
            ukuran_blok: 500,
            induk: 0,
            id_nama: id_new_file_nama.0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Berkas as u8,
            penanda: 0,
            _padding: 0,
        });

        pohon.terapkan_subtree(IdEntri(1), sub_gudang, IdEntri(0));

        // Verifikasi anak sub sekarang adalah file baru (new.txt)
        let sub_children = pohon.children_ids(IdEntri(1));
        assert_eq!(sub_children.len(), 1);
        let new_file_id = sub_children[0];
        assert_eq!(pohon.nama(new_file_id), "new.txt");
        assert_eq!(pohon.gudang.ambil(new_file_id).ukuran_blok, 500);

        // Verifikasi ukuran sub terupdate menjadi 500
        assert_eq!(pohon.gudang.ambil(IdEntri(1)).ukuran_blok, 500);

        // Verifikasi ukuran root terpropagasi delta (+400): 100 -> 500
        assert_eq!(pohon.gudang.ambil(IdEntri(0)).ukuran_blok, 500);
    }
}
