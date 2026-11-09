use crate::icons::{
    icon_colored, ICON_BOLT, ICON_CHECK, ICON_GLOBE, ICON_HDD, ICON_SPINNER, ICON_USB,
};
use crate::theme::{
    chip_style, COLOR_CYAN, COLOR_PRIMARY, COLOR_SUCCESS, COLOR_WARNING, TEXT_MUTED,
    TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::view_state::ViewState;
use crate::widgets::size_bar::{format_bytes, format_count};
use fast_disk_analyze_core::{IdEntri, PohonSistemBerkas};
use fast_disk_analyze_platform::MediumKind;
use iced::widget::{container, row, text, Space};
use iced::{Element, Length};

#[derive(Clone, Debug)]
pub struct ProgressTick {
    pub items: u64,
    pub bytes: u64,
    pub errors: u64,
    pub elapsed_ms: u64,
}

pub fn status_bar_widget<'a, Message: 'a + 'static>(
    state: &ViewState,
    model: Option<&PohonSistemBerkas>,
    last_tick: Option<&ProgressTick>,
    medium: Option<MediumKind>,
) -> Element<'a, Message> {
    let mut left_row = row![].spacing(8).align_y(iced::Alignment::Center);

    if state.is_scanning {
        left_row = left_row.push(icon_colored(ICON_SPINNER, COLOR_CYAN).size(13));
        if let Some(tick) = last_tick {
            let speed = if tick.elapsed_ms > 0 {
                (tick.items as f64 / (tick.elapsed_ms as f64 / 1000.0)) as u64
            } else {
                0
            };
            left_row = left_row.push(
                text(format!(
                    "Memindai: {:.1}s • {} entri • {} • {} entri/s",
                    tick.elapsed_ms as f64 / 1000.0,
                    format_count(tick.items as u32),
                    format_bytes(tick.bytes),
                    format_count(speed as u32)
                ))
                .size(12)
                .color(TEXT_SECONDARY),
            );
        } else {
            left_row = left_row.push(
                text("Memulai proses pemindaian...")
                    .size(12)
                    .color(TEXT_SECONDARY),
            );
        }
    } else if let Some(m) = model {
        let root_size = m.effective_size(IdEntri(0), state.size_mode);
        let root_items = m.jumlah_anak(IdEntri(0));
        let (icon_code, icon_col, med_str) = match medium.unwrap_or(MediumKind::Unknown) {
            MediumKind::NvmeSsd => (ICON_BOLT, COLOR_CYAN, "NVMe SSD"),
            MediumKind::SataSsd => (ICON_BOLT, COLOR_PRIMARY, "SATA SSD"),
            MediumKind::Hdd => (ICON_HDD, COLOR_WARNING, "HDD"),
            MediumKind::Removable => (ICON_USB, COLOR_SUCCESS, "USB Removable"),
            MediumKind::Network => (ICON_GLOBE, COLOR_PRIMARY, "Network FS"),
            MediumKind::Unknown => (ICON_HDD, COLOR_CYAN, "Penyimpanan"),
        };

        left_row = left_row.push(icon_colored(ICON_CHECK, COLOR_SUCCESS).size(13));
        left_row = left_row.push(
            text(format!(
                "Total: {} ({} item)",
                format_bytes(root_size),
                format_count(root_items)
            ))
            .size(12)
            .color(TEXT_PRIMARY),
        );
        left_row = left_row.push(
            container(
                row![
                    icon_colored(icon_code, icon_col).size(11),
                    Space::with_width(Length::Fixed(4.0)),
                    text(med_str).size(11).color(icon_col),
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding([2, 6])
            .style(chip_style),
        );
    } else {
        left_row = left_row.push(
            text("Siap memindai direktori atau drive.")
                .size(12)
                .color(TEXT_MUTED),
        );
    }

    let right_row = if let Some(m) = model {
        if let Some(sel) = state.selected_node {
            let sel_name = m.display_path(sel);
            let sel_size = m.effective_size(sel, state.size_mode);
            text(format!("Terpilih: {} ({})", sel_name, format_bytes(sel_size)))
                .size(12)
                .color(TEXT_SECONDARY)
        } else if let Some(curr) = state.current_dir {
            let curr_size = m.effective_size(curr, state.size_mode);
            let curr_items = m.jumlah_anak(curr);
            text(format!(
                "Folder ini: {} ({} item)",
                format_bytes(curr_size),
                format_count(curr_items)
            ))
            .size(12)
            .color(TEXT_MUTED)
        } else {
            text("").size(12)
        }
    } else {
        text("").size(12)
    };

    row![
        left_row,
        Space::with_width(Length::Fill),
        right_row,
    ]
    .spacing(8)
    .padding([6, 14])
    .align_y(iced::Alignment::Center)
    .into()
}
