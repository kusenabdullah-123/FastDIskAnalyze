# Planning Arsitektur: fast-disk-analyze

**Versi:** 1.1 (revisi: layout POD valid, nama berbasis byte, HAL Windows/Linux dipertajam, CLI headless, alur delete/delta)
**Target:** Aplikasi desktop GUI (Iced) pemindai disk lintas platform (Windows & Linux) berkinerja tinggi.
**Filosofi:** *"Tanya medan dulu, catat datar, rakit belakangan; peta tinggal di perpustakaan, bukan di layar."*

---

## 1. Tujuan & Prinsip Desain

### 1.1 Tujuan Non-Negosiasi
1. **I/O-aware**: Jumlah worker thread dan kedalaman antrean **wajib** disesuaikan dengan medium (HDD/SSD/NVMe/Network).
2. **Cancellation nyata**: Pembatalan menghentikan pembacaan disk dalam ≤ 1 detik.
3. **Backpressure**: Tidak ada channel tanpa batas; memori memiliki plafon eksplisit.
4. **Akurasi lintas OS**: Disk usage dan hardlink dedup **benar** di Windows maupun Linux (lihat bagian 6.4 untuk trade-off Windows).
5. **Skalabilitas**: Mampu memindai ≥ 10 juta file tanpa OOM pada mesin 16 GB RAM (dengan mode *Summarized* jika perlu).
6. **Zero unsafe untuk navigasi**: Semua akses pohon menggunakan indeks/arena, bukan pointer mentah.
7. **Penamaan Domain**: Menggunakan Bahasa Indonesia untuk entitas inti (Entri, Gudang, Rekaman) agar *codebase* mudah dipahami.
8. **Nama berkas adalah byte, bukan teks**: Linux mengizinkan byte sembarang, Windows mengizinkan UTF-16 tak valid. Tidak boleh ada `unwrap()` pada konversi nama; tidak boleh panic karena nama aneh.
9. **Terukur sejak awal**: Ada CLI headless dan baseline benchmark sebelum GUI dibuat, sehingga semua klaim performa dapat diverifikasi.

### 1.2 Prinsip Arsitektur
- **Pemisahan lapisan tegas**: Presentation → Domain → Engine → Platform (HAL).
- **Arah data satu arah**: Perintah turun, data/pesan naik. Tidak ada lompatan lapisan.
- **Platform-specific code hanya di HAL**: Satu-satunya tempat `#[cfg(windows)]` / `#[cfg(unix)]` muncul.
- **POD (Plain Old Data) di hot path**: `RekamanEntri` harus `Copy`/`Pod` agar merge = `memcpy`. Field enum **dilarang** di struct POD; gunakan integer + konversi (`TryFrom`).
- **Layout dibuktikan, bukan diasumsikan**: setiap struct POD punya `const assert!(size_of == N)` dan test `bytemuck`.
- **Rekursi dilarang di jalur besar**: Agregasi, drop, dan sorting harus iteratif.
- **Data sementara tidak masuk record permanen**: identitas hardlink `(dev, ino)` hanya hidup di engine selama scan, lalu di-drop.

---

## 2. Diagram Lapisan

```text
┌────────────────────────────────────────────────────────────────────┐
│  PRESENTATION LAYER (GUI Iced)                                     │
│  Framework: Iced                                                   │
│  Tanggung jawab: render, input, ViewState. TIDAK pegang model.     │
└──────────────────────────────┬─────────────────────────────────────┘
                               │ Query API (pull) ─ Event (push)
┌──────────────────────────────▼─────────────────────────────────────┐
│  DOMAIN LAYER (Model Pohon)                                        │
│  Tanggung jawab: GudangEntri final, view cache, error log.         │
│  TIDAK tahu GUI, TIDAK melakukan syscall.                          │
└──────────────────────────────┬─────────────────────────────────────┘
                               │ HasilScan / apply_delta
┌──────────────────────────────▼─────────────────────────────────────┐
│  ENGINE LAYER                                                      │
│  PengendaliScan │ Pemindai │ Perakit │ Penghapus │ PengawasMemori  │
│  TIDAK tahu GUI. Berkomunikasi via bounded channel.                │
└──────────────────────────────┬─────────────────────────────────────┘
                               │ trait call (HAL)
┌──────────────────────────────▼─────────────────────────────────────┐
│  PLATFORM LAYER (HAL)                                              │
│  StorageProbe │ DirIterator │ FsOps │ IoBackend                    │
│  Satu-satunya lapisan dengan #[cfg(windows)] / #[cfg(unix)].       │
└────────────────────────────────────────────────────────────────────┘
```

Consumer lapisan Presentation ada dua: `ui` (GUI Iced) dan `cli` (headless). Keduanya memakai Query API dan kontrak Engine yang sama, sehingga `core` terbukti tidak bergantung pada GUI.

---

## 3. Struktur Modul (Workspace Cargo)

