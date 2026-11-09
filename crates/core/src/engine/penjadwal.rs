use fast_disk_analyze_platform::MediumKind;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Kebijakan {
    pub threads: usize,
    pub queue_depth: usize,
    pub timeout: Option<Duration>,
}

pub struct PenjadwalScan;

impl PenjadwalScan {
    pub fn policy(medium: MediumKind) -> Kebijakan {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        match medium {
            MediumKind::Hdd => Kebijakan {
                threads: 1,
                queue_depth: 1,
                timeout: None,
            },
            MediumKind::SataSsd => Kebijakan {
                threads: cores.clamp(2, 4),
                queue_depth: 8,
                timeout: None,
            },
            MediumKind::NvmeSsd => Kebijakan {
                threads: cores.clamp(4, 16),
                queue_depth: 32,
                timeout: None,
            },
            MediumKind::Network => Kebijakan {
                threads: 2,
                queue_depth: 2,
                timeout: Some(Duration::from_secs(5)),
            },
            MediumKind::Removable => Kebijakan {
                threads: 2,
                queue_depth: 2,
                timeout: Some(Duration::from_secs(10)),
            },
            MediumKind::Unknown => Kebijakan {
                threads: 2,
                queue_depth: 2,
                timeout: None,
            },
        }
    }
}

