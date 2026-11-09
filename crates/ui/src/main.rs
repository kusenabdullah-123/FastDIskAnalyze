#![windows_subsystem = "windows"]

use fast_disk_analyze_ui::FastDiskApp;

fn main() -> iced::Result {
    FastDiskApp::run()
}
