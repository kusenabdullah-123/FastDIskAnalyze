use crate::ids::IdEntri;
use fast_disk_analyze_platform::FsError;

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
}

impl Default for Penghapus {
    fn default() -> Self {
        Self::new()
    }
}