```text
fast-disk-analyze/
├── Cargo.toml                    # [workspace]
├── crates/
│   ├── core/                     # Domain + Engine (tanpa platform)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ids.rs            # IdEntri, IdNama, INDUK_AKAR
│   │   │   ├── entri.rs          # RekamanEntri, JenisEntri, Penanda
│   │   │   ├── gudang.rs         # GudangEntri, GudangNama
│   │   │   ├── model.rs          # PohonSistemBerkas + Query API
│   │   │   ├── view.rs           # ViewCache, SortMode, FilterCache
│   │   │   ├── engine/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pengendali.rs # PengendaliScan, cancel
│   │   │   │   ├── penjadwal.rs  # MediumKind -> Kebijakan
│   │   │   │   ├── pemindai.rs   # traversal loop
│   │   │   │   ├── perakit.rs    # merge + attach + aggregate + dedup hardlink
│   │   │   │   ├── penghapus.rs  # deletion + delta
│   │   │   │   └── pengawas.rs   # memory pressure
│   │   │   └── error.rs
│   │   └── Cargo.toml
│   │
│   ├── platform/                 # HAL (Windows + Linux)
│   │   ├── src/
│   │   │   ├── lib.rs            # re-export traits
│   │   │   ├── traits.rs         # StorageProbe, DirIterator, FsOps, IoBackend
│   │   │   ├── windows/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── probe.rs      # DeviceIoControl, GetDriveTypeW
│   │   │   │   ├── iterator.rs   # NtQueryDirectoryFile (handle direktori), fallback FindFirstFileEx
│   │   │   │   ├── fsops.rs      # AllocationSize, hardlink, verbatim path, trash
│   │   │   │   └── iocp.rs       # IOCP binding
│   │   │   └── linux/
│   │   │       ├── mod.rs
│   │   │       ├── probe.rs      # /sys/block, /proc/mounts (unescape)
│   │   │       ├── iterator.rs   # getdents64 + statx (flag minimal)
│   │   │       ├── fsops.rs      # statx blocks, hardlink (dev,ino), trash
│   │   │       └── io_uring.rs   # io_uring binding
│   │   └── Cargo.toml
│   │
│   ├── ui/                       # Presentation GUI (Iced)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── view_state.rs     # ViewState (tipis)
│   │   │   ├── app.rs            # event loop, input routing
│   │   │   └── widgets/          # tree view, size bar, status, modal
│   │   └── Cargo.toml
│   │
│   └── cli/                      # Presentation headless (WAJIB sejak M0)
│       ├── src/main.rs           # scan, bench, dump statistik, uji memori
│       └── Cargo.toml
└── xtask/                        # build automation, benchmark harness, generator pohon sintetis
```

---

## 4. Tipe Data Kunci

### 4.1 ID & Konstanta

```rust
// ids.rs
use bytemuck::{Pod, Zeroable};

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Pod, Zeroable)]
pub struct IdEntri(pub u32);

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Pod, Zeroable)]
pub struct IdNama(pub u32);

/// Penanda bahwa entri tidak memiliki induk (adalah Root).
/// Disimpan sebagai u32 mentah di RekamanEntri.induk (bukan enum) agar tetap Pod.
pub const INDUK_AKAR: u32 = u32::MAX;
```

**Batas:** `u32` membatasi ±4,29 miliar entri/nama; jauh di atas target 10 juta. Pengawas memori (bagian 7) menolak melebihi `u32::MAX - 1`.

### 4.2 Rekaman Entri (POD)

```rust
// entri.rs
use bytemuck::{Pod, Zeroable};

/// Nilai jenis disimpan sebagai u8 di RekamanEntri.jenis (enum tidak boleh Pod).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum JenisEntri {
    Berkas = 0,
    Direktori = 1,
    Tautan = 2,   // symlink / junction / reparse point: dicatat, TIDAK di-traverse
    Lainnya = 3,  // device, socket, fifo, dsb.
}

impl TryFrom<u8> for JenisEntri {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, ()> {
        match v { 0 => Ok(Self::Berkas), 1 => Ok(Self::Direktori),
                  2 => Ok(Self::Tautan), 3 => Ok(Self::Lainnya), _ => Err(()) }
    }
}

/// Bit flag untuk RekamanEntri.penanda
pub mod penanda {
    pub const NAMA_BUKAN_UTF8: u8   = 1 << 0; // nama dikonversi lossy saat tampil
    pub const ADA_ERROR: u8         = 1 << 1; // lihat ErrorLog
    pub const TAUTAN_GANDA: u8      = 1 << 2; // hardlink kedua+; ukuran tidak dihitung ulang
    pub const TIDAK_DITELUSURI: u8  = 1 << 3; // direktori dilewati (mount lain / reparse / cap memori)
    pub const DIRINGKAS: u8         = 1 << 4; // anak tidak disimpan (mode Summarized)
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct RekamanEntri {
    // Urutan field: u64 dulu, lalu u32, lalu u8/u16  =>  tanpa padding implisit.
    pub ukuran_asli: u64,    // 8  (apparent size / EndOfFile). Direktori: agregat.
    pub ukuran_blok: u64,    // 8  (alokasi nyata di disk).     Direktori: agregat.
    pub induk: u32,          // 4  (INDUK_AKAR jika root)
    pub id_nama: u32,        // 4  (IdNama.0)
    pub anak_awal: u32,      // 4  (indeks anak pertama; valid setelah perakitan)
    pub jumlah_anak: u32,    // 4
    pub jumlah_tautan: u32,  // 4  (nlink)
    pub jenis: u8,           // 1  (JenisEntri as u8)
    pub penanda: u8,         // 1  (bit flag)
    pub _padding: u16,       // 2  (eksplisit)
}
const _: () = assert!(core::mem::size_of::<RekamanEntri>() == 40);
const _: () = assert!(core::mem::align_of::<RekamanEntri>() == 8);
// Total: 40 byte. Target ukuran per entri ≤ 40 byte.

impl RekamanEntri {
    pub fn jenis(&self) -> JenisEntri {
        JenisEntri::try_from(self.jenis).unwrap_or(JenisEntri::Lainnya)
    }
    pub fn anak(&self) -> core::ops::Range<usize> {
        let a = self.anak_awal as usize;
        a..a + self.jumlah_anak as usize
    }
}
```

**Aturan:** Entri yang sudah di gudang **tidak menyimpan path**. Path direkonstruksi lewat rantai `induk`. Setelah perakitan, anak setiap direktori **kontigu** di `daftar_entri` (diurutkan berdasarkan `induk`), sehingga `children(id)` adalah slice `daftar_entri[anak()]` tanpa alokasi.

