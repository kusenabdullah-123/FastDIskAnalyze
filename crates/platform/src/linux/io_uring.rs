use crate::traits::{Completion, IoBackend};

pub struct LinuxIoUringBackend;

impl LinuxIoUringBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxIoUringBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl IoBackend for LinuxIoUringBackend {
    type Req = ();

    fn submit_batch(&self, _reqs: &mut [Self::Req]) {
        // Implementasi io_uring batching (M7)
    }

    fn poll_completions(&self, _max: usize) -> Vec<Completion> {
        Vec::new()
    }

    fn cancel_all(&self) {}
}

