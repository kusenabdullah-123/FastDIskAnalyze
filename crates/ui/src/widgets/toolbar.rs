use crate::view_state::ViewState;
use fast_disk_analyze_core::SizeMode;
use iced::widget::{button, row, text, text_input};
use iced::{Element, Length};
use std::path::PathBuf;

pub fn toolbar_widget<'a, Message: 'a + Clone>(
    state: &ViewState,
    target_path: &'a str,
    quick_drives: &[PathBuf],
    on_open_dialog: Message,
    on_select_drive: impl Fn(PathBuf) -> Message + 'static + Copy,
    on_start_scan: Message,
    on_cancel_scan: Message,
    on_filter_changed: impl Fn(String) -> Message + 'static + Copy,
    on_toggle_size_mode: Message,
) -> Element<'a, Message> {
    // 1. Baris Pertama: Pemilih folder/drive dan kontrol scan
    let mut drive_buttons = row![].spacing(4).align_y(iced::Alignment::Center);
    for drive in quick_drives {
        let label = drive.to_string_lossy().to_string();
        drive_buttons = drive_buttons.push(
            button(text(label).size(12))
                .padding([4, 8])
                .on_press(on_select_drive(drive.clone())),
        );
    }

    let folder_btn = button(text("📁 Pilih Folder...").size(13))
        .padding([6, 12])
        .on_press(on_open_dialog);

    let path_label = text(if target_path.is_empty() {
        "Pilih direktori atau drive untuk memulai pemindaian"
    } else {
        target_path
    })
    .size(13);

    let scan_btn = if state.is_scanning {
        button(text("⏳ Memindai...").size(13))
            .padding([6, 14])
    } else {
        button(text("▶ Mulai Scan").size(13))
            .padding([6, 14])
            .style(button::primary)
            .on_press(on_start_scan)
    };

    let cancel_btn = if state.is_scanning {
        button(text("⏹ Batal").size(13))
            .padding([6, 14])
            .style(button::danger)
            .on_press(on_cancel_scan)
    } else {
        button(text("⏹ Batal").size(13))
            .padding([6, 14])
    };

    let size_mode_text = match state.size_mode {
        SizeMode::UkuranBlok => "💾 Ukuran Disk (Blok)",
        SizeMode::UkuranAsli => "📄 Ukuran File (Logis)",
    };
    let size_mode_btn = button(text(size_mode_text).size(12))
        .padding([6, 10])
        .on_press(on_toggle_size_mode);

    let row1 = row![
        folder_btn,
        drive_buttons,
        path_label,
        iced::widget::Space::with_width(Length::Fill),
        size_mode_btn,
        scan_btn,
        cancel_btn,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    // 2. Baris Kedua: Pencarian Live Filter
    let search_input = text_input(
        "🔍 Filter nama berkas / folder di direktori ini (zero-allocation)...",
        &state.filter_query,
    )
    .on_input(on_filter_changed)
    .padding([6, 10])
    .size(13)
    .width(Length::Fill);

    iced::widget::column![
        row1,
        search_input,
    ]
    .spacing(8)
    .into()
}
