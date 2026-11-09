use crate::ids::IdEntri;
use lru::LruCache;
use rustc_hash::FxHashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SortKey {
    UkuranMenurun,     // Ukuran blok/asli terbesar dulu (default)
    UkuranMenaik,      // Ukuran terkecil dulu
    NamaAZ,            // Abjad A-Z
    NamaZA,            // Abjad Z-A
    JumlahAnakMenurun, // Folder paling padat isi anak dulu
    JumlahAnakMenaik,  // Folder isi anak paling sedikit dulu
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SizeMode {
    UkuranAsli, // Apparent size (EndOfFile)
    UkuranBlok, // Allocated disk blocks
}

pub struct ViewCache {
    pub sort_cache: FxHashMap<(IdEntri, SortKey), Arc<[IdEntri]>>,
    pub filter_cache: LruCache<(IdEntri, String), Arc<[IdEntri]>>,
    max_sort_entries: usize,
}

impl ViewCache {
    pub fn baru() -> Self {
        Self {
            sort_cache: FxHashMap::default(),
            filter_cache: LruCache::new(NonZeroUsize::new(256).unwrap()),
            max_sort_entries: 1024,
        }
    }

    pub fn simpan_sort(&mut self, id: IdEntri, key: SortKey, list: Arc<[IdEntri]>) {
        if self.sort_cache.len() >= self.max_sort_entries {
            // Bersihkan separuh atau reset jika cache melebihi batas plafon memori
            self.sort_cache.clear();
        }
        self.sort_cache.insert((id, key), list);
    }

    pub fn invalidasi_semua(&mut self) {
        self.sort_cache.clear();
        self.filter_cache.clear();
    }

    pub fn invalidasi_entri(&mut self, id: IdEntri) {
        self.sort_cache.retain(|&(k_id, _), _| k_id != id);
        self.filter_cache.clear();
    }

    pub fn invalidasi_banyak(&mut self, ids: &[IdEntri]) {
        self.sort_cache.retain(|&(k_id, _), _| !ids.contains(&k_id));
        self.filter_cache.clear();
    }
}

impl Default for ViewCache {
    fn default() -> Self {
        Self::baru()
    }
}
