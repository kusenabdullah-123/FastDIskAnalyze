use crate::ids::IdEntri;
use fast_disk_analyze_platform::ScanError;

/// Catatan kesalahan per entri, terurut berdasarkan IdEntri untuk pencarian biner O(log E).
#[derive(Clone, Debug, Default)]
pub struct ErrorLog {
    entries: Vec<(IdEntri, ScanError)>,
}

impl ErrorLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn catat(&mut self, id: IdEntri, err: ScanError) {
        self.entries.push((id, err));
    }

    pub fn selesaikan_urut(&mut self) {
        self.entries.sort_by_key(|(id, _)| *id);
    }

    pub fn untuk_entri(&self, id: IdEntri) -> Vec<&ScanError> {
        let idx = self.entries.binary_search_by_key(&id, |(k, _)| *k);
        match idx {
            Ok(found) => {
                let mut hasil = vec![&self.entries[found].1];
                let mut i = found;
                while i > 0 && self.entries[i - 1].0 == id {
                    i -= 1;
                    hasil.push(&self.entries[i].1);
                }
                let mut j = found + 1;
                while j < self.entries.len() && self.entries[j].0 == id {
                    hasil.push(&self.entries[j].1);
                    j += 1;
                }
                hasil
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn jumlah(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

