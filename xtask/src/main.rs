use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match command {
        "bench" => {
            println!("Menjalankan benchmark harness...");
        }
        "generate-synthetic" => {
            let count = args
                .get(2)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1_000_000);
            println!("Membuat pohon berkas sintetis dengan {count} entri...");
        }
        "baseline" => {
            println!("Merekam baseline FastDiskScan lama...");
        }
        _ => {
            println!("Penggunaan xtask:");
            println!("  cargo run -p xtask -- bench");
            println!("  cargo run -p xtask -- generate-synthetic [jumlah_entri]");
            println!("  cargo run -p xtask -- baseline");
        }
    }
}

