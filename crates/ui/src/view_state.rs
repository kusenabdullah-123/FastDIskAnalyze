use fast_disk_analyze_core::{IdEntri, SizeMode, SortKey};

/// Keadaan tampilan GUI (tipis, TIDAK memegang PohonSistemBerkas / RekamanEntri).
#[derive(Clone, Debug)]
pub struct ViewState {
    pub selected_node: Option<IdEntri>,
    pub current_dir: Option<IdEntri>,
    pub nav_stack: Vec<IdEntri>,
    pub sort_key: SortKey,
    pub size_mode: SizeMode,
    pub filter_query: String,
    pub is_scanning: bool,
}

impl ViewState {
    pub fn new() -> Self {
        Self {
            selected_node: None,
            current_dir: None,
            nav_stack: Vec::new(),
            sort_key: SortKey::UkuranMenurun,
            size_mode: SizeMode::UkuranBlok,
            filter_query: String::new(),
            is_scanning: false,
        }
    }

    pub fn navigate_to(&mut self, dir: IdEntri) {
        if let Some(curr) = self.current_dir {
            self.nav_stack.push(curr);
        }
        self.current_dir = Some(dir);
    }

    pub fn navigate_up(&mut self) -> Option<IdEntri> {
        let prev = self.nav_stack.pop();
        if prev.is_some() {
            self.current_dir = prev;
        }
        prev
    }

    /// Memastikan stack navigasi tetap valid setelah rescan / penghapusan (clamp)
    pub fn clamp_nav_stack<F>(&mut self, is_valid: F)
    where
        F: Fn(IdEntri) -> bool,
    {
        self.nav_stack.retain(|&id| is_valid(id));
        if let Some(curr) = self.current_dir {
            if !is_valid(curr) {
                self.current_dir = self.nav_stack.pop();
            }
        }
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}

