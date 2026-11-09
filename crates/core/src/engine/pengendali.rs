use crate::engine::pengawas::CapPolicy;
use crate::error::ErrorLog;
use crate::gudang::GudangEntri;
use crate::ids::IdEntri;
use crate::model::PohonSistemBerkas;
use crate::view::SizeMode;
use crossbeam_channel::Receiver;
use fast_disk_analyze_platform::{MediumKind, ScanError};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AkurasiTautan {
    Cepat,  // default: tanpa handle per berkas di Windows
    Akurat, // ambil nlink lengkap
}

#[derive(Clone, Debug)]
pub struct PermintaanScan {
    pub root_path: PathBuf,
    pub medium: MediumKind,
    pub mode: SizeMode,
    pub cap_policy: CapPolicy,
    pub akurasi_tautan: AkurasiTautan,
    pub lintas_mount: bool,
}

#[derive(Clone, Debug)]
pub enum ProgressEvent {
    Started {
        total_hint: Option<u64>,
    },
    Tick {
        items: u64,
        bytes: u64,
        errors: u64,
        elapsed_ms: u64,
    },
    MemoryPressure {
        used_nodes: usize,
        cap: usize,
    },
    Finished(HasilScan),
    Cancelled {
        items: u64,
    },
    Failed {
        error: ScanError,
    },
}

#[derive(Clone, Debug)]
pub enum IncompleteReason {
    MemoryCapReached,
    CancelledEarly,
    PartialPermission,
}

#[derive(Clone, Debug)]
pub enum HasilScan {
    Complete(HasilLengkap),
    Incomplete {
        result: HasilLengkap,
        reason: IncompleteReason,
    },
}

#[derive(Clone, Debug)]
pub struct HasilLengkap {
    pub gudang: GudangEntri,
    pub root: IdEntri,
    pub errors: ErrorLog,
    pub elapsed: Duration,
    pub medium: MediumKind,
}

pub trait PengendaliScan: Send {
    fn mulai(&mut self, req: PermintaanScan) -> Receiver<ProgressEvent>;
    fn batalkan(&self);
    fn scan_subtree(&mut self, model: &PohonSistemBerkas, id: IdEntri) -> Receiver<ProgressEvent>;
}

