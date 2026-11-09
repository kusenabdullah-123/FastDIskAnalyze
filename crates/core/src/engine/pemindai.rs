use crate::engine::pengendali::{HasilLengkap, ProgressEvent};
use crate::engine::perakit::{Perakit, WorkerScanResult};
use crate::entri::{JenisEntri, RekamanEntri};
use crate::gudang::GudangEntri;
use crate::ids::{IdEntri, INDUK_AKAR};
use crossbeam_channel::Sender;
use crossbeam_deque::{Injector, Worker};
use fast_disk_analyze_platform::{
    default_dir_iterator, CancellationToken, MediumKind, ScanError,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Tugas pemindaian satu direktori yang didistribusikan antar worker
#[derive(Clone, Debug)]
pub struct DirTask {
    pub path: PathBuf,
    pub parent_token: u32,
    pub my_token: u32,
}

pub struct Pemindai;

impl Pemindai {
    /// Menjalankan pemindaian paralel berbasis work-stealing multithread
    pub fn scan_parallel(
        root_path: &Path,
        medium: MediumKind,
        num_threads: usize,
        token: &CancellationToken,
        progress_tx: Option<Sender<ProgressEvent>>,
    ) -> Result<HasilLengkap, ScanError> {
        let start_time = Instant::now();
        let root_canonical = root_path.canonicalize().unwrap_or_else(|_| root_path.to_path_buf());

        if let Some(ref tx) = progress_tx {
            let _ = tx.send(ProgressEvent::Started { total_hint: None });
        }

        let injector = Arc::new(Injector::<DirTask>::new());
        // Masukkan task root ke injector
        injector.push(DirTask {
            path: root_canonical.clone(),
            parent_token: INDUK_AKAR,
            my_token: 0,
        });

        let next_token = Arc::new(AtomicU32::new(1));
        let active_workers = Arc::new(AtomicU32::new(0));

        let global_items = Arc::new(AtomicU64::new(0));
        let global_bytes = Arc::new(AtomicU64::new(0));
        let global_errors = Arc::new(AtomicU64::new(0));

        let mut workers = Vec::with_capacity(num_threads);
        let mut stealers = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            let w = Worker::new_fifo();
            stealers.push(w.stealer());
            workers.push(w);
        }

        let stealers = Arc::new(stealers);
        let is_running = Arc::new(AtomicBool::new(true));

        // Thread pemancar progres periodik (Tick coalesced tiap 100ms)
        let progress_handle = if let Some(tx) = progress_tx.clone() {
            let running = is_running.clone();
            let items = global_items.clone();
            let bytes = global_bytes.clone();
            let errs = global_errors.clone();
            let start = start_time;

            Some(thread::spawn(move || {
                while running.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(100));
                    let elapsed = start.elapsed().as_millis() as u64;
                    let event = ProgressEvent::Tick {
                        items: items.load(Ordering::Relaxed),
                        bytes: bytes.load(Ordering::Relaxed),
                        errors: errs.load(Ordering::Relaxed),
                        elapsed_ms: elapsed,
                    };
                    let _ = tx.try_send(event); // Coalesced: jika penuh, drop
                }
            }))
        } else {
            None
        };

        // Spawn N worker threads
        let mut worker_handles = Vec::with_capacity(num_threads);

        for worker in workers {
            let injector = injector.clone();
            let stealers = stealers.clone();
            let active = active_workers.clone();
            let next_tok = next_token.clone();
            let token = token.clone();
            let g_items = global_items.clone();
            let g_bytes = global_bytes.clone();
            let g_errs = global_errors.clone();

            let handle = thread::spawn(move || {
                let mut local_gudang = GudangEntri::baru();
                let mut local_tautan = Vec::new();
                let mut local_iterator = default_dir_iterator();
                let mut local_items = 0u64;
                let mut local_bytes = 0u64;
                let mut local_errors = Vec::new();

                loop {
                    if token.is_cancelled() {
                        break;
                    }

                    // 1. Ambil task: local deque -> injector -> steal dari peer
                    let task = worker.pop().or_else(|| {
                        std::iter::repeat_with(|| {
                            injector.steal_batch_and_pop(&worker).or_else(|| {
                                stealers.iter().map(|s| s.steal()).collect()
                            })
                        })
                        .find(|s| !s.is_retry())
                        .and_then(|s| s.success())
                    });

                    let task = match task {
                        Some(t) => t,
                        None => {
                            // Cek kondisi selesai: tidak ada worker yang sedang aktif dan injector kosong
                            if active.load(Ordering::SeqCst) == 0 && injector.is_empty() {
                                break;
                            }
                            thread::yield_now();
                            continue;
                        }
                    };

                    active.fetch_add(1, Ordering::SeqCst);

                    // Jika task adalah Root (my_token == 0), catat entri Root
                    if task.my_token == 0 {
                        let root_name = task.path.file_name().map(|n| n.as_encoded_bytes()).unwrap_or(b"/");
                        let name_id = local_gudang.nama.simpan_nama(root_name);
                        local_gudang.tambah_entri(RekamanEntri {
                            ukuran_asli: 0,
                            ukuran_blok: 0,
                            induk: INDUK_AKAR,
                            id_nama: name_id.0,
                            anak_awal: 0, // my_token = 0
                            jumlah_anak: 0,
                            jumlah_tautan: 1,
                            jenis: JenisEntri::Direktori as u8,
                            penanda: 0,
                            _padding: 0,
                        });
                        local_items += 1;
                        g_items.fetch_add(1, Ordering::Relaxed);
                    }

                    let parent_token = task.my_token;
                    let mut sub_tasks = Vec::new();

                    let read_res = local_iterator.read_dir(&task.path, &token, &mut |raw| {
                        let name_id = local_gudang.nama.simpan_nama(raw.nama);
                        let local_idx = local_gudang.daftar_entri.len();
                        let is_dir = raw.jenis == JenisEntri::Direktori;

                        let (dir_token, child_path) = if is_dir {
                            let dt = next_tok.fetch_add(1, Ordering::Relaxed);
                            let mut cp = task.path.clone();
                            #[cfg(unix)]
                            {
                                use std::ffi::OsStr;
                                use std::os::unix::ffi::OsStrExt;
                                cp.push(OsStr::from_bytes(raw.nama));
                            }
                            #[cfg(not(unix))]
                            {
                                let s = String::from_utf8_lossy(raw.nama);
                                cp.push(s.as_ref());
                            }
                            (dt, Some(cp))
                        } else {
                            (0, None)
                        };

                        let rekaman = RekamanEntri {
                            ukuran_asli: raw.ukuran_asli,
                            ukuran_blok: raw.ukuran_blok,
                            induk: parent_token,
                            id_nama: name_id.0,
                            anak_awal: dir_token, // Simpan my_token sementara di anak_awal
                            jumlah_anak: 0,
                            jumlah_tautan: raw.jumlah_tautan,
                            jenis: raw.jenis as u8,
                            penanda: 0,
                            _padding: 0,
                        };

                        let delta_bytes = if raw.ukuran_blok > 0 {
                            raw.ukuran_blok
                        } else {
                            raw.ukuran_asli
                        };

                        local_gudang.tambah_entri(rekaman);
                        local_items += 1;
                        local_bytes += delta_bytes;
                        g_items.fetch_add(1, Ordering::Relaxed);
                        g_bytes.fetch_add(delta_bytes, Ordering::Relaxed);

                        if let Some(id_berkas) = raw.identitas {
                            local_tautan.push((local_idx, id_berkas));
                        }

                        if let Some(cp) = child_path {
                            sub_tasks.push(DirTask {
                                path: cp,
                                parent_token,
                                my_token: dir_token,
                            });
                        }
                    });

                    if let Err(e) = read_res {
                        local_errors.push((parent_token, e));
                        g_errs.fetch_add(1, Ordering::Relaxed);
                    }

                    for st in sub_tasks {
                        worker.push(st);
                    }

                    active.fetch_sub(1, Ordering::SeqCst);
                }

                WorkerScanResult {
                    daftar_entri: local_gudang.daftar_entri,
                    nama: local_gudang.nama,
                    tautan: local_tautan,
                    items_scanned: local_items,
                    bytes_scanned: local_bytes,
                    errors: local_errors,
                }
            });

            worker_handles.push(handle);
        }

        // Tunggu semua worker selesai
        let mut worker_results = Vec::with_capacity(num_threads);
        for h in worker_handles {
            if let Ok(res) = h.join() {
                worker_results.push(res);
            }
        }

        is_running.store(false, Ordering::Relaxed);
        if let Some(h) = progress_handle {
            let _ = h.join();
        }

        if token.is_cancelled() {
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(ProgressEvent::Cancelled {
                    items: global_items.load(Ordering::Relaxed),
                });
            }
            return Err(ScanError::Cancelled);
        }

        // Jalankan Perakitan (Assembly Pipeline)
        let total_tokens = next_token.load(Ordering::Relaxed);
        let (gudang, errors) = Perakit::rakit_multi_worker(worker_results, total_tokens);
        let elapsed = start_time.elapsed();

        let hasil = HasilLengkap {
            root_path: root_canonical,
            gudang,
            root: IdEntri(0),
            errors,
            elapsed,
            medium,
        };

        if let Some(ref tx) = progress_tx {
            let _ = tx.send(ProgressEvent::Finished(
                crate::engine::pengendali::HasilScan::Complete(hasil.clone()),
            ));
        }

        Ok(hasil)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_parallel_local_dir() {
        let token = CancellationToken::new();
        let res = Pemindai::scan_parallel(
            Path::new("src"),
            MediumKind::NvmeSsd,
            4,
            &token,
            None,
        );

        assert!(res.is_ok());
        let hasil = res.unwrap();
        assert!(hasil.gudang.jumlah() >= 2);

        let root = hasil.gudang.ambil(IdEntri(0));
        assert_eq!(root.induk, INDUK_AKAR);
        assert!(root.ukuran_asli > 0);
        assert!(root.jumlah_anak > 0);
    }

    #[test]
    fn test_scan_parallel_cancellation() {
        let token = CancellationToken::new();
        token.cancel(); // batalkan segera

        let res = Pemindai::scan_parallel(
            Path::new("src"),
            MediumKind::Hdd,
            2,
            &token,
            None,
        );

        assert!(matches!(res, Err(ScanError::Cancelled)));
    }
}