### 4.3 Gudang Memori

Nama disimpan sebagai **byte** (Linux: byte mentah; Windows: WTF-8, yaitu UTF-8 yang dapat memuat surrogate tunggal). Tidak ada `Box<str>` per nama dan tidak ada duplikasi teks.

```rust
// gudang.rs
use crate::ids::{IdEntri, IdNama};
use crate::entri::RekamanEntri;
use hashbrown::HashTable;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

fn hash_bytes(b: &[u8]) -> u64 {
    let mut h = FxHasher::default();
    b.hash(&mut h);
    h.finish()
}

pub struct GudangNama {
    pub teks_mentah: Vec<u8>,
    pub daftar_posisi: Vec<(u32, u32)>,   // (awal, panjang) per IdNama
    indeks_nama: HashTable<IdNama>,       // hanya menyimpan IdNama; kunci dibaca dari teks_mentah
}

impl GudangNama {
    pub fn baru() -> Self {
        Self { teks_mentah: Vec::new(), daftar_posisi: Vec::new(), indeks_nama: HashTable::new() }
    }

    pub fn simpan_nama(&mut self, nama: &[u8]) -> IdNama {
        let h = hash_bytes(nama);
        let (teks, posisi) = (&self.teks_mentah, &self.daftar_posisi);
        if let Some(&id) = self.indeks_nama.find(h, |&id| {
            let (a, p) = posisi[id.0 as usize];
            &teks[a as usize..(a + p) as usize] == nama
        }) {
            return id;
        }
        let id_baru = IdNama(self.daftar_posisi.len() as u32);
        let awal = self.teks_mentah.len() as u32;
        self.teks_mentah.extend_from_slice(nama);
        self.daftar_posisi.push((awal, nama.len() as u32));
        let (teks, posisi) = (&self.teks_mentah, &self.daftar_posisi);
        self.indeks_nama.insert_unique(h, id_baru, |&id| {
            let (a, p) = posisi[id.0 as usize];
            hash_bytes(&teks[a as usize..(a + p) as usize])
        });
        id_baru
    }

    pub fn ambil_nama(&self, id: IdNama) -> &[u8] {
        let (awal, panjang) = self.daftar_posisi[id.0 as usize];
        &self.teks_mentah[awal as usize..(awal + panjang) as usize]
    }

    /// Untuk tampilan GUI: tidak pernah panic. `Cow::Borrowed` jika UTF-8 valid.
    pub fn nama_tampil(&self, id: IdNama) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(self.ambil_nama(id))
    }
}

pub struct GudangEntri {
    pub daftar_entri: Vec<RekamanEntri>,
    pub nama: GudangNama,
}

impl GudangEntri {
    pub fn baru() -> Self {
        Self { daftar_entri: Vec::with_capacity(100_000), nama: GudangNama::baru() }
    }
    pub fn tambah_entri(&mut self, rekaman: RekamanEntri) -> IdEntri {
        let id = IdEntri(self.daftar_entri.len() as u32);
        self.daftar_entri.push(rekaman);
        id
    }
}
```

**Gudang per worker:** setiap worker memegang `GudangEntri` lokal (tanpa lock). Pada perakitan, gudang digabung: `daftar_entri` di-`extend` (memcpy), `IdNama` di-*remap* lewat tabel `Vec<u32>` per worker, `induk` lokal di-*offset*.

### 4.4 Identitas Berkas (sementara, hanya untuk dedup hardlink)

```rust
// Tidak disimpan di RekamanEntri. Hidup di worker/engine selama scan.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct IdentitasBerkas {
    pub volume: u64,   // Linux: st_dev       | Windows: VolumeSerialNumber
    pub berkas: u128,  // Linux: st_ino       | Windows: FileId (64/128 bit)
}

// Hanya entri dengan jumlah_tautan > 1 yang dicatat:
//   Vec<(IdEntri, IdentitasBerkas)>   (jarang; pada umumnya << 1% entri)
// Perakit mengurutkan/menaruhnya di FxHashSet; kemunculan kedua+ diberi
// penanda::TAUTAN_GANDA dan ukurannya tidak ikut diagregasi. Setelah agregasi: drop.
```

### 4.5 Model Query API

```rust
// model.rs
pub struct PohonSistemBerkas {
    pub gudang: GudangEntri,
    pub root: IdEntri,
    pub errors: ErrorLog,
    view_cache: FxHashMap<(IdEntri, SortKey), Arc<[IdEntri]>>,
    filter_cache: LruCache<(IdEntri, String), Arc<[IdEntri]>>,
}

impl PohonSistemBerkas {
    pub fn children(&self, id: IdEntri) -> &[RekamanEntri];       // slice kontigu, O(1)
    pub fn sorted(&mut self, id: IdEntri, key: SortKey) -> Arc<[IdEntri]>;
    pub fn filtered(&mut self, id: IdEntri, q: &str) -> Arc<[IdEntri]>; // tanpa String per entri
    pub fn nama(&self, id: IdEntri) -> Cow<'_, str>;              // lossy, tidak panic
    pub fn display_path(&self, id: IdEntri) -> DisplayPath<'_>;   // lazy
    pub fn fs_path_verbatim(&self, id: IdEntri) -> VerbatimPath;  // byte asli / \\?\ di Windows
    pub fn effective_size(&self, id: IdEntri, mode: SizeMode) -> u64;
    pub fn errors_for(&self, id: IdEntri) -> &[ScanError];
}
```

`ErrorLog` disimpan sebagai `Vec<(IdEntri, ScanError)>` terurut berdasarkan `IdEntri` (pencarian biner di `errors_for`), bukan di dalam record, agar record tetap 40 byte.

