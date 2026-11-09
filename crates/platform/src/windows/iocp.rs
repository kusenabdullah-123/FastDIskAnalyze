use crate::traits::{Completion, IoBackend};

pub struct WindowsIocpBackend;

impl WindowsIocpBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsIocpBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl IoBackend for WindowsIocpBackend {
    type Req = ();

    fn submit_batch(&self, _reqs: &mut [Self::Req]) {
        // Implementasi IOCP (M7)
    }

    fn poll_completions(&self, _max: usize) -> Vec<Completion> {
        Vec::new()
    }

    fn cancel_all(&self) {}
}

