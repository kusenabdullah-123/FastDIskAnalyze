#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CapPolicy {
    None,
    Soft(usize), // Batas jumlah entri sebelum beralih ke Summarized
    Hard(usize), // Batas absolut jumlah entri sebelum scan dihentikan (Incomplete)
}

#[derive(Clone, Debug)]
pub struct PengawasMemori {
    pub cap_policy: CapPolicy,
}

impl PengawasMemori {
    pub fn new(cap_policy: CapPolicy) -> Self {
        Self { cap_policy }
    }

    #[inline(always)]
    pub fn periksa_batas(&self, jumlah_entri: usize) -> StatusMemori {
        match self.cap_policy {
            CapPolicy::None => StatusMemori::Aman,
            CapPolicy::Soft(soft) => {
                if jumlah_entri >= soft {
                    StatusMemori::Ringkas
                } else {
                    StatusMemori::Aman
                }
            }
            CapPolicy::Hard(hard) => {
                if jumlah_entri >= hard {
                    StatusMemori::Hentikan
                } else {
                    StatusMemori::Aman
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StatusMemori {
    Aman,
    Ringkas,
    Hentikan,
}

