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

pub struct PohonSistemBerkas {
    pub gudang: GudangEntri,
    pub root: IdEntri,
    pub errors: ErrorLog,
    view_cache: ViewCache,
}

impl PohonSistemBerkas {
    pub fn baru(gudang: GudangEntri, root: IdEntri, errors: ErrorLog) -> Self {
        Self {
            gudang,
            root,
            errors,
            view_cache: ViewCache::baru(),
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

    /// Mendapatkan slice IdEntri untuk anak-anak direktori.
    pub fn children_ids(&self, id: IdEntri) -> Vec<IdEntri> {
        let entri = self.gudang.ambil(id);
        (entri.anak_awal..entri.anak_awal + entri.jumlah_anak)
            .map(IdEntri)
            .collect()
    }

    pub fn sorted(&mut self, id: IdEntri, key: SortKey) -> Arc<[IdEntri]> {
        if let Some(cached) = self.view_cache.sort_cache.get(&(id, key)) {
            return cached.clone();
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
        }

        let arc: Arc<[IdEntri]> = ids.into();
        self.view_cache.sort_cache.insert((id, key), arc.clone());
        arc
    }

    pub fn filtered(&mut self, id: IdEntri, q: &str) -> Arc<[IdEntri]> {
        let cache_key = (id, q.to_string());
        if let Some(cached) = self.view_cache.filter_cache.get(&cache_key) {
            return cached.clone();
        }

        let q_bytes = q.as_bytes();
        let q_lower: Vec<u8> = q_bytes.iter().map(|b| b.to_ascii_lowercase()).collect();

        let ids = self.children_ids(id);
        let hasil: Vec<IdEntri> = ids
            .into_iter()
            .filter(|&child_id| {
                let nama_bytes = self.gudang.nama.ambil_nama(IdNama(self.gudang.ambil(child_id).id_nama));
                if q_lower.is_empty() {
                    return true;
                }
                // Case-insensitive ASCII substring search tanpa alokasi String
                nama_bytes
                    .windows(q_lower.len())
                    .any(|w| w.iter().map(|b| b.to_ascii_lowercase()).eq(q_lower.iter().copied()))
            })
            .collect();

        let arc: Arc<[IdEntri]> = hasil.into();
        self.view_cache.filter_cache.put(cache_key, arc.clone());
        arc
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

    pub fn invalidasi_cache(&mut self) {
        self.view_cache.invalidasi_semua();
    }
}

