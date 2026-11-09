use crate::theme::{dynamic_progress_bar_style, TEXT_SECONDARY};
use iced::widget::{progress_bar, row, text};
use iced::{Element, Length};

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_count(count: u32) -> String {
    let s = count.to_string();
    let mut result = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('.');
        }
        result.push(c);
    }
    result
}

/// Widget visual bar yang menunjukkan proporsi ukuran file/folder relatif terhadap induknya.
pub fn size_bar_widget<'a, Message: 'a>(fraction: f32) -> Element<'a, Message> {
    let clamped = fraction.clamp(0.0, 1.0);
    let pct_text = format!("{:>5.1}%", clamped * 100.0);

    let bar = progress_bar(0.0..=1.0, clamped)
        .height(6)
        .width(Length::Fixed(70.0))
        .style(dynamic_progress_bar_style(clamped));

    row![
        bar,
        text(pct_text).size(11).color(TEXT_SECONDARY),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center)
    .into()
}