---

## 5. Kontrak Engine ↔ UI

```rust
// pengendali.rs
pub struct PermintaanScan {
    pub root_path: PathBuf,
    pub medium: MediumKind,        // hasil probe (atau Auto)
    pub mode: SizeMode,
    pub cap_policy: CapPolicy,     // Soft / Hard / None
    pub akurasi_tautan: AkurasiTautan, // Cepat | Akurat (lihat bagian 6.4)
    pub lintas_mount: bool,        // default false
}

pub enum ProgressEvent {
    Started { total_hint: Option<u64> },
    Tick { items: u64, bytes: u64, errors: u64, elapsed_ms: u64 },
    MemoryPressure { used_nodes: usize, cap: usize },
    Finished(HasilScan),
    Cancelled { items: u64 },
    Failed { error: ScanError },
}

pub enum HasilScan {
    Complete(HasilLengkap),
    Incomplete { result: HasilLengkap, reason: IncompleteReason },
}

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

// penghapus.rs
pub enum ModeHapus { Permanen, Sampah }

pub struct HasilPenghapusan {
    pub terhapus: Vec<IdEntri>,
    pub gagal: Vec<(IdEntri, FsError)>,
    pub dibebaskan_asli: u64,
    pub dibebaskan_blok: u64,      // nlink-aware: hardlink hanya dihitung jika tautan terakhir
}
```

**Kanal:** `crossbeam_channel::bounded(64)`. `Tick` di-*coalesce*: jika penuh, `try_send` tick terbaru dan buang yang lama. Event terminal (`Finished`/`Cancelled`/`Failed`) memakai `send` blocking dan tidak boleh dibuang.

**Pembatalan:** `CancellationToken` (`Arc<AtomicBool>`) dicek tiap 512 entri **dan** sebelum setiap syscall batch/`submit`. Pada `IoBackend`, request yang sudah di-submit dibatalkan (`CancelIoEx` / `IORING_OP_ASYNC_CANCEL`) atau ditunggu selesai dengan batas waktu pendek.

---

## 6. Kontrak HAL (Platform Layer)

### 6.1 Storage Probe

```rust
pub trait StorageProbe: Send + Sync {
    fn probe(&self, path: &Path) -> MediumKind;
}

pub enum MediumKind {
    Hdd,
    SataSsd,
    NvmeSsd,
    Network,
    Removable,
    Unknown,
}
```

- **Windows:** `DeviceIoControl(IOCTL_STORAGE_QUERY_PROPERTY)` → `BusType` + `StorageDeviceSeekPenaltyProperty` (HDD vs SSD); `GetDriveTypeW` untuk remote/removable.
- **Linux:** baca `/sys/block/<dev>/queue/rotational`; deteksi `nvme*` dari nama device; parse `/proc/mounts` dengan unescape (`\040` dst). Untuk device-mapper/LVM/RAID/btrfs, telusuri `slaves/` ke device fisik; jika tidak jelas → `Unknown`.
- **Probe async dengan timeout** (mis. 2 s). Jika gagal/timeout → `Unknown` (kebijakan konservatif), scan tetap berjalan.

### 6.2 Directory Iterator

```rust
pub trait DirIterator: Send {
    fn walk(&mut self, root: &Path, token: &CancellationToken,
            emit: &mut dyn FnMut(RawEntry)) -> Result<(), ScanError>;
}

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
```

**Linux:**
- `getdents64` dengan buffer besar (≥ 64 KB); gunakan `d_type` untuk jenis awal.
- `statx` per entri dengan `AT_STATX_DONT_SYNC | AT_SYMLINK_NOFOLLOW | AT_NO_AUTOMOUNT` dan mask minimal `STATX_TYPE | STATX_SIZE | STATX_BLOCKS | STATX_NLINK | STATX_INO`. Panggil `statx` relatif ke fd direktori (`dirfd`), bukan path penuh.
- `ukuran_blok = stx_blocks * 512`; `identitas = (stx_dev_major/minor, stx_ino)` bila `stx_nlink > 1`.
- Kernel ≥ 5.6: `statx` di-*batch* via `IORING_OP_STATX` (lihat bagian 6.3). Selain itu: panggilan sinkron di worker.

**Windows:**
- Enumerasi lewat **handle direktori** dengan `NtQueryDirectoryFile` / `GetFileInformationByHandleEx` kelas `FileIdBothDirectoryInfo` / `FileIdExtdDirectoryInfo` (buffer besar). Satu panggilan mengembalikan banyak entri dengan `EndOfFile`, `AllocationSize`, atribut, dan `FileId` — **tanpa** `CreateFileW` per file.
- Fallback: `FindFirstFileExW(FindExInfoBasic, FIND_FIRST_EX_LARGE_FETCH)`; `ukuran_blok` perkiraan dibulatkan ke cluster (flag `perkiraan`).
- `AllocationSize` sudah mencerminkan file compressed/sparse; `GetCompressedFileSizeW` hanya dipakai sebagai alat verifikasi di test (bagian 11), bukan di hot path.
- Nama UTF-16 → WTF-8 (tanpa kehilangan surrogate tunggal) agar round-trip ke path asli aman.

**Aturan traversal (keselamatan loop):**
- Linux: `DT_LNK` → `JenisEntri::Tautan`, tidak di-traverse. Direktori dengan `st_dev` berbeda dari root dilewati kecuali `lintas_mount = true` (diberi `TIDAK_DITELUSURI`).
- Windows: atribut `FILE_ATTRIBUTE_REPARSE_POINT` (symlink, junction, mount point, OneDrive placeholder) → `Tautan`, tidak di-traverse; placeholder cloud tidak boleh dipicu *hydrate* (jangan membuka isi).
- Semua direktori yang gagal dibuka mencatat `ScanError` ke `ErrorLog`, memberi `ADA_ERROR`, dan scan lanjut.

