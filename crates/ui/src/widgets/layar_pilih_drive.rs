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
    // Section Header (This Device)
    let section_label = text("This Device")
        .size(12)
        .color(TEXT_MUTED);

    // Grouped Boxed List (GNOME Disks / Baobab Style)
    let mut list_col = column![].width(Length::Fill);
    let total_drives = drives.len();

    for (idx, drive) in drives.iter().enumerate() {
        let is_home = drive.label.starts_with("Home") || drive.id.starts_with("user-home");
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

        let display_name = if is_home {
            "Home Folder".to_string()
        } else {
            drive.label.clone()
        };

        let path_str = drive.mount_point.display().to_string();
        let pct = drive.usage_percentage() as f32;

        let middle_section = if drive.total_bytes > 0 {
            column![
                row![
                    text(display_name)
                        .size(13)
                        .color(TEXT_PRIMARY),
                    Space::with_width(Length::Fill),
                    text(format!("{} Available", format_bytes(drive.available_bytes)))
                        .size(12)
                        .color(TEXT_SECONDARY),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    text(path_str)
                        .size(11)
                        .color(TEXT_MUTED),
                    Space::with_width(Length::Fill),
                    text(format!("{} Total", format_bytes(drive.total_bytes)))
                        .size(11)
                        .color(TEXT_MUTED),
                ]
                .align_y(iced::Alignment::Center),
                Space::with_height(Length::Fixed(4.0)),
                progress_bar(0.0..=1.0, pct)
                    .height(4)
                    .width(Length::Fill)
                    .style(dynamic_progress_bar_style(pct)),
            ]
            .spacing(2)
            .width(Length::Fill)
        } else {
            column![
                text(display_name)
                    .size(13)
                    .color(TEXT_PRIMARY),
                text(path_str)
                    .size(11)
                    .color(TEXT_MUTED),
            ]
            .spacing(2)
            .width(Length::Fill)
        };

        let chevron = icon_colored(ICON_CHEVRON_RIGHT, TEXT_MUTED).size(13);

        let row_content = row![
            icon_box,
            Space::with_width(Length::Fixed(14.0)),
            middle_section,
            Space::with_width(Length::Fixed(14.0)),
            chevron,
        ]
        .align_y(iced::Alignment::Center)
        .padding([12, 16]);

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
        section_label,
        Space::with_height(Length::Fixed(8.0)),
        boxed_list,
    ]
    .width(Length::Fixed(580.0))
    .align_x(iced::Alignment::Start);

    container(content_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([48, 24])
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Top)
        .into()
}
