use crate::ids::IdEntri;
use lru::LruCache;
use rustc_hash::FxHashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SortKey {
    UkuranMenurun,
    UkuranMenaik,
    NamaAZ,
    NamaZA,
    JumlahAnakMenurun,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SizeMode {
    UkuranAsli, // Apparent size
    UkuranBlok, // Allocated disk blocks
}

pub struct ViewCache {
    pub sort_cache: FxHashMap<(IdEntri, SortKey), Arc<[IdEntri]>>,
    pub filter_cache: LruCache<(IdEntri, String), Arc<[IdEntri]>>,
}

impl ViewCache {
    pub fn baru() -> Self {
        Self {
            sort_cache: FxHashMap::default(),
            filter_cache: LruCache::new(NonZeroUsize::new(256).unwrap()),
        }
    }

    pub fn invalidasi_semua(&mut self) {
        self.sort_cache.clear();
        self.filter_cache.clear();
    }

    pub fn invalidasi_entri(&mut self, id: IdEntri) {
        self.sort_cache.retain(|&(k_id, _), _| k_id != id);
        // Untuk LRU, kita bersihkan seluruh cache filter untuk kesederhanaan
        self.filter_cache.clear();
    }
}

impl Default for ViewCache {
    fn default() -> Self {
        Self::baru()
    }
}

