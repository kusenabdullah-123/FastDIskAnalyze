use fast_disk_analyze_core::entri::{JenisEntri, RekamanEntri};
use fast_disk_analyze_core::error::ErrorLog;
use fast_disk_analyze_core::gudang::GudangEntri;
use fast_disk_analyze_core::ids::{IdEntri, INDUK_AKAR};
use fast_disk_analyze_core::model::PohonSistemBerkas;
use fast_disk_analyze_core::view::SortKey;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[test]
fn test_benchmark_1000_render_sorted_100k_entri() {
    let mut gudang = GudangEntri::baru();
    let root_name = gudang.nama.simpan_nama(b"root");

    // 1. Buat Root
    gudang.tambah_entri(RekamanEntri {
        ukuran_asli: 0,
        ukuran_blok: 0,
        induk: INDUK_AKAR,
        id_nama: root_name.0,
        anak_awal: 1,
        jumlah_anak: 100_000,
        jumlah_tautan: 1,
        jenis: JenisEntri::Direktori as u8,
        penanda: 0,
        _padding: 0,
    });

    // 2. Buat 100.000 anak sintetis
    let name_file = gudang.nama.simpan_nama(b"berkas_data.bin");
    for i in 0..100_000 {
        // Variasi ukuran acak sederhana
        let sz = ((i * 37 + 101) % 1_000_000) as u64;
        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: sz,
            ukuran_blok: (sz + 511) / 512 * 512,
            induk: 0,
            id_nama: name_file.0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Berkas as u8,
            penanda: 0,
            _padding: 0,
        });
    }

    let model = PohonSistemBerkas::baru(PathBuf::from("/"), gudang, IdEntri(0), ErrorLog::new());

    // Panggilan pertama (cold sort O(K log K))
    let first_sort_start = Instant::now();
    let initial_res = model.sorted(IdEntri(0), SortKey::UkuranMenurun);
    let first_sort_time = first_sort_start.elapsed();
    assert_eq!(initial_res.len(), 100_000);
    println!("Waktu cold sort pertama (100k entri): {:?}", first_sort_time);

    // Benchmark: 1.000 kali render UI virtual scroll (cache hits)
    let bench_start = Instant::now();
    for _ in 0..1_000 {
        let sorted = model.sorted(IdEntri(0), SortKey::UkuranMenurun);
        // Pastikan optimizer tidak menghilangkan loop
        assert_eq!(sorted.len(), 100_000);
    }
    let total_elapsed = bench_start.elapsed();
    println!(
        "Total waktu 1.000 panggilan sorted() via view_cache: {:?} (rata-rata: {:.3} µs / render)",
        total_elapsed,
        total_elapsed.as_micros() as f64 / 1000.0
    );

    // Kriteria Penerimaan M3: 1000 render sorted() pada 100k entri < 10 ms total
    assert!(
        total_elapsed < Duration::from_millis(10),
        "1.000 render cache melebihi 10 ms! Terukur: {:?}",
        total_elapsed
    );
}

#[test]
fn test_filter_zero_allocation_case_insensitive() {
    let mut gudang = GudangEntri::baru();
    let r_name = gudang.nama.simpan_nama(b"root");
    let name_a = gudang.nama.simpan_nama(b"Gambar_Liburan.PNG");
    let name_b = gudang.nama.simpan_nama(b"Dokumen_Penting.pdf");
    let name_c = gudang.nama.simpan_nama(b"icon_app.png");
    let name_d = gudang.nama.simpan_nama(b"setup.exe");

    gudang.tambah_entri(RekamanEntri {
        ukuran_asli: 0,
        ukuran_blok: 0,
        induk: INDUK_AKAR,
        id_nama: r_name.0,
        anak_awal: 1,
        jumlah_anak: 4,
        jumlah_tautan: 1,
        jenis: JenisEntri::Direktori as u8,
        penanda: 0,
        _padding: 0,
    });

    for name_id in [name_a, name_b, name_c, name_d] {
        gudang.tambah_entri(RekamanEntri {
            ukuran_asli: 100,
            ukuran_blok: 512,
            induk: 0,
            id_nama: name_id.0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Berkas as u8,
            penanda: 0,
            _padding: 0,
        });
    }

    let model = PohonSistemBerkas::baru(PathBuf::from("/"), gudang, IdEntri(0), ErrorLog::new());

    // Cari ".png" (harus cocok dengan .PNG dan .png)
    let matched_png = model.filtered(IdEntri(0), ".png");
    assert_eq!(matched_png.len(), 2);
    assert_eq!(matched_png[0], IdEntri(1)); // Gambar_Liburan.PNG
    assert_eq!(matched_png[1], IdEntri(3)); // icon_app.png

    // Cari "DOKUMEN" (harus cocok dengan Dokumen_Penting.pdf)
    let matched_doc = model.filtered(IdEntri(0), "DOKUMEN");
    assert_eq!(matched_doc.len(), 1);
    assert_eq!(matched_doc[0], IdEntri(2));

    // Cari string yang tidak ada
    let matched_none = model.filtered(IdEntri(0), "xyz");
    assert_eq!(matched_none.len(), 0);

    // Cari string kosong (harus mengembalikan semua anak)
    let matched_all = model.filtered(IdEntri(0), "");
    assert_eq!(matched_all.len(), 4);
}

