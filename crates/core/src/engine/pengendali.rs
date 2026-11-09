use crate::engine::pemindai::Pemindai;
use crate::engine::pengawas::CapPolicy;
use crate::engine::penjadwal::PenjadwalScan;
use crate::error::ErrorLog;
use crate::gudang::GudangEntri;
use crate::ids::IdEntri;
use crate::model::PohonSistemBerkas;
use crate::view::SizeMode;
use crossbeam_channel::{bounded, Receiver};
use fast_disk_analyze_platform::{CancellationToken, MediumKind, ScanError};
use std::path::PathBuf;
use std::thread;
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

/// Implementasi standar PengendaliScan dengan worker thread pool
pub struct DefaultPengendaliScan {
    token: CancellationToken,
}

impl DefaultPengendaliScan {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    pub fn token(&self) -> &CancellationToken {
        &self.token
    }
}

impl Default for DefaultPengendaliScan {
    fn default() -> Self {
        Self::new()
    }
}

impl PengendaliScan for DefaultPengendaliScan {
    fn mulai(&mut self, req: PermintaanScan) -> Receiver<ProgressEvent> {
        let (tx, rx) = bounded(64);
        let token = CancellationToken::new();
        self.token = token.clone();

        thread::spawn(move || {
            let policy = PenjadwalScan::policy(req.medium);
            let scan_res = Pemindai::scan_parallel(
                &req.root_path,
                req.medium,
                policy.threads,
                &token,
                Some(tx.clone()),
            );

            if let Err(e) = scan_res {
                if !token.is_cancelled() {
                    let _ = tx.send(ProgressEvent::Failed { error: e });
                }
            }
        });

        rx
    }

    fn batalkan(&self) {
        self.token.cancel();
    }

    fn scan_subtree(&mut self, model: &PohonSistemBerkas, id: IdEntri) -> Receiver<ProgressEvent> {
        let path = model.fs_path_verbatim(id);
        let req = PermintaanScan {
            root_path: path.0,
            medium: MediumKind::Unknown,
            mode: SizeMode::UkuranBlok,
            cap_policy: CapPolicy::None,
            akurasi_tautan: AkurasiTautan::Cepat,
            lintas_mount: false,
        };
        self.mulai(req)
    }
}
