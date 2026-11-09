use iced::widget::text;
use iced::{Color, Font};

pub const ICONS_FONT_BYTES: &[u8] = include_bytes!("../resources/icons.ttf");
pub const ICONS_FONT: Font = Font::with_name("FontAwesome");

// Codepoints FontAwesome
pub const ICON_HOME: char = '\u{f015}';
pub const ICON_HDD: char = '\u{f0a0}';
pub const ICON_BOLT: char = '\u{f0e7}';
pub const ICON_USB: char = '\u{f287}';
pub const ICON_GLOBE: char = '\u{f0ac}';
pub const ICON_DATABASE: char = '\u{f1c0}';
pub const ICON_DESKTOP: char = '\u{f108}';
pub const ICON_FOLDER: char = '\u{f07b}';
pub const ICON_FOLDER_OPEN: char = '\u{f07c}';
pub const ICON_FILE: char = '\u{f15b}';
pub const ICON_SEARCH: char = '\u{f002}';
pub const ICON_CHEVRON_RIGHT: char = '\u{f054}';
pub const ICON_CHEVRON_LEFT: char = '\u{f053}';
pub const ICON_CHEVRON_UP: char = '\u{f077}';
pub const ICON_ARROW_LEFT: char = '\u{f060}';
pub const ICON_ARROW_RIGHT: char = '\u{f061}';
pub const ICON_ARROW_UP: char = '\u{f062}';
pub const ICON_SPINNER: char = '\u{f110}';
pub const ICON_REFRESH: char = '\u{f021}';
pub const ICON_TIMES: char = '\u{f00d}';
pub const ICON_BAN: char = '\u{f05e}';
pub const ICON_CHECK: char = '\u{f00c}';
pub const ICON_TRASH: char = '\u{f1f8}';
pub const ICON_SORT_UP: char = '\u{f0de}';
pub const ICON_SORT_DOWN: char = '\u{f0dd}';
pub const ICON_SORT_DEFAULT: char = '\u{f0dc}';
pub const ICON_ROCKET: char = '\u{f135}';
pub const ICON_TACHOMETER: char = '\u{f0e4}';
pub const ICON_CLOCK: char = '\u{f017}';
pub const ICON_PIE_CHART: char = '\u{f200}';

pub fn icon(ch: char) -> iced::widget::Text<'static> {
    text(ch.to_string()).font(ICONS_FONT)
}

pub fn icon_colored(ch: char, color: Color) -> iced::widget::Text<'static> {
    text(ch.to_string()).font(ICONS_FONT).color(color)
}