### 6.3 I/O Backend (Async)

```rust
pub trait IoBackend: Send + Sync {
    type Req: Send;
    fn submit_batch(&self, reqs: &mut [Self::Req]);
    fn poll_completions(&self, max: usize) -> Vec<Completion>;
    fn cancel_all(&self);
}
```

- **Windows:** IOCP + Overlapped. Dipakai untuk pembacaan direktori/metadata bila terbukti menguntungkan di benchmark (M1 spike); jika tidak, jalur sinkron multi-thread tetap menjadi default.
- **Linux:** `io_uring` (`IORING_OP_STATX`, batched submit, buffer terdaftar).
- **Fallback (Linux < 5.6 / `io_uring` diblokir seccomp/container):** thread-pool blocking. Deteksi di runtime lewat `io_uring_setup` + probe opcode; hasilnya di-*cache*.
- **Prinsip:** `IoBackend` adalah optimasi (M7). Jalur sinkron (bagian 6.2) harus sudah benar dan teruji lebih dulu.

### 6.4 Hardlink & Akurasi Ukuran di Windows

Direktori-info NTFS **tidak** menyertakan jumlah tautan (`NumberOfLinks`). Sehingga:

| `AkurasiTautan` | Perilaku | Biaya |
|:---|:---|:---|
| `Cepat` (default) | Tidak ada dedup hardlink di Windows; ukuran sesuai `AllocationSize`. Linux tetap akurat (`nlink` dari `statx`). | Tanpa handle per file |
| `Akurat` | Buka handle `FILE_READ_ATTRIBUTES` (tanpa akses data) untuk berkas, ambil `FILE_STANDARD_INFO.NumberOfLinks` + `FileId`, lalu dedup. | Lebih lambat; ditampilkan di UI sebagai opsi |
| Auto-sempit | `Cepat`, tetapi `Akurat` otomatis untuk subtree yang dikenal padat hardlink (mis. `WinSxS`). | Terbatas |

UI menandai hasil Windows mode `Cepat` dengan catatan "hardlink tidak didedup". Validasi biaya dan perilaku nyata dilakukan di **M1 spike Windows** sebelum diputuskan final.

### 6.5 FsOps

```rust
pub trait FsOps: Send + Sync {
    fn delete_permanent(&self, path: &VerbatimPath) -> Result<(), FsError>;
    fn move_to_trash(&self, path: &VerbatimPath) -> Result<TrashOutcome, FsError>;
    fn path_is_cross_fs(&self, path: &Path, trash_root: &Path) -> bool;
    fn open_default(&self, path: &Path) -> Result<(), FsError>;
    fn reveal_in_file_manager(&self, path: &Path) -> Result<(), FsError>;
}
```

- `VerbatimPath` menyimpan **byte/UTF-16 asli**; di Windows selalu berawalan `\\?\` (path > 260 karakter).
- `move_to_trash` Windows (`IFileOperation`) dijalankan di thread terpisah; Linux mengikuti spesifikasi XDG Trash. Jika `path_is_cross_fs` = true, UI menampilkan peringatan bahwa trash berarti *copy + delete* (lambat) sebelum eksekusi.

---

## 7. Kebijakan Scheduler & Pengawas Memori

### 7.1 Medium → Thread

| Medium | Worker | Queue Depth | Timeout | Catatan |
|:---|:---:|:---:|:---:|:---|
| HDD | 1 | 1 (sekuensial) | — | Jangan paralel; boleh `io_uring` untuk isi jeda |
| SataSsd | 2–4 | 8 | — | Manfaatkan NCQ |
| NvmeSsd | `min(physical_cores, 16)` | 32 | — | Jangan pakai logical core |
| Network | 1–2 | 2 | 5 s per op | Backoff eksponensial |
| Removable | 1–2 | 2 | 10 s | Sama seperti network |
| Unknown | 2 | 2 | — | Konservatif |

`PenjadwalScan::policy(medium) -> Kebijakan { threads, queue_depth, timeout }`. Angka di atas adalah **titik awal**; M1/M7 wajib menyetelnya dari hasil benchmark nyata per medium.

### 7.2 Pengawas Memori

- Dua ambang: **soft cap** (mulai mode `Summarized`: direktori yang sangat dalam diringkas, anak tidak disimpan, `DIRINGKAS`) dan **hard cap** (hentikan scan → `HasilScan::Incomplete` dengan hasil parsial yang konsisten).
- Pemeriksaan tiap kelipatan 512 entri; tidak memakai alokasi saat pemeriksaan.
- Hasil `Incomplete` tetap valid sebagai pohon (agregat dihitung dari yang sudah terbaca) dan ditandai jelas di UI.

---

## 8. Alur Runtime (End-to-End)

### 8.1 Scan Penuh

```text
[User pilih drive]
     │
     ▼
[GUI/CLI] → PengendaliScan::mulai(req)
     │
     ▼
[Engine] → StorageProbe.probe(path)  →  MediumKind        (async + timeout; gagal → Unknown)
     │
     ▼
[Engine] → PenjadwalScan::policy(medium)  →  Kebijakan
     │
     ▼
[Engine] spawn N workers (pool tunggal, reuse)
     │
     ├── Worker loop:
     │     while let Some(task) = work_steal() {
     │         token.check()?;
     │         for entry in DirIterator.walk(task, token) {
     │             local_gudang.push(RekamanEntri { ... });
     │             if jumlah_tautan > 1 { local_tautan.push((id, identitas)); }
     │             if local_gudang.len() & 511 == 0 { tick_coalescer.maybe_emit(); pengawas.check()?; }
     │         }
     │     }
     │
     ▼
