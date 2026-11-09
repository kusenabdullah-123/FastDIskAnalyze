use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Token pembatalan untuk menghentikan operasi I/O dan traversal secara kooperatif.
#[derive(Clone, Default, Debug)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn check(&self) -> Result<(), ScanError> {
        if self.is_cancelled() {
            Err(ScanError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// Jenis media penyimpanan fisik hasil deteksi hardware probe.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MediumKind {
    Hdd,
    SataSsd,
    NvmeSsd,
    Network,
    Removable,
    Unknown,
}

/// Jenis entri sistem berkas. Disimpan sebagai u8 pada RekamanEntri (POD).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum JenisEntri {
    Berkas = 0,
    Direktori = 1,
    Tautan = 2,   // symlink / junction / reparse point: dicatat, TIDAK di-traverse
    Lainnya = 3,  // device, socket, fifo, dsb.
}

impl TryFrom<u8> for JenisEntri {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, ()> {
        match v {
            0 => Ok(Self::Berkas),
            1 => Ok(Self::Direktori),
            2 => Ok(Self::Tautan),
            3 => Ok(Self::Lainnya),
            _ => Err(()),
        }
    }
}

/// Identitas berkas unik sementara untuk dedup hardlink (hanya hidup di engine selama scan).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct IdentitasBerkas {
    pub volume: u64,   // Linux: st_dev       | Windows: VolumeSerialNumber
    pub berkas: u128,  // Linux: st_ino       | Windows: FileId (64/128 bit)
}

/// Kesalahan pembacaan entri berkas / direktori.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryError {
    PermissionDenied,
    NotFound,
    IoError(String),
}

/// Kesalahan pemindaian global.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanError {
    Cancelled,
    IoError(String),
    PathNotFound(PathBuf),
    PermissionDenied(PathBuf),
    Internal(String),
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "Pemindaian dibatalkan"),
            Self::IoError(e) => write!(f, "Kesalahan I/O: {e}"),
            Self::PathNotFound(p) => write!(f, "Jalur tidak ditemukan: {}", p.display()),
            Self::PermissionDenied(p) => write!(f, "Izin ditolak: {}", p.display()),
            Self::Internal(msg) => write!(f, "Kesalahan internal: {msg}"),
        }
    }
}

impl std::error::Error for ScanError {}

/// Kesalahan operasi sistem berkas (hapus, pindah sampah, buka).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FsError {
    NotFound,
    PermissionDenied,
    CrossDevice,
    IoError(String),
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Berkas atau direktori tidak ditemukan"),
            Self::PermissionDenied => write!(f, "Izin operasi berkas ditolak"),
            Self::CrossDevice => write!(f, "Operasi lintas perangkat sistem berkas"),
            Self::IoError(e) => write!(f, "Kesalahan I/O berkas: {e}"),
        }
    }
}

impl std::error::Error for FsError {}

/// Hasil pemindahan ke tempat sampah.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrashOutcome {
    Success,
    FallbackDeletePermanen,
}

/// Jalur representasi verbatim untuk operasi FsOps (menjaga format \\?\ di Windows dan byte asli di Linux).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerbatimPath(pub PathBuf);

impl VerbatimPath {
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Entri mentah yang di-emit oleh DirIterator saat membaca direktori.
pub struct RawEntry<'a> {
    pub induk: u32,
    pub nama: &'a [u8],               // byte mentah (Linux) / WTF-8 (Windows)
    pub jenis: JenisEntri,
    pub ukuran_asli: u64,
    pub ukuran_blok: u64,
    pub jumlah_tautan: u32,
    pub identitas: Option<IdentitasBerkas>, // hanya diisi bila jumlah_tautan > 1
    pub error: Option<EntryError>,
}

/// Informasi partisi/drive fisik atau logis yang terdeteksi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InformasiDrive {
    /// ID sistem atau path mount, mis. "C:\" atau "/dev/nvme0n1p2"
    pub id: String,
    /// Path root direktori pemindaian, mis. "C:\" atau "/" atau "/home"
    pub mount_point: PathBuf,
    /// Label ramah manusia, mis. "Local Disk (C:)" atau "Root Sistem (/)"
    pub label: String,
    /// Tipe filesystem, mis. "NTFS", "ext4", "btrfs", "exFAT"
    pub fs_type: String,
    /// Total kapasitas penyimpanan dalam byte
    pub total_bytes: u64,
    /// Kapasitas yang tersedia untuk pengguna dalam byte
    pub available_bytes: u64,
    /// Deteksi hardware media (NVMe, SATA SSD, HDD, Removable)
    pub medium: MediumKind,
}

impl InformasiDrive {
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }

    pub fn usage_percentage(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes() as f64 / self.total_bytes as f64) as f32
        }
    }
}

/// Trait untuk mendeteksi tipe medium penyimpanan fisik.
pub trait StorageProbe: Send + Sync {
    fn probe(&self, path: &Path) -> MediumKind;
    fn list_drives(&self) -> Vec<InformasiDrive>;
}

/// Trait untuk melakukan enumerasi isi direktori.
pub trait DirIterator: Send {
    /// Membaca direktori secara rekursif.
    fn walk(
        &mut self,
        root: &Path,
        token: &CancellationToken,
        emit: &mut dyn FnMut(RawEntry<'_>),
    ) -> Result<(), ScanError>;

    /// Membaca hanya 1 tingkat isi sebuah direktori tanpa rekursi (untuk engine traversal paralel).
    fn read_dir(
        &mut self,
        path: &Path,
        token: &CancellationToken,
        emit: &mut dyn FnMut(RawEntry<'_>),
    ) -> Result<(), ScanError>;
}

/// Hasil penyelesaian operasi async I/O.
pub struct Completion {
    pub id: u64,
    pub result: Result<usize, ScanError>,
}

/// Trait abstraksi antrean I/O asinkron (IOCP di Windows, io_uring di Linux).
pub trait IoBackend: Send + Sync {
    type Req: Send;
    fn submit_batch(&self, reqs: &mut [Self::Req]);
    fn poll_completions(&self, max: usize) -> Vec<Completion>;
    fn cancel_all(&self);
}

/// Trait operasi sistem berkas (penghapusan, trash, buka berkas).
pub trait FsOps: Send + Sync {
    fn delete_permanent(&self, path: &VerbatimPath) -> Result<(), FsError>;
    fn move_to_trash(&self, path: &VerbatimPath) -> Result<TrashOutcome, FsError>;
    fn path_is_cross_fs(&self, path: &Path, trash_root: &Path) -> bool;
    fn open_default(&self, path: &Path) -> Result<(), FsError>;
    fn reveal_in_file_manager(&self, path: &Path) -> Result<(), FsError>;
}

