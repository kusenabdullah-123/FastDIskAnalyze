use crate::icons::{
    icon_colored, ICON_FILE, ICON_FOLDER, ICON_SORT_DEFAULT, ICON_SORT_DOWN, ICON_SORT_UP,
};
use crate::theme::{
    header_sort_button, table_header_container, table_row_button, COLOR_FOLDER, COLOR_PRIMARY,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::view_state::{SortColumn, ViewState};
use crate::widgets::size_bar::{format_bytes, size_bar_widget};
use fast_disk_analyze_core::{IdEntri, PohonSistemBerkas, SortKey};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Element, Length};
use std::sync::Arc;

pub fn virtual_table_widget<'a, Message: 'a + Clone + 'static>(
    model: &'a PohonSistemBerkas,
    state: &ViewState,
    on_toggle_sort: impl Fn(SortColumn) -> Message + 'static + Copy,
    on_navigate: impl Fn(IdEntri) -> Message + 'static + Copy,
    on_select: impl Fn(IdEntri) -> Message + 'static + Copy,
    on_scroll: impl Fn(scrollable::Viewport) -> Message + 'static + Copy,
) -> Element<'a, Message> {
    let current_dir = state.current_dir.unwrap_or(IdEntri(0));

    // Ambil daftar anak terurut via Query API domain (O(1) dari cache)
    let entries: Arc<[IdEntri]> = model.sorted(current_dir, state.sort_key);
    let total_count = entries.len();

    // 1. Header 2 Kolom Bersih: NAMA dan UKURAN
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
        header_cell("UKURAN", SortColumn::Ukuran, Length::Fixed(240.0)),
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
                .width(Length::Fixed(90.0)),
            container(size_bar_widget(pct))
                .width(Length::Fixed(140.0)),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .width(Length::Fixed(240.0));

        let row_content = row![
            name_col,
            size_col,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let mut row_btn = button(row_content)
            .width(Length::Fill)
            .padding([5, 10])
            .style(table_row_button(is_selected));

        if is_dir {
            row_btn = row_btn.on_press(on_navigate(id));
        } else {
            row_btn = row_btn.on_press(on_select(id));
        }

        rows_col = rows_col.push(row_btn);
    }

    // Spacer Bawah
    if end_idx < total_count {
        let bottom_height = (total_count - end_idx) as f32 * ViewState::ROW_HEIGHT;
        rows_col = rows_col.push(Space::with_height(Length::Fixed(bottom_height)));
    }

    let scroll = scrollable(rows_col)
        .on_scroll(on_scroll)
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::default()))
        .width(Length::Fill)
        .height(Length::Fill);

    column![
        header,
        Space::with_height(Length::Fixed(4.0)),
        scroll,
    ]
    .spacing(2)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