[Engine/Perakit]
     1. merge gudang worker (memcpy entri + remap IdNama + offset induk)
     2. sort by induk (stabil) → anak kontigu → isi anak_awal/jumlah_anak
     3. dedup hardlink: pass atas `tautan` (FxHashSet) → tandai TAUTAN_GANDA; set di-drop
     4. agregasi iteratif bottom-up (urutan terbalik kedalaman; tanpa rekursi)
     │
     ▼
[Engine] → ProgressEvent::Finished(HasilScan::Complete(result))
     │
     ▼
[Domain] PohonSistemBerkas::install(result)   // swap gudang, invalidate cache
     │
     ▼
[GUI] query model.sorted(root)   // O(K log K) sekali, lalu cache
```

### 8.2 Rescan Subtree (Delta)

```text
[GUI] → PengendaliScan::scan_subtree(model, id)
     │
     ▼
[Engine] scan subtree → gudang_subtree baru (induk lokal)
     │
     ▼
[Domain] model.terapkan_subtree(id, gudang_subtree)
         - tambahkan entri baru ke ekor daftar_entri (append-only);
           anak lama ditandai yatim (tidak dirujuk lagi)
         - perbarui anak_awal/jumlah_anak pada `id`
         - delta agregat = (baru - lama) dirambatkan ke rantai leluhur: O(D)
     │
     ▼
[Domain] invalidasi view_cache hanya untuk `id` dan leluhurnya
     │
     ▼
[Domain] jika yatim > 25% dari daftar_entri → kompaksi (rebuild kontigu) di thread latar
```

Karena anak harus kontigu, subtree baru ditaruh di ekor dan `anak_awal` induk diarahkan ke sana; ini menjaga rescan O(subtree + D) tanpa menggeser seluruh array.

### 8.3 Penghapusan

```text
[GUI] → Penghapus::hapus(ids, mode)
     │
     ▼
[Platform] FsOps::delete_permanent / move_to_trash   (thread terpisah, progress via channel)
     │
     ▼
[Engine] HasilPenghapusan { terhapus, gagal, dibebaskan_asli, dibebaskan_blok }
     │
     ▼
[Domain] model.terapkan_penghapusan(&terhapus)
         - nlink-aware: kurangi ukuran leluhur hanya jika tautan terakhir (jumlah_tautan == 1
           atau semua tautan lain juga dalam daftar hapus)
         - penghapusan parsial: hanya `terhapus` yang dikurangi; `gagal` tetap + dicatat ke ErrorLog
     │
     ▼
