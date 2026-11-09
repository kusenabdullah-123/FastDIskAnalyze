use crate::engine::pengawas::PengawasMemori;
use crate::gudang::GudangEntri;
use fast_disk_analyze_platform::{CancellationToken, DirIterator, IdentitasBerkas, ScanError};
use std::path::Path;

pub struct Pemindai;

impl Pemindai {
    pub fn scan_path<I: DirIterator>(
        path: &Path,
        iterator: &mut I,
        token: &CancellationToken,
        _governor: &PengawasMemori,
    ) -> Result<(GudangEntri, Vec<(usize, IdentitasBerkas)>), ScanError> {
        let mut gudang = GudangEntri::baru();
        let mut tautan_list = Vec::new();

        iterator.walk(path, token, &mut |raw| {
            let id_nama = gudang.nama.simpan_nama(raw.nama);
            let entri_idx = gudang.daftar_entri.len();

            let rekaman = crate::entri::RekamanEntri {
                ukuran_asli: raw.ukuran_asli,
                ukuran_blok: raw.ukuran_blok,
                induk: raw.induk,
                id_nama: id_nama.0,
                anak_awal: 0,
                jumlah_anak: 0,
                jumlah_tautan: raw.jumlah_tautan,
                jenis: raw.jenis as u8,
                penanda: 0,
                _padding: 0,
            };

            gudang.tambah_entri(rekaman);

            if let Some(identitas) = raw.identitas {
                tautan_list.push((entri_idx, identitas));
            }
        })?;

        Ok((gudang, tautan_list))
    }
}

