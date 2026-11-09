use crate::icons::{
    icon_colored, ICON_FILE, ICON_FOLDER, ICON_SORT_DEFAULT, ICON_SORT_DOWN, ICON_SORT_UP,
    ICON_TRASH,
};
use crate::theme::{
    ghost_button, header_sort_button, table_header_container, table_row_button, COLOR_DANGER,
    COLOR_FOLDER, COLOR_PRIMARY, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::view_state::{SortColumn, ViewState};
use crate::widgets::size_bar::{format_bytes, size_bar_widget};
use fast_disk_analyze_core::{IdEntri, PohonSistemBerkas, SortKey};
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{Element, Length};
use std::sync::Arc;

pub fn virtual_table_widget<'a, Message: 'a + Clone + 'static>(
    model: &'a PohonSistemBerkas,
    state: &ViewState,
    on_toggle_sort: impl Fn(SortColumn) -> Message + 'static + Copy,
    on_navigate: impl Fn(IdEntri) -> Message + 'static + Copy,
    on_select: impl Fn(IdEntri) -> Message + 'static + Copy,
    on_scroll: impl Fn(scrollable::Viewport) -> Message + 'static + Copy,
    on_delete_trash: impl Fn(IdEntri) -> Message + 'static + Copy,
    on_right_click: impl Fn(IdEntri) -> Message + 'static + Copy,
) -> Element<'a, Message> {
    let current_dir = state.current_dir.unwrap_or(IdEntri(0));

    // Ambil daftar anak terurut via Query API domain (O(1) dari cache)
    let entries: Arc<[IdEntri]> = model.sorted(current_dir, state.sort_key);
    let total_count = entries.len();

    // 1. Header: NAMA, UKURAN, dan Ruang Tombol Hapus
    let sort_icon_for = |col: SortColumn| match (col, state.sort_key) {
        (SortColumn::Nama, SortKey::NamaAZ) => icon_colored(ICON_SORT_UP, COLOR_PRIMARY).size(11),
        (SortColumn::Nama, SortKey::NamaZA) => icon_colored(ICON_SORT_DOWN, COLOR_PRIMARY).size(11),
        (SortColumn::Ukuran, SortKey::UkuranMenaik) => icon_colored(ICON_SORT_UP, COLOR_PRIMARY).size(11),
        (SortColumn::Ukuran, SortKey::UkuranMenurun) => icon_colored(ICON_SORT_DOWN, COLOR_PRIMARY).size(11),
        _ => icon_colored(ICON_SORT_DEFAULT, TEXT_MUTED).size(11),
    };

    let header_cell = |label: &'static str, col: SortColumn, width: Length| {
        button(
            row![
                text(label).size(12).color(TEXT_SECONDARY),
                Space::with_width(Length::Fixed(6.0)),
                sort_icon_for(col),
            ]
            .align_y(iced::Alignment::Center),
        )
        .width(width)
        .padding([6, 10])
        .style(header_sort_button)
        .on_press(on_toggle_sort(col))
    };

    let header_row = row![
        header_cell("NAMA", SortColumn::Nama, Length::Fill),
        header_cell("UKURAN", SortColumn::Ukuran, Length::Fixed(220.0)),
        Space::with_width(Length::Fixed(52.0)), // Ruang untuk tombol hapus (28px) + jarak scrollbar (24px)
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let header = container(header_row)
        .width(Length::Fill)
        .style(table_header_container);

    // 2. Windowing Viewport Calculation
    let (start_idx, end_idx) = state.visible_range(total_count);

    let mut rows_col = column![].spacing(2);

    // Spacer Atas
    if start_idx > 0 {
        let top_height = start_idx as f32 * ViewState::ROW_HEIGHT;
        rows_col = rows_col.push(Space::with_height(Length::Fixed(top_height)));
    }

    // Baris-baris yang tampak di viewport (start_idx .. end_idx)
    for &id in &entries[start_idx..end_idx] {
        let is_dir = model.adalah_direktori(id);
        let raw_name = model.nama(id);
        let display_name = if raw_name.is_empty() {
            String::from("/")
        } else {
            raw_name.into_owned()
        };

        let item_icon = if is_dir {
            icon_colored(ICON_FOLDER, COLOR_FOLDER).size(14)
        } else {
            icon_colored(ICON_FILE, TEXT_MUTED).size(14)
        };

        let bytes = model.effective_size(id, state.size_mode);
        let size_str = format_bytes(bytes);
        let pct = (model.size_percentage(id) / 100.0) as f32;
        let is_selected = state.selected_node == Some(id);

        let name_col = row![
            item_icon,
            Space::with_width(Length::Fixed(10.0)),
            text(display_name)
                .size(13)
                .color(TEXT_PRIMARY),
        ]
        .align_y(iced::Alignment::Center)
        .width(Length::Fill);

        let size_col = row![
            text(size_str)
                .size(12)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(85.0)),
            container(size_bar_widget(pct))
                .width(Length::Fixed(125.0)),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .width(Length::Fixed(218.0));

        let main_click_area = row![
            name_col,
            size_col,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill);

        let mut main_btn = button(main_click_area)
            .width(Length::Fill)
            .padding([5, 8])
            .style(table_row_button(is_selected));

        if is_dir {
            main_btn = main_btn.on_press(on_navigate(id));
        } else {
            main_btn = main_btn.on_press(on_select(id));
        }

        let main_area = mouse_area(main_btn).on_right_press(on_right_click(id));

        // Tombol hapus di sebelah kanan kapasitas (default ke trash)
        let trash_btn = button(
            container(icon_colored(ICON_TRASH, COLOR_DANGER).size(13))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(26.0))
        .padding([4, 4])
        .style(ghost_button)
        .on_press(on_delete_trash(id));

        let full_row = row![
            main_area,
            Space::with_width(Length::Fixed(4.0)),
            trash_btn,
            Space::with_width(Length::Fixed(20.0)), // Padding kanan agar TIDAK tertutup scrollbar vertikal
        ]
        .align_y(iced::Alignment::Center)
        .width(Length::Fill);

        rows_col = rows_col.push(full_row);
    }

    // Spacer Bawah
    if end_idx < total_count {
        let bottom_height = (total_count - end_idx) as f32 * ViewState::ROW_HEIGHT;
        rows_col = rows_col.push(Space::with_height(Length::Fixed(bottom_height)));
    }

    let scroll = scrollable(rows_col)
        .height(Length::Fill)
        .width(Length::Fill)
        .on_scroll(on_scroll);

    column![
        header,
        Space::with_height(Length::Fixed(4.0)),
        scroll,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