[GUI] invalidasi cache leluhur terdampak (tanpa rescan)
```

### 8.4 Validasi `ViewState`
`ViewState` hanya menyimpan `IdEntri` dan parameter tampilan. Setiap kali model di-*install* atau diubah (delta/hapus), `nav_stack` divalidasi: id yang tidak lagi valid atau yatim dipotong ke leluhur terdekat yang valid (*clamp*).

---

## 9. Milestone & Kriteria Penerimaan

### M0 — Fondasi Workspace + Baseline (1 minggu)
- Workspace Cargo dengan 4 crate (`core`, `platform`, `ui`, `cli`) + `xtask`.
- `core` compile dengan `RekamanEntri` POD (40 byte, assert compile-time), `GudangEntri`, `GudangNama` (byte, tanpa `Box<str>`) + unit test roundtrip, termasuk nama non-UTF-8.
- `platform` trait definitions.
- `xtask`: generator pohon sintetis (1 juta / 10 juta entri) dan skrip ukur baseline **FastDiskScan lama** (waktu & RAM) di mesin acuan.
- **Acceptance:** `cargo build --workspace` di Windows & Linux; `bytemuck::Pod` valid untuk `RekamanEntri`; angka baseline tercatat di `docs/baseline.md` (menjadi acuan metrik bagian 11).

### M1 — HAL Linux & Windows (2 minggu)
- `StorageProbe` di kedua OS + test unit (mock filesystem).
- `DirIterator` di kedua OS dengan data lengkap per entri; aturan loop/symlink/reparse/mount.
- **Spike Windows:** bandingkan `NtQueryDirectoryFile` vs `FindFirstFileEx` vs handle-per-file; ukur biaya mode `Akurat` (bagian 6.4) dan putuskan default.
- **Spike Iced:** prototipe tree view virtual 100 ribu baris untuk memvalidasi estimasi M4 lebih awal.
- **Acceptance:** probe mengembalikan `MediumKind` benar untuk NVMe/SSD/HDD; iterator membaca 100 ribu file tanpa error; direktori dengan symlink loop dan nama non-UTF-8 tidak panic; hasil spike terdokumentasi.

### M2 — Engine Traversal + Assembly (2 minggu)
- `PengendaliScan` end-to-end dengan channel bounded; `cli scan <path>` berfungsi.
- `CancellationToken` dicek tiap 512 entri dan sebelum syscall batch; test cancel < 1 detik.
- Perakit: merge, sort by `induk`, `anak_awal/jumlah_anak`, dedup hardlink, agregasi iteratif.
- **Acceptance:** scan 1 juta file selesai < 60 detik di NVMe; cancel berhenti ≤ 1 detik; agregat pohon sintetis cocok dengan nilai ekspektasi generator; `cargo test` hijau.

### M3 — Domain Model + Query API (1 minggu)
- `PohonSistemBerkas` dengan `view_cache` & `filter_cache`; `ErrorLog` terurut.
- Benchmark: 1000 render `sorted()` pada folder 100 ribu entri < 10 ms total.
- **Acceptance:** `filtered(q)` tidak mengalokasi `String` per entri; `display_path()` lazy; `nama()` tidak pernah panic.

### M4 — GUI Iced (3–4 minggu)
- Iced diintegrasikan dengan `ViewState` murni; widget tree view virtual (hasil spike M1).
- **Acceptance:** GUI tidak menyimpan `PohonSistemBerkas`; tidak ada referensi `RekamanEntri` di luar Domain; frame time < 16 ms untuk folder 100 ribu entri.
- Cadangan waktu 1 minggu jika spike M1 menunjukkan Iced perlu widget kustom besar.

### M5 — Deletion & Delta (2 minggu)
- `Penghapus` + `terapkan_penghapusan` nlink-aware; penghapusan parsial konsisten.
- Trash cross-fs detection + estimasi di UI.
- `scan_subtree` + `terapkan_subtree` (append-only + kompaksi) dengan delta ancestor chain.
- **Acceptance:** hapus 1000 file tidak memicu rescan; `dibebaskan_blok` cocok dengan `df` sebelum/sesudah (toleransi < 1%); clamp `nav_stack` teruji.

### M6 — Memory Governor & Skala (1 minggu)
- Soft/hard cap; status `Summarized`/`Incomplete` di UI dan CLI.
- Test 10 juta file sintetis dengan RAM cap 4 GB → no OOM; UI menampilkan `Incomplete`.
- **Acceptance:** `cargo test --release --features stress` hijau di CI 16 GB.

### M7 — Optimasi & Hardening (2 minggu)
- `io_uring` aktif di Linux (fallback otomatis); IOCP bila terbukti menguntungkan.
- Penyetelan tabel Medium → Thread berdasarkan benchmark.
- Fuzz test `DirIterator` (path invalid, nama non-UTF-8, symlink loop, permission).
- **Acceptance:** benchmark tiap medium mencapai ≥ 90% dari throughput referensi (`du`/`find` Linux, WizTree/`dir /s` Windows sebagai pembanding terukur, bukan teoretis).

**Total estimasi:** ~12–13 minggu (paralel 2 engineer).

---

## 10. Risiko & Mitigasi

| Risiko | Dampak | Mitigasi |
|:---|:---|:---|
| `io_uring` tidak tersedia / diblokir seccomp | Linux fallback | Trait `IoBackend`; deteksi runtime; jalur sinkron selalu benar lebih dulu |
| Iced belum menyediakan tree view virtual | Waktu UI membengkak | Spike di M1; buffer waktu M4; abstraksi `Widget` tipis agar Domain tidak tersentuh |
| Layout `RekamanEntri` melanggar `Pod` | Merge salah / compile error | `const assert!` ukuran+align, field enum dilarang, test `bytemuck` di CI |
| Nama berkas non-UTF-8 / surrogate tunggal | Panic atau path rusak | Nama disimpan sebagai byte/WTF-8; `nama_tampil` lossy; `VerbatimPath` memakai byte asli |
| Windows: hardlink tidak terlihat dari dir-info | Ukuran terlalu besar | Mode `Cepat`/`Akurat` eksplisit (bagian 6.4); ditandai di UI; diputuskan di spike M1 |
| Windows: handle per file lambat (AV/minifilter) | Scan 10–100× lebih lambat | Enumerasi via handle direktori; handle file hanya mode `Akurat` |
| Placeholder cloud (OneDrive) ter-hydrate | I/O jaringan tak diinginkan | Reparse point tidak dibuka/di-traverse |
| Trash Windows (`IFileOperation`) blocking | UI freeze | Thread terpisah; progress via channel |
| Path > 260 karakter Windows | Hapus gagal | `VerbatimPath` `\\?\`; test dengan path sintetis panjang |
| BTRFS subvolume / ZFS dataset / bind mount | Partisi hilang atau terhitung ganda | Deteksi tipe fs di `/proc/mounts`; dedup mount berdasar `st_dev`; tampilkan mount user-accessible |
| Rescan append-only menimbulkan sampah yatim | Memori membengkak | Hitung yatim; kompaksi di latar bila > 25% |
| Target performa tanpa baseline | Klaim tidak terverifikasi | Baseline di M0 (`docs/baseline.md`) sebelum angka bagian 11 dianggap final |
| Estimasi RAM 200 MB/1 juta file terlampaui | Gagal metrik memori | Anggaran: 40 MB record + nama (~30–60 MB) + overhead; ukur di M2, jika perlu optimasi nama (prefix sharing) |

---

## 11. Metrik Keberhasilan (Definition of Done)

Arsitektur baru dinyatakan selesai ketika:

1. **Performa:** Scan 1 juta file di NVMe ≥ 5× lebih cepat dari FastDiskScan lama (diukur terhadap baseline M0).
2. **Memori:** Scan 1 juta file dengan RAM ≤ 200 MB (vs > 2 GB di arsitektur lama; diukur RSS puncak lewat `cli`).
3. **Cancel:** Tombol batal menghentikan I/O dalam ≤ 1 detik (diverifikasi `strace`/`iotop`/Process Monitor).
4. **Akurasi Windows:** `disk_usage` untuk folder dengan file sparse/compressed cocok dengan agregat `GetCompressedFileSizeW` (toleransi < 0.5%); mode `Akurat` juga cocok untuk hardlink.
5. **Akurasi Linux:** `disk_usage` cocok dengan `statvfs` delta setelah `sync` (toleransi < 0.5%); hardlink tidak terhitung ganda.
6. **Skalabilitas:** Scan 10 juta file dengan RAM cap 4 GB menghasilkan `Incomplete`, bukan OOM.
7. **Ketahanan:** Nama non-UTF-8, symlink loop, junction, permission denied, dan mount lain tidak menyebabkan panic, hang, atau penghitungan ganda.
8. **Lintas OS:** Semua test M0–M7 lulus di Windows 11 dan Ubuntu 22.04+ (CI matrix).

---

## 12. Yang TIDAK Akan Dibangun (Out of Scope v1)

- Spill-to-disk (pohon tidak muat RAM → tulis ke SQLite).
- Pemindaian jaringan multi-host.
- Histori scan tersimpan.
- Deteksi duplikat berbasis konten (hash).
- Pemindaian file terenkripsi.
- Dukungan macOS (HAL dirancang agar dapat ditambahkan, tetapi bukan target v1).

Semua ini dapat ditambahkan di v2 tanpa mengubah kontrak antar-lapisan.

---

## 13. Referensi Silang Perbaikan (Dari Arsitektur Lama)

| Temuan Arsitektur Lama | Solusi di Arsitektur Baru |
|:---|:---|
| Tidak ada hardware probing | `StorageProbe` (HAL) per OS, async + timeout (bagian 6.1) |
| Cancellation tidak nyata | `CancellationToken` dicek tiap 512 entri + sebelum syscall batch (bagian 5) |
| Unbounded channel | Bounded + coalesced progress (bagian 5) |
| Pool per scan | Pool tunggal yang di-reconfigure (bagian 8.1) |
| Network tanpa batas | `TimeoutPolicy` + thread 1–2 (bagian 7.1) |
| Tanpa plafon memori | `PengawasMemori` soft/hard cap (bagian 7.2) |
| `PathBuf` per node | `IdNama` + `induk`, path on-demand (bagian 4.2) |
| Allocation storm | Gudang (Arena) + `RekamanEntri` POD + `GudangNama` tanpa `Box<str>` (bagian 4.3) |
| Key `PathBuf` di map | Sort by `induk`, tanpa map (bagian 8.1) |
| Rekursi drop/agregasi | Anak kontigu + agregasi iteratif (bagian 4.2, bagian 8.1) |
| Skalabilitas RAM | PengawasMemori + mode Summarized (bagian 7.2) |
| Disk usage Windows salah | `AllocationSize` dari dir-info (bagian 6.2) |
| Hardlink Windows tidak didedup | Mode `Akurat` + `IdentitasBerkas` sementara (bagian 4.4, bagian 6.4) |
| Long path gagal hapus | Dua representasi path (display vs verbatim) (bagian 6.5) |
| Startup blocking | Probe async + timeout (bagian 6.1) |
| Rescan O(N) | Delta ancestor chain O(D) (bagian 8.2) |
| `nav_stack` invalid | Validasi + clamp (bagian 8.4) |
| Error traversal hilang | `ErrorLog` terurut per `IdEntri` (bagian 4.5) |
| Akuntansi hardlink hapus | `nlink` aware (bagian 8.3) |
| Hapus parsial tidak konsisten | `HasilPenghapusan` eksplisit (bagian 5, bagian 8.3) |
| Trash copy+delete tersembunyi | Estimasi di UI (bagian 6.5) |
| Alokasi filter per frame | Cache view di Domain (bagian 4.5) |
| GUI memegang `root` | `ViewState` tipis (bagian 8.4) |
| *(baru)* Panic pada nama non-UTF-8 | Nama byte/WTF-8, `nama_tampil` lossy (bagian 4.3) |
| *(baru)* Symlink/junction loop | Aturan traversal HAL (bagian 6.2) |

---

## 14. Catatan Revisi v1.1

| # | Perubahan | Alasan |
|:---|:---|:---|
| 1 | Urutan field `RekamanEntri` ditata ulang + assert 40 byte | Padding implisit sebelumnya membuat `derive(Pod)` gagal dan ukuran menjadi 48 |
| 2 | `jenis` jadi `u8` + `TryFrom`, enum keluar dari struct POD | `bytemuck::Pod` menolak enum |
| 3 | Tambah `anak_awal`/`jumlah_anak` | Query `children()` O(1) tanpa alokasi |
| 4 | Nama = byte/WTF-8; `GudangNama` memakai `HashTable<IdNama>` | Hilangkan `Box<str>`, duplikasi teks, dan `unwrap()` panic |
| 5 | HAL Windows via handle direktori; mode hardlink `Cepat`/`Akurat` | Hindari `CreateFileW` per file; dir-info tak memuat nlink |
| 6 | Linux `statx` dengan flag & mask minimal, `dirfd`-relative | Kurangi biaya syscall |
| 7 | Aturan symlink/reparse/mount + `IdentitasBerkas` sementara | Cegah loop dan penghitungan ganda |
| 8 | Crate `cli` wajib sejak M0 + baseline benchmark | Verifikasi performa tanpa menunggu GUI |
| 9 | Alur rescan (append-only + kompaksi) dan penghapusan dikembalikan | M5 membutuhkan spesifikasi delta |
| 10 | Spike Windows & Iced di M1; buffer M4 | Kurangi risiko estimasi |
| 11 | Pembanding M7 diganti pembanding nyata (`du`/WizTree) | "Performa teoretis" tidak terukur |

---

## 15. Langkah Berikutnya

1. **Review dokumen ini**, khususnya keputusan default `AkurasiTautan` di Windows (bagian 6.4) dan target kernel minimum Linux (io_uring vs fallback).
2. **Jalankan M0**, termasuk pengukuran baseline FastDiskScan lama untuk mengunci angka bagian 11.
3. **Update dokumen** setiap milestone selesai; versi resmi disimpan di `docs/architecture.md`.

---

**Status:** Draft untuk review. Perubahan pada bagian 4 (tipe data) dan bagian 6 (HAL) memerlukan persetujuan sebelum M1 dimulai.
