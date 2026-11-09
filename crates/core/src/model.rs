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
    pub gudang: GudangEntri,
    pub root: IdEntri,
    pub errors: ErrorLog,
    view_cache: std::sync::RwLock<ViewCache>,
}

impl PohonSistemBerkas {
    pub fn baru(gudang: GudangEntri, root: IdEntri, errors: ErrorLog) -> Self {
        Self {
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
            self.gudang.ambil(id).jumlah_anak
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
        let mut parts = Vec::new();
        let mut curr = id.0;

        while curr != INDUK_AKAR && (curr as usize) < self.gudang.jumlah() {
            let entri = self.gudang.ambil(IdEntri(curr));
            parts.push(self.gudang.nama.ambil_nama(IdNama(entri.id_nama)));
            curr = entri.induk;
        }

        parts.reverse();
        let mut path_buf = PathBuf::new();
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
}
