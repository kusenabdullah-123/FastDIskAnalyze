use crate::view_state::ViewState;

pub struct FastDiskApp {
    pub state: ViewState,
}

impl FastDiskApp {
    pub fn new() -> Self {
        Self {
            state: ViewState::new(),
        }
    }
}

impl Default for FastDiskApp {
    fn default() -> Self {
        Self::new()
    }
}