#[test]
fn test_ancestor_chain_dan_depth() {
    let mut gudang = GudangEntri::baru();
    let n_root = gudang.nama.simpan_nama(b"root");
    let n_usr = gudang.nama.simpan_nama(b"usr");
    let n_local = gudang.nama.simpan_nama(b"local");
    let n_bin = gudang.nama.simpan_nama(b"bin");
    let n_app = gudang.nama.simpan_nama(b"myapp");

    // Root (id 0)
    gudang.tambah_entri(RekamanEntri {
        ukuran_asli: 0,
        ukuran_blok: 1000,
        induk: INDUK_AKAR,
        id_nama: n_root.0,
        anak_awal: 1,
        jumlah_anak: 1,
        jumlah_tautan: 1,
        jenis: JenisEntri::Direktori as u8,
        penanda: 0,
        _padding: 0,
    });

    // usr (id 1, induk 0)
    gudang.tambah_entri(RekamanEntri {
        ukuran_asli: 0,
        ukuran_blok: 800,
        induk: 0,
        id_nama: n_usr.0,
        anak_awal: 2,
        jumlah_anak: 1,
        jumlah_tautan: 1,
        jenis: JenisEntri::Direktori as u8,
        penanda: 0,
        _padding: 0,
    });

    // local (id 2, induk 1)
    gudang.tambah_entri(RekamanEntri {
        ukuran_asli: 0,
        ukuran_blok: 500,
        induk: 1,
        id_nama: n_local.0,
        anak_awal: 3,
        jumlah_anak: 1,
        jumlah_tautan: 1,
        jenis: JenisEntri::Direktori as u8,
        penanda: 0,
        _padding: 0,
    });

    // bin (id 3, induk 2)
    gudang.tambah_entri(RekamanEntri {
        ukuran_asli: 0,
        ukuran_blok: 300,
        induk: 2,
        id_nama: n_bin.0,
        anak_awal: 4,
        jumlah_anak: 1,
        jumlah_tautan: 1,
        jenis: JenisEntri::Direktori as u8,
        penanda: 0,
        _padding: 0,
    });

    // myapp (id 4, induk 3)
    gudang.tambah_entri(RekamanEntri {
        ukuran_asli: 100,
        ukuran_blok: 150,
        induk: 3,
        id_nama: n_app.0,
        anak_awal: 0,
        jumlah_anak: 0,
        jumlah_tautan: 1,
        jenis: JenisEntri::Berkas as u8,
        penanda: 0,
        _padding: 0,
    });

    let model = PohonSistemBerkas::baru(PathBuf::from("/"), gudang, IdEntri(0), ErrorLog::new());

    // Cek ancestor chain
    let chain = model.ancestor_chain(IdEntri(4));
    assert_eq!(
        chain,
        vec![IdEntri(0), IdEntri(1), IdEntri(2), IdEntri(3), IdEntri(4)]
    );

    // Cek kedalaman (depth)
    assert_eq!(model.depth(IdEntri(0)), 0);
    assert_eq!(model.depth(IdEntri(1)), 1);
    assert_eq!(model.depth(IdEntri(4)), 4);

    // Cek persentase ukuran (size_percentage)
    // bin = 300, myapp = 150 -> 50.0%
    assert_eq!(model.size_percentage(IdEntri(4)), 50.0);
    // usr = 800, root = 1000 -> 80.0%
    assert_eq!(model.size_percentage(IdEntri(1)), 80.0);
}
