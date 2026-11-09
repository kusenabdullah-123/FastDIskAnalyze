# Desain UI/UX: fast-disk-analyze (Iced Framework)

Dokumen ini mendefinisikan alur pengalaman pengguna (UX), tata letak (layout), dan arsitektur antarmuka (UI) menggunakan framework Rust **Iced**.

**Prinsip Desain UI:**
1. **Zero-Lag (Anti Freeze)**: UI murni sebagai "penonton". Segala proses berat (baca disk, hapus) dilakukan di *background thread* (Engine).
2. **State Machine**: UI dikendalikan oleh *State* yang jelas (Loading -> Pilih Drive -> Scanning -> Explorer).
3. **Data-Dense**: Tampilan dirancang untuk *Power User*. Menampilkan informasi sebanyak mungkin (ukuran asli, ukuran blok, jumlah file) tanpa terasa sesak.
4. **Tema Gelap (Dark Mode)**: Kontras tinggi untuk kenyamanan mata, dengan aksen warna yang membedakan file, folder, dan tombol aksi berbahaya (hapus).

---

## 1. Alur Pengalaman Pengguna (User Flow)

Aplikasi berjalan melalui 4 fase layar utama:

### Fase 1: Splash Screen / Loading (Awal Buka)
*   **Tampilan**: Layar gelap kosong di tengahnya terdapat logo aplikasi dan ikon *Spinner* (🔄) yang berputar.
*   **Teks**: *"Mendeteksi perangkat penyimpanan..."*
*   **Aksi di Balik Layar**: Engine memanggil `StorageProbe` untuk mendeteksi Drive (C:\, D:\, /home, dll) beserta jenis mediumnya (NVMe, HDD, SSD).
*   **Transisi**: Setelah probe selesai (< 2 detik), layar berubah ke Fase 2.

### Fase 2: Pemilihan Drive (Drive List)
*   **Tampilan**: Daftar kotak (Card) untuk setiap partisi/drive yang terdeteksi.
*   **Isi Kotak**: 
    *   Ikon Drive (💽 untuk HDD, ⚡ untuk NVMe/SSD).
    *   Nama Drive (misal: `Local Disk (C:)` atau `/home`).
    *   Kapasitas (misal: `120 GB / 256 GB`).
    *   Label Medium (misal: `NVMe SSD`).
    *   **Tombol Panah Kanan (➡️)** di ujung kanan kotak.
*   **Aksi**: Jika pengguna mengklik kotak atau tombol panah kanan, aplikasi masuk ke Fase 3.

### Fase 3: Proses Pemindaian (Scanning)
*   **Tampilan**: Layar penuh menampilkan animasi progres.
*   **Elemen**:
    *   *Indeterminate Progress Bar* (garis berjalan dari kiri ke kanan) di bagian atas.
    *   Teks besar di tengah: *"Memindai C:\ ..."*
    *   Teks statistik real-time di bawahnya: *"1.250.000 file | 145.2 GB | 3.4 detik"*
    *   **Tombol "Batalkan" (🛑)**: Berwarna merah, berfungsi menghentikan scan secara instan.

### Fase 4: Penjelajah File (File Explorer)
*   **Tampilan**: Ini adalah layar utama aplikasi. Terdiri dari Toolbar (atas), Tabel Virtual (tengah), dan Status Bar (bawah).
*   **Tabel Virtual**: Menampilkan file/folder yang bisa di-scroll (hanya merender baris yang terlihat agar tidak *lag*).
*   **Aksi**: Pengguna bisa navigasi masuk/keluar folder, mencari file, dan menyortir ukuran.

---

## 2. Tata Letak (Layout) Layar Explorer

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ [1] HEADER / TOOLBAR                                                     │
│ [⬅️ Kembali] [C:\ > Users > Kusena ▼]  [🔍 Cari...] [Sort: Ukuran ▼]     │
├──────────────────────────────────────────────────────────────────────────┤
│ [2] TABEL VIRTUAL (Virtualized List)                                     │
│ ┌──────────────────────────────────────────────────────────────────────┐ │
│ │ Ikon │ Nama File / Folder       │ Blok  │ Asli  │ Item │ Aksi        │ │
│ ├──────────────────────────────────────────────────────────────────────┤ │
│ │ 📂   │ node_modules            │ 1.2GB │ 1.4GB │ 45k  │ [🗑️]        │ │
│ │ 📄   │ setup.exe               │ 800MB │ 800MB │  1   │ [🗑️]        │ │
│ │ 📄   │ video.mp4               │ 450MB │ 450MB │  1   │ [🗑️]        │ │
│ │ 📁   │ Dokumen                 │ 120MB │ 125MB │ 120  │ [🗑️]        │ │
│ └──────────────────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────────┤
│ [3] STATUS BAR                                                           │
│ Selesai | 1.2jt file | Total: 256 GB | Waktu: 5.2s | Medium: NVMe       │
└──────────────────────────────────────────────────────────────────────────┘
```

### Detail Komponen:
*   **Breadcrumb (Navigasi)**: Jika pengguna menekan folder, path akan memanjang (`C:\ > Users > Downloads`). Tombol ⬅️ (Kembali) untuk naik satu level.
*   **Kolom Ukuran**: 
    *   **Blok**: Ukuran fisik di disk (memperhitungkan *cluster/sparse file*).
    *   **Asli**: Ukuran logis byte file.
    *   **Item**: Jumlah file di dalamnya (jika folder).
*   **Ikon Hapus (🗑️)**: Hanya terlihat saat kursor mouse berada di atas baris tersebut (*hover*). Ini mencegah tampilan terlalu ramai.

---

## 3. Alur Penghapusan (Deletion Flow)

Karena penghapusan adalah aksi destruktif, UX harus sangat jelas. Terdapat **Modal Konfirmasi** yang muncul saat ikon 🗑️ diklik.

### Tampilan Modal (Overlay di tengah layar):
```text
┌──────────────────────────────────────────────┐
│  Pilih Metode Penghapusan                    │
│                                              │
│  Target: node_modules (1.2 GB)               │
│                                              │
│  [ Pindahkan ke Trash ]  <-- Tombol Kuning   │
│  (Aman, bisa dikembalikan)                   │
│                                              │
│  [ Hapus Permanen ]      <-- Tombol Merah    │
│  (Data hilang selamanya, tanpa rescan)       │
│                                              │
│  [ Batal ]               <-- Tombol Abu-abu  │
└──────────────────────────────────────────────┘
```

**UX Penghapusan:**
1. Jika **Pindahkan ke Trash** dipilih, UI mengirim perintah. File dipindah ke Recycle Bin (Windows) atau XDG Trash (Linux).
2. Jika **Hapus Permanen** dipilih, file dihapus dari disk (`remove_file`/`remove_dir_all`).
3. **Efek di UI**: Saat Engine selesai menghapus, UI **tidak melakukan scan ulang**. UI hanya memanggil `model.terapkan_penghapusan()` dan baris file tersebut langsung memudar dan menghilang. Angka total ukuran di folder induk langsung berkurang seketika (*Reactive Update*).

---

## 4. Arsitektur Iced (State, Message, Update)

### A. State (Data UI)
```rust
pub enum LayarAplikasi {
    Loading,
    PilihDrive(Vec<InformasiDrive>),
    Memindai { progres: f32, statistik: String },
    Explorer { path: Vec<IdEntri> },
}

