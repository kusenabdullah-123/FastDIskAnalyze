use fast_disk_analyze_core::engine::{
    AkurasiTautan, CapPolicy, DefaultPengendaliScan, HasilScan, PengendaliScan,
    PenjadwalScan, PermintaanScan, ProgressEvent,
};
use fast_disk_analyze_core::model::PohonSistemBerkas;
use fast_disk_analyze_core::view::{SizeMode, SortKey};
use fast_disk_analyze_platform::{default_probe, JenisEntri};
use std::env;
use std::path::PathBuf;
use std::time::Instant;

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let target_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from(".")
    };

    println!("============================================================");
    println!("Fast Disk Analyze — CLI Parallel Scanner (Milestone M2)");
    println!("Target Path: {}", target_path.display());

    // 1. Hardware Probe
    let probe = default_probe();
    let medium = probe.probe(&target_path);
    println!("Tipe Media : {:?}", medium);

    let policy = PenjadwalScan::policy(medium);
    println!(
        "Kebijakan  : {} worker threads (work-stealing), antrean: {}",
        policy.threads, policy.queue_depth
    );
    println!("------------------------------------------------------------");

    // 2. Siapkan PengendaliScan & Permintaan
    let mut controller = DefaultPengendaliScan::new();
    let req = PermintaanScan {
        root_path: target_path.clone(),
        medium,
        mode: SizeMode::UkuranBlok,
        cap_policy: CapPolicy::None,
        akurasi_tautan: AkurasiTautan::Cepat,
        lintas_mount: false,
    };

    let start = Instant::now();
    println!("Memulai pemindaian paralel multi-worker...");

    let rx = controller.mulai(req);

    let mut model_hasil: Option<PohonSistemBerkas> = None;
    let mut scan_selesai = false;

    while let Ok(event) = rx.recv() {
        match event {
            ProgressEvent::Started { .. } => {}
            ProgressEvent::Tick {
                items,
                bytes,
                elapsed_ms,
                ..
            } => {
                let elapsed_sec = elapsed_ms as f64 / 1000.0;
                let rate = if elapsed_sec > 0.0 {
                    items as f64 / elapsed_sec
                } else {
                    0.0
                };
                eprint!(
                    "\rProgres: {} entri ({:.0} entri/detik, {})...",
                    items,
                    rate,
                    format_bytes(bytes)
                );
            }
            ProgressEvent::MemoryPressure { used_nodes, cap } => {
                eprintln!("\n[!] Tekanan memori: {}/{} entri", used_nodes, cap);
            }
            ProgressEvent::Finished(hasil_scan) => {
                eprintln!(); // newline setelah progres
                scan_selesai = true;
                match hasil_scan {
                    HasilScan::Complete(res) => {
                        model_hasil = Some(PohonSistemBerkas::baru(
                            res.gudang,
                            res.root,
                            res.errors,
                        ));
                    }
                    HasilScan::Incomplete { result: res, reason } => {
                        println!("[!] Pemindaian tidak lengkap: {:?}", reason);
                        model_hasil = Some(PohonSistemBerkas::baru(
                            res.gudang,
                            res.root,
                            res.errors,
                        ));
                    }
                }
                break;
            }
            ProgressEvent::Cancelled { items } => {
                eprintln!();
                println!("[!] Pemindaian dibatalkan setelah {} entri.", items);
                return;
            }
            ProgressEvent::Failed { error } => {
                eprintln!();
                println!("[X] Pemindaian gagal: {error}");
                return;
            }
        }
    }

    let elapsed = start.elapsed();
    println!("------------------------------------------------------------");

    if !scan_selesai || model_hasil.is_none() {
        println!("Pemindaian berhenti sebelum selesai.");
        return;
    }

    let model = model_hasil.unwrap();
    let total_entri = model.gudang.jumlah();
    let root_node = *model.gudang.ambil(model.root);

    let mut total_berkas = 0;
    let mut total_direktori = 0;
    let mut total_tautan = 0;
    let mut total_lainnya = 0;
    let mut total_hardlinks = 0;

    for entri in &model.gudang.daftar_entri {
        match entri.jenis() {
            JenisEntri::Berkas => total_berkas += 1,
            JenisEntri::Direktori => total_direktori += 1,
            JenisEntri::Tautan => total_tautan += 1,
            JenisEntri::Lainnya => total_lainnya += 1,
        }
        if entri.jumlah_tautan > 1 {
            total_hardlinks += 1;
        }
    }

    let elapsed_sec = elapsed.as_secs_f64();
    let throughput_entri = if elapsed_sec > 0.0 {
        total_entri as f64 / elapsed_sec
    } else {
        0.0
    };
    let throughput_mb = if elapsed_sec > 0.0 {
        (root_node.ukuran_asli as f64 / (1024.0 * 1024.0)) / elapsed_sec
    } else {
        0.0
    };

    println!("Status     : SELESAI");
    println!(
        "Waktu      : {:.3?} ({:.0} entri/detik, {:.1} MB/detik)",
        elapsed, throughput_entri, throughput_mb
    );
    println!("Total Item : {} entri", total_entri);
    println!(" - Berkas    : {}", total_berkas);
    println!(" - Direktori : {}", total_direktori);
    println!(" - Tautan    : {}", total_tautan);
    println!(" - Lainnya   : {}", total_lainnya);
    println!(" - Hardlinks : {}", total_hardlinks);
    println!(" - Error     : {}", model.errors.jumlah());
    println!(
        "Ukuran Asli: {} ({} bytes)",
        format_bytes(root_node.ukuran_asli),
        root_node.ukuran_asli
    );
    println!(
        "Ukuran Blok: {} ({} bytes)",
        format_bytes(root_node.ukuran_blok),
        root_node.ukuran_blok
    );

    println!("------------------------------------------------------------");
    println!("TOP DIREKTORI / BERKAS TERBESAR:");
    println!(
        "{:<4} {:<6} {:<12} {:<12} {:>6}  {}",
        "No", "Tipe", "Ukuran Asli", "Ukuran Blok", "% Disk", "Nama"
    );
    println!("{:-<60}", "");

    let sorted_children = model.sorted(model.root, SortKey::UkuranMenurun);
    let top_limit = sorted_children.len().min(15);

    for (i, &child_id) in sorted_children[..top_limit].iter().enumerate() {
        let child = model.gudang.ambil(child_id);
        let nama = model.nama(child_id);
        let tipe = match child.jenis() {
            JenisEntri::Direktori => "[DIR]",
            JenisEntri::Berkas => "[FILE]",
            JenisEntri::Tautan => "[LINK]",
            JenisEntri::Lainnya => "[ETC]",
        };

        let pct = if root_node.ukuran_blok > 0 {
            (child.ukuran_blok as f64 / root_node.ukuran_blok as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "{:<4} {:<6} {:<12} {:<12} {:>5.1}%  {}",
            i + 1,
            tipe,
            format_bytes(child.ukuran_asli),
            format_bytes(child.ukuran_blok),
            pct,
            nama
        );
    }

    println!("============================================================");
}
