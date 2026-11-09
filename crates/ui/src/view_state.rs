use fast_disk_analyze_core::{IdEntri, SizeMode, SortKey};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SortColumn {
    Nama,
    Ukuran,
    Item,
}

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
    pub scroll_offset_y: f32,
    pub viewport_height: f32,
    pub context_menu_target: Option<IdEntri>,
}

impl ViewState {
    pub const ROW_HEIGHT: f32 = 30.0;

    pub fn new() -> Self {
        Self {
            selected_node: None,
            current_dir: None,
            nav_stack: Vec::new(),
            sort_key: SortKey::UkuranMenurun,
            size_mode: SizeMode::UkuranBlok,
            filter_query: String::new(),
            is_scanning: false,
            scroll_offset_y: 0.0,
            viewport_height: 600.0,
            context_menu_target: None,
        }
    }

    pub fn navigate_to(&mut self, dir: IdEntri) {
        if let Some(curr) = self.current_dir {
            if curr != dir {
                self.nav_stack.push(curr);
            }
        }
        self.current_dir = Some(dir);
        self.selected_node = None;
        self.context_menu_target = None;
        self.scroll_offset_y = 0.0;
    }

    pub fn navigate_up(&mut self) -> Option<IdEntri> {
        let prev = self.nav_stack.pop();
        if prev.is_some() {
            self.current_dir = prev;
            self.selected_node = None;
            self.context_menu_target = None;
            self.scroll_offset_y = 0.0;
        }
        prev
    }

    pub fn toggle_sort(&mut self, column: SortColumn) {
        self.sort_key = match column {
            SortColumn::Ukuran => match self.sort_key {
                SortKey::UkuranMenurun => SortKey::UkuranMenaik,
                _ => SortKey::UkuranMenurun,
            },
            SortColumn::Nama => match self.sort_key {
                SortKey::NamaAZ => SortKey::NamaZA,
                _ => SortKey::NamaAZ,
            },
            SortColumn::Item => match self.sort_key {
                SortKey::JumlahAnakMenurun => SortKey::JumlahAnakMenaik,
                _ => SortKey::JumlahAnakMenurun,
            },
        };
    }

    /// Menghitung rentang indeks baris yang terlihat (start..end) untuk virtual table windowing
    pub fn visible_range(&self, total_items: usize) -> (usize, usize) {
        if total_items == 0 {
            return (0, 0);
        }
        let start = (self.scroll_offset_y / Self::ROW_HEIGHT).floor() as usize;
        let start = start.min(total_items);
        let visible_count = (self.viewport_height / Self::ROW_HEIGHT).ceil() as usize + 6; // buffer 6 baris
        let end = (start + visible_count).min(total_items);
        (start, end)
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
        if let Some(sel) = self.selected_node {
            if !is_valid(sel) {
                self.selected_node = None;
            }
        }
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_state_navigation() {
        let mut state = ViewState::new();
        assert_eq!(state.current_dir, None);
        assert!(state.nav_stack.is_empty());

        state.navigate_to(IdEntri(10));
        assert_eq!(state.current_dir, Some(IdEntri(10)));
        assert!(state.nav_stack.is_empty());

        state.navigate_to(IdEntri(20));
        assert_eq!(state.current_dir, Some(IdEntri(20)));
        assert_eq!(state.nav_stack, vec![IdEntri(10)]);

        let up = state.navigate_up();
        assert_eq!(up, Some(IdEntri(10)));
        assert_eq!(state.current_dir, Some(IdEntri(10)));
        assert!(state.nav_stack.is_empty());

        let up2 = state.navigate_up();
        assert_eq!(up2, None);
    }

    #[test]
    fn test_virtual_table_visible_range() {
        let mut state = ViewState::new();
        state.viewport_height = 600.0; // 20 baris @ 30px + buffer 6 = 26 baris

        // 1. Kasus kosong
        assert_eq!(state.visible_range(0), (0, 0));

        // 2. Pada posisi paling atas
        state.scroll_offset_y = 0.0;
        let (s, e) = state.visible_range(100_000);
        assert_eq!(s, 0);
        assert_eq!(e, 26);

        // 3. Scroll 100 baris ke bawah (3000px)
        state.scroll_offset_y = 3000.0;
        let (s, e) = state.visible_range(100_000);
        assert_eq!(s, 100);
        assert_eq!(e, 126);

        // 4. Di dekat akhir dokumen
        state.scroll_offset_y = 2_999_700.0;
        let (s, e) = state.visible_range(100_000);
        assert_eq!(s, 99990);
        assert_eq!(e, 100_000);
    }

    #[test]
    fn test_toggle_sort() {
        let mut state = ViewState::new();
        assert_eq!(state.sort_key, SortKey::UkuranMenurun);

        state.toggle_sort(SortColumn::Ukuran);
        assert_eq!(state.sort_key, SortKey::UkuranMenaik);

        state.toggle_sort(SortColumn::Ukuran);
        assert_eq!(state.sort_key, SortKey::UkuranMenurun);

        state.toggle_sort(SortColumn::Nama);
        assert_eq!(state.sort_key, SortKey::NamaAZ);

        state.toggle_sort(SortColumn::Nama);
        assert_eq!(state.sort_key, SortKey::NamaZA);
    }

    #[test]
    fn test_clamp_nav_stack() {
        let mut state = ViewState::new();
        state.navigate_to(IdEntri(1));
        state.navigate_to(IdEntri(2));
        state.navigate_to(IdEntri(3));

        // Anggap node 2 terhapus, hanya node 1 dan 3 yang valid
        state.clamp_nav_stack(|id| id.0 != 2);
        assert_eq!(state.nav_stack, vec![IdEntri(1)]);
        assert_eq!(state.current_dir, Some(IdEntri(3)));
    }
}
