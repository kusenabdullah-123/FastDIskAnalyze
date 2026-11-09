use fast_disk_analyze_platform::{
    default_dir_iterator, default_probe, CancellationToken, JenisEntri,
};
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
    println!("Fast Disk Analyze — CLI Headless Scanner (Milestone M1)");
    println!("Target Path: {}", target_path.display());

    // 1. Probe tipe penyimpanan
    let probe = default_probe();
    let medium = probe.probe(&target_path);
    println!("Tipe Media : {:?}", medium);

    let policy = fast_disk_analyze_core::engine::PenjadwalScan::policy(medium);
    println!(
        "Kebijakan  : {} worker threads, kedalaman antrean: {}",
        policy.threads, policy.queue_depth
    );
    println!("------------------------------------------------------------");

    // 2. Siapkan CancellationToken dan handler Ctrl+C
    let token = CancellationToken::new();
    let token_clone = token.clone();
    let _ = ctrlc_handler(move || {
        println!("\n[!] Menerima sinyal pembatalan (Ctrl+C)... Menghentikan I/O...");
        token_clone.cancel();
    });

    // 3. Jalankan pemindaian direktori nyata via HAL
    let mut iterator = default_dir_iterator();
    let mut total_entri: u64 = 0;
    let mut total_berkas: u64 = 0;
    let mut total_direktori: u64 = 0;
    let mut total_tautan: u64 = 0;
    let mut total_lainnya: u64 = 0;
    let mut total_apparent_bytes: u64 = 0;
    let mut total_block_bytes: u64 = 0;
    let mut total_hardlinks: u64 = 0;
    let mut total_errors: u64 = 0;

    let start = Instant::now();
    println!("Memulai pemindaian via getdents64 + statx...");

    let res = iterator.walk(&target_path, &token, &mut |raw| {
        total_entri += 1;
        total_apparent_bytes += raw.ukuran_asli;
        total_block_bytes += raw.ukuran_blok;

        match raw.jenis {
            JenisEntri::Berkas => total_berkas += 1,
            JenisEntri::Direktori => total_direktori += 1,
            JenisEntri::Tautan => total_tautan += 1,
            JenisEntri::Lainnya => total_lainnya += 1,
        }

        if raw.jumlah_tautan > 1 {
            total_hardlinks += 1;
        }

        if raw.error.is_some() {
            total_errors += 1;
        }

        // Cetak progres per 50.000 entri
        if total_entri % 50_000 == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            let rate = if elapsed > 0.0 {
                total_entri as f64 / elapsed
            } else {
                0.0
            };
            eprint!(
                "\rProgres: {} entri ({:.0} entri/detik, {})...",
                total_entri,
                rate,
                format_bytes(total_apparent_bytes)
            );
        }
    });

    let elapsed = start.elapsed();
    eprintln!(); // newline setelah progres
    println!("------------------------------------------------------------");

    match res {
        Ok(()) => {
            println!("Status     : SELESAI");
        }
        Err(e) => {
            println!("Status     : BERHENTI ({e})");
        }
    }

    let elapsed_sec = elapsed.as_secs_f64();
    let throughput_entri = if elapsed_sec > 0.0 {
        total_entri as f64 / elapsed_sec
    } else {
        0.0
    };
    let throughput_mb = if elapsed_sec > 0.0 {
        (total_apparent_bytes as f64 / (1024.0 * 1024.0)) / elapsed_sec
    } else {
        0.0
    };

    println!("Waktu      : {:.3?} ({:.0} entri/detik, {:.1} MB/detik)", elapsed, throughput_entri, throughput_mb);
    println!("Total Item : {} entri", total_entri);
    println!(" - Berkas    : {}", total_berkas);
    println!(" - Direktori : {}", total_direktori);
    println!(" - Tautan    : {}", total_tautan);
    println!(" - Lainnya   : {}", total_lainnya);
    println!(" - Hardlinks : {}", total_hardlinks);
    println!(" - Error/Skip: {}", total_errors);
    println!("Ukuran Asli: {} ({} bytes)", format_bytes(total_apparent_bytes), total_apparent_bytes);
    println!("Ukuran Blok: {} ({} bytes)", format_bytes(total_block_bytes), total_block_bytes);
    println!("============================================================");
}

fn ctrlc_handler<F: Fn() + Send + Sync + 'static>(_handler: F) -> Result<(), ()> {
    Ok(())
}
