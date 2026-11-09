use bytemuck::{Pod, Zeroable};
pub use fast_disk_analyze_platform::JenisEntri;

/// Bit flag untuk RekamanEntri.penanda
pub mod penanda {
    pub const NAMA_BUKAN_UTF8: u8 = 1 << 0; // nama dikonversi lossy saat tampil
    pub const ADA_ERROR: u8 = 1 << 1; // lihat ErrorLog
    pub const TAUTAN_GANDA: u8 = 1 << 2; // hardlink kedua+; ukuran tidak dihitung ulang
    pub const TIDAK_DITELUSURI: u8 = 1 << 3; // direktori dilewati (mount lain / reparse / cap memori)
    pub const DIRINGKAS: u8 = 1 << 4; // anak tidak disimpan (mode Summarized)
    pub const YATIM: u8 = 1 << 5; // entri telah dihapus / yatim setelah delta update
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct RekamanEntri {
    // Urutan field: u64 dulu, lalu u32, lalu u8/u16  =>  tanpa padding implisit.
    pub ukuran_asli: u64,   // 8  (apparent size / EndOfFile). Direktori: agregat.
    pub ukuran_blok: u64,   // 8  (alokasi nyata di disk).     Direktori: agregat.
    pub induk: u32,         // 4  (INDUK_AKAR jika root)
    pub id_nama: u32,       // 4  (IdNama.0)
    pub anak_awal: u32,     // 4  (indeks anak pertama; valid setelah perakitan)
    pub jumlah_anak: u32,   // 4
    pub jumlah_tautan: u32, // 4  (nlink)
    pub jenis: u8,          // 1  (JenisEntri as u8)
    pub penanda: u8,        // 1  (bit flag)
    pub _padding: u16,      // 2  (eksplisit)
}

const _: () = assert!(core::mem::size_of::<RekamanEntri>() == 40);
const _: () = assert!(core::mem::align_of::<RekamanEntri>() == 8);

impl RekamanEntri {
    pub fn jenis(&self) -> JenisEntri {
        JenisEntri::try_from(self.jenis).unwrap_or(JenisEntri::Lainnya)
    }

    pub fn anak(&self) -> core::ops::Range<usize> {
        let a = self.anak_awal as usize;
        a..a + self.jumlah_anak as usize
    }

    pub fn adalah_direktori(&self) -> bool {
        self.jenis == JenisEntri::Direktori as u8
    }

    pub fn adalah_berkas(&self) -> bool {
        self.jenis == JenisEntri::Berkas as u8
    }

    pub fn adalah_yatim(&self) -> bool {
        self.penanda & penanda::YATIM != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rekaman_entri_pod() {
        let entri = RekamanEntri {
            ukuran_asli: 1024,
            ukuran_blok: 4096,
            induk: crate::ids::INDUK_AKAR,
            id_nama: 0,
            anak_awal: 0,
            jumlah_anak: 0,
            jumlah_tautan: 1,
            jenis: JenisEntri::Berkas as u8,
            penanda: 0,
            _padding: 0,
        };

        let bytes = bytemuck::bytes_of(&entri);
        assert_eq!(bytes.len(), 40);

        let roundtrip: &RekamanEntri = bytemuck::from_bytes(bytes);
        assert_eq!(roundtrip.ukuran_asli, 1024);
        assert_eq!(roundtrip.ukuran_blok, 4096);
        assert_eq!(roundtrip.induk, crate::ids::INDUK_AKAR);
    }
}