pub struct Aplikasi {
    pub layar: LayarAplikasi,
    pub model_pohon: Option<Arc<PohonSistemBerkas>>,
    pub kata_kunci: String,
    pub mode_sortir: SortMode,
    pub modal_hapus: Option<IdEntri>, // Menyimpan ID file yang akan dihapus
}
```

### B. Message (Aksi)
```rust
#[derive(Debug, Clone)]
pub enum Pesan {
    // Navigasi Awal
    DriveDitemukan(Vec<InformasiDrive>),
    PilihDrive(InformasiDrive),
    BatalkanScan,
    
    // Proses Scan
    ProgresScanTick(f32, String),
    ScanSelesai(HasilScan),
    
    // Navigasi Explorer
    BukaFolder(IdEntri),
    KembaliKeInduk,
    KetikPencarian(String),
    UbahSortir(SortMode),
    
    // Aksi Hapus
    BukaModalHapus(IdEntri),
    TutupModalHapus,
    KonfirmasiHapusTrash,
    KonfirmasiHapusPermanen,
    HapusSelesai(Vec<IdEntri>),
}
```

### C. View (Render)
Di dalam fungsi `view`, kita menggunakan `match app.layar` untuk merender layar yang tepat:

```rust
pub fn view(app: &Aplikasi) -> Element<Pesan> {
    match &app.layar {
        LayarAplikasi::Loading => render_loading(),
        LayarAplikasi::PilihDrive(daftar) => render_pilih_drive(daftar),
        LayarAplikasi::Memindai { progres, statistik } => render_scanning(progres, statistik),
        LayarAplikasi::Explorer { path } => {
            let utama = render_explorer(app);
            if app.modal_hapus.is_some() {
                // Tumpuk modal di atas explorer
                stack![utama, render_modal_hapus()].into()
            } else {
                utama
            }
        }
    }
}
```

---

## 5. Animasi & Interaksi (Iced Features)

1. **Spinner Loading**: Menggunakan `Subscription::run_with_id` yang memicu `Message::AnimasiTick` setiap 100ms. Ikon logo diputar berdasarkan state waktu.
2. **Progress Bar**: 
   * Saat scan ukuran belum diketahui, gunakan `ProgressBar` mode *indeterminate* (Iced mendukung ini via style kustom atau pergerakan offset).
3. **Hover State (Micro-interaction)**: 
   * Iced memiliki `mouse_area` widget. Kita membungkus baris tabel dengan `mouse_area`, dan saat `on_enter`, kita set state `hovered_row = Some(id)`. Tombol hapus (🗑️) hanya dirender jika baris tersebut sedang di-hover.
4. **Transisi Halus**: Setelah penghapusan, baris file tidak langsung hilang, melainkan memudar (fade out) selama 200ms (opsional, bisa dilakukan dengan menambahkan state `baris_memudar` sebelum pembaruan model).

---

## 6. Struktur Folder UI (Implementasi)

```text
crates/ui/
├── src/
│   ├── lib.rs
│   ├── aplikasi.rs      # Implementasi `iced::Application` (State & Update)
│   ├── pesan.rs         # Enum Pesan
│   ├── tema.rs          # Skema warna (Merah untuk hapus, Kuning untuk trash, dll)
│   └── widgets/
│       ├── layar_loading.rs   # Tampilan awal (Spinner)
│       ├── layar_pilih_drive.rs # Kartu drive dengan tombol panah
│       ├── layar_scanning.rs  # Progress bar dan teks statistik
│       ├── tabel_virtual.rs   # Inti UI: Custom Tree/Table View
│       └── modal_hapus.rs     # Overlay konfirmasi Trash / Permanen
```