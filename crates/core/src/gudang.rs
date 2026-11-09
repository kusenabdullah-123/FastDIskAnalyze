use crate::entri::RekamanEntri;
use crate::ids::{IdEntri, IdNama};
use hashbrown::HashTable;
use rustc_hash::FxHasher;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

fn hash_bytes(b: &[u8]) -> u64 {
    let mut h = FxHasher::default();
    b.hash(&mut h);
    h.finish()
}

#[derive(Clone, Debug)]
pub struct GudangNama {
    pub teks_mentah: Vec<u8>,
    pub daftar_posisi: Vec<(u32, u32)>, // (awal, panjang) per IdNama
    indeks_nama: HashTable<IdNama>,     // hanya menyimpan IdNama; kunci dibaca dari teks_mentah
}

impl GudangNama {
    pub fn baru() -> Self {
        Self {
            teks_mentah: Vec::new(),
            daftar_posisi: Vec::new(),
            indeks_nama: HashTable::new(),
        }
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

    /// Untuk tampilan GUI / CLI: tidak pernah panic. `Cow::Borrowed` jika UTF-8 valid.
    pub fn nama_tampil(&self, id: IdNama) -> Cow<'_, str> {
        String::from_utf8_lossy(self.ambil_nama(id))
    }

    pub fn jumlah(&self) -> usize {
        self.daftar_posisi.len()
    }
}

impl Default for GudangNama {
    fn default() -> Self {
        Self::baru()
    }
}

#[derive(Clone, Debug)]
pub struct GudangEntri {
    pub daftar_entri: Vec<RekamanEntri>,
    pub nama: GudangNama,
}

impl GudangEntri {
    pub fn baru() -> Self {
        Self {
            daftar_entri: Vec::with_capacity(100_000),
            nama: GudangNama::baru(),
        }
    }

    pub fn dengan_kapasitas(kapasitas: usize) -> Self {
        Self {
            daftar_entri: Vec::with_capacity(kapasitas),
            nama: GudangNama::baru(),
        }
    }

    pub fn tambah_entri(&mut self, rekaman: RekamanEntri) -> IdEntri {
        let id = IdEntri(self.daftar_entri.len() as u32);
        self.daftar_entri.push(rekaman);
        id
    }

    pub fn ambil(&self, id: IdEntri) -> &RekamanEntri {
        &self.daftar_entri[id.0 as usize]
    }

    pub fn ambil_mut(&mut self, id: IdEntri) -> &mut RekamanEntri {
        &mut self.daftar_entri[id.0 as usize]
    }

    pub fn jumlah(&self) -> usize {
        self.daftar_entri.len()
    }

    pub fn is_empty(&self) -> bool {
        self.daftar_entri.is_empty()
    }
}

impl Default for GudangEntri {
    fn default() -> Self {
        Self::baru()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gudang_nama_roundtrip_dan_interning() {
        let mut gudang = GudangNama::baru();
        let id1 = gudang.simpan_nama(b"berkas_a.txt");
        let id2 = gudang.simpan_nama(b"berkas_b.txt");
        let id3 = gudang.simpan_nama(b"berkas_a.txt"); // duplikat

        assert_eq!(id1, id3);
        assert_ne!(id1, id2);
        assert_eq!(gudang.ambil_nama(id1), b"berkas_a.txt");
        assert_eq!(gudang.ambil_nama(id2), b"berkas_b.txt");
        assert_eq!(gudang.jumlah(), 2);
    }

    #[test]
    fn test_nama_non_utf8_lossy() {
        let mut gudang = GudangNama::baru();
        // Byte tidak valid UTF-8
        let invalid_utf8 = [0xFF, 0xFE, 0xFD];
        let id = gudang.simpan_nama(&invalid_utf8);

        assert_eq!(gudang.ambil_nama(id), &invalid_utf8);
        let tampil = gudang.nama_tampil(id);
        assert!(tampil.contains('\u{FFFD}')); // fallback lossy replacement character
    }
}
