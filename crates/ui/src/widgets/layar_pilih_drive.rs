use crate::icons::{
    icon_colored, ICON_BOLT, ICON_CHEVRON_RIGHT, ICON_DATABASE, ICON_GLOBE, ICON_HDD, ICON_HOME,
    ICON_USB,
};
use crate::theme::{
    boxed_list_container, divider_style, drive_row_button, dynamic_progress_bar_style,
    icon_badge_container, COLOR_CYAN, COLOR_PRIMARY, COLOR_SUCCESS, COLOR_WARNING, TEXT_MUTED,
    TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::widgets::size_bar::format_bytes;
use fast_disk_analyze_platform::{InformasiDrive, MediumKind};
use iced::widget::{button, column, container, progress_bar, row, text, Space};
use iced::{Element, Length};

pub fn view_pilih_drive<'a, Message: 'a + Clone + 'static>(
    drives: &'a [InformasiDrive],
    on_select_drive: impl Fn(InformasiDrive) -> Message + 'static + Copy,
) -> Element<'a, Message> {
    // 1. Header Atas (Devices & Locations)
    let header_title = text("Devices & Locations")
        .size(15)
        .color(TEXT_PRIMARY);

    // 2. Section Header (This Device)
    let section_label = text("This Device")
        .size(12)
        .color(TEXT_MUTED);

    // 3. Grouped Boxed List (Mirip GNOME Disks / Baobab)
    let mut list_col = column![].width(Length::Fill);
    let total_drives = drives.len();

    for (idx, drive) in drives.iter().enumerate() {
        let is_home = drive.label.starts_with("Home Pengguna") || drive.id.starts_with("user-home");
        let (icon_code, icon_color) = if is_home {
            (ICON_HOME, TEXT_PRIMARY)
        } else {
            match drive.medium {
                MediumKind::NvmeSsd => (ICON_BOLT, COLOR_CYAN),
                MediumKind::SataSsd => (ICON_BOLT, COLOR_PRIMARY),
                MediumKind::Hdd => (ICON_HDD, COLOR_WARNING),
                MediumKind::Removable => (ICON_USB, COLOR_SUCCESS),
                MediumKind::Network => (ICON_GLOBE, COLOR_PRIMARY),
                MediumKind::Unknown => (ICON_DATABASE, TEXT_PRIMARY),
            }
        };

        let icon_box = container(icon_colored(icon_code, icon_color).size(15))
            .width(Length::Fixed(36.0))
            .height(Length::Fixed(36.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(icon_badge_container);

        // Nama & Subtitle Path
        let display_name = if is_home {
            "Home Folder".to_string()
        } else if drive.mount_point == std::path::Path::new("/") {
            if drive.label.contains('(') {
                drive.label.clone()
            } else {
                format!("Root Partition ({})", drive.label)
            }
        } else {
            drive.label.clone()
        };

        let path_str = drive.mount_point.display().to_string();

        let pct = drive.usage_percentage() as f32;

        let mut left_info = column![
            text(display_name)
                .size(13)
                .color(TEXT_PRIMARY),
            text(path_str)
                .size(11)
                .color(TEXT_MUTED),
        ]
        .spacing(2);

        // Jika ada kapasitas disk, tampilkan thin progress bar horizontal
        if drive.total_bytes > 0 {
            let bar = progress_bar(0.0..=1.0, pct)
                .height(4)
                .width(Length::Fixed(220.0))
                .style(dynamic_progress_bar_style(pct));

            left_info = left_info.push(Space::with_height(Length::Fixed(3.0)));
            left_info = left_info.push(bar);
        }

        let left_section = row![
            icon_box,
            Space::with_width(Length::Fixed(12.0)),
            left_info,
        ]
        .align_y(iced::Alignment::Center);

        // Info Kapasitas Kanan (Available & Total)
        let right_section = if drive.total_bytes > 0 {
            column![
                text(format!("{} Available", format_bytes(drive.available_bytes)))
                    .size(12)
                    .color(TEXT_SECONDARY),
                text(format!("{} Total", format_bytes(drive.total_bytes)))
                    .size(11)
                    .color(TEXT_MUTED),
            ]
            .spacing(1)
            .align_x(iced::Alignment::End)
        } else {
            column![]
        };

        let chevron = icon_colored(ICON_CHEVRON_RIGHT, TEXT_MUTED).size(13);

        let row_content = row![
            left_section,
            Space::with_width(Length::Fill),
            right_section,
            Space::with_width(Length::Fixed(12.0)),
            chevron,
        ]
        .align_y(iced::Alignment::Center)
        .padding([10, 14]);

        let row_btn = button(row_content)
            .width(Length::Fill)
            .style(drive_row_button)
            .on_press(on_select_drive(drive.clone()));

        list_col = list_col.push(row_btn);

        // Divider antar baris
        if idx + 1 < total_drives {
            let divider = container(Space::with_height(Length::Fixed(1.0)))
                .width(Length::Fill)
                .style(divider_style);
            list_col = list_col.push(divider);
        }
    }

    let boxed_list = container(list_col)
        .width(Length::Fill)
        .style(boxed_list_container);

    let content_box = column![
        header_title,
        Space::with_height(Length::Fixed(20.0)),
        section_label,
        Space::with_height(Length::Fixed(6.0)),
        boxed_list,
    ]
    .spacing(2)
    .width(Length::Fixed(560.0))
    .align_x(iced::Alignment::Start);

    container(content_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}
