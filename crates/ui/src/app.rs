use crate::view_state::{SortColumn, ViewState};
use crate::widgets::{
    view_dialog_hapus, view_explorer, view_loading, view_pilih_drive, view_scanning,
};
use crossbeam_channel::Receiver;
use fast_disk_analyze_core::engine::penghapus::{ModeHapus, Penghapus};
use fast_disk_analyze_core::engine::pengendali::{
    AkurasiTautan, CapPolicy, DefaultPengendaliScan, HasilLengkap, HasilScan, PengendaliScan,
    PermintaanScan, ProgressEvent,
};
use fast_disk_analyze_core::view::SizeMode;
use fast_disk_analyze_core::{IdEntri, PohonSistemBerkas};
use fast_disk_analyze_platform::{
    default_fs_ops, default_probe, CancellationToken, InformasiDrive, MediumKind,
};
use iced::{Element, Subscription, Task};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum LayarAplikasi {
    Loading,
    PilihDrive(Vec<InformasiDrive>),
    Memindai {
        drive: InformasiDrive,
        items: u64,
        bytes: u64,
        elapsed_ms: u64,
    },
    Explorer {
        drive: InformasiDrive,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    // Fase 1: Loading & Deteksi Drive
    DrivesFound(Vec<InformasiDrive>),

    // Fase 2: Pilih Drive
    SelectDrive(InformasiDrive),
    OpenFolderDialog,
    FolderSelected(PathBuf),
    DialogCancelled,

    // Fase 3: Pemindaian
    PollScanProgress,
    CancelScan,

    // Fase 4: Explorer
    BackToDriveList,
    NavigateTo(IdEntri),
    NavigateUp,
    SelectNode(IdEntri),
    ToggleSort(SortColumn),
    ToggleSizeMode,
    FilterChanged(String),
    ScrollTable(iced::widget::scrollable::Viewport),

    // Fase 5: Rescan & Penghapusan ke Trash (M5)
    RescanSubtree,
    SubtreeScanComplete(IdEntri, Box<HasilLengkap>),
    PromptDeleteToTrash(IdEntri),
    ConfirmDeleteToTrash,
    CancelDelete,
    OpenContextMenu(IdEntri),
    CloseContextMenu,
}

pub struct FastDiskApp {
    pub layar: LayarAplikasi,
    pub state: ViewState,
    pub known_drives: Vec<InformasiDrive>,
    pub model: Option<PohonSistemBerkas>,
    pub cancel_token: Option<CancellationToken>,
    pub scan_rx: Option<Arc<Receiver<ProgressEvent>>>,
    pub confirm_delete_target: Option<IdEntri>,
}

impl FastDiskApp {
    pub fn new() -> (Self, Task<Message>) {
        let app = Self {
            layar: LayarAplikasi::Loading,
            state: ViewState::new(),
            known_drives: Vec::new(),
            model: None,
            cancel_token: None,
            scan_rx: None,
            confirm_delete_target: None,
        };

        // Deteksi perangkat penyimpanan di latar belakang saat aplikasi pertama kali dibuka
        let task = Task::future(async {
            let probe = default_probe();
            probe.list_drives()
        })
        .map(Message::DrivesFound);

        (app, task)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DrivesFound(drives) => {
                self.known_drives = drives.clone();
                self.layar = LayarAplikasi::PilihDrive(drives);
                Task::none()
            }
            Message::SelectDrive(drive) => {
                let target = drive.mount_point.clone();
                let medium = drive.medium;

                self.layar = LayarAplikasi::Memindai {
                    drive: drive.clone(),
                    items: 0,
                    bytes: 0,
                    elapsed_ms: 0,
                };

                let req = PermintaanScan {
                    root_path: target,
                    medium,
                    mode: self.state.size_mode,
                    cap_policy: CapPolicy::None,
                    akurasi_tautan: AkurasiTautan::Cepat,
                    lintas_mount: false,
                };

                let mut pengendali = DefaultPengendaliScan::new();
                let rx = pengendali.mulai(req);
                self.cancel_token = Some(pengendali.token().clone());
                self.scan_rx = Some(Arc::new(rx));

                Task::none()
            }
            Message::OpenFolderDialog => {
                Task::future(async {
                    let folder = rfd::AsyncFileDialog::new().pick_folder().await;
                    folder.map(|h| h.path().to_path_buf())
                })
                .map(|opt| {
                    if let Some(p) = opt {
                        Message::FolderSelected(p)
                    } else {
                        Message::DialogCancelled
                    }
                })
            }
            Message::FolderSelected(path) => {
                let probe = default_probe();
                let medium = probe.probe(&path);
                let label = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());

                let custom_drive = InformasiDrive {
                    id: path.display().to_string(),
                    mount_point: path,
                    label: format!("Folder ({label})"),
                    fs_type: "Custom".to_string(),
                    total_bytes: 0,
                    available_bytes: 0,
                    medium,
                };

                self.update(Message::SelectDrive(custom_drive))
            }
            Message::DialogCancelled => Task::none(),
            Message::CancelScan => {
                if let Some(token) = &self.cancel_token {
                    token.cancel();
                }
                self.scan_rx = None;
                self.cancel_token = None;
                self.layar = LayarAplikasi::PilihDrive(self.known_drives.clone());
                Task::none()
            }
            Message::PollScanProgress => {
                let mut finished_data = None;

                if let Some(rx) = &self.scan_rx {
                    while let Ok(evt) = rx.try_recv() {
                        match evt {
                            ProgressEvent::Tick {
                                items: it,
                                bytes: by,
                                elapsed_ms: el,
                                ..
                            } => {
                                if let LayarAplikasi::Memindai {
                                    items,
                                    bytes,
                                    elapsed_ms,
                                    ..
                                } = &mut self.layar
                                {
                                    *items = it;
                                    *bytes = by;
                                    *elapsed_ms = el;
                                }
                            }
                            ProgressEvent::Finished(hasil) => {
                                match hasil {
                                    HasilScan::Complete(res) => {
                                        finished_data = Some(res);
                                    }
                                    HasilScan::Incomplete { result, .. } => {
                                        finished_data = Some(result);
                                    }
                                }
                                break;
                            }
                            ProgressEvent::Cancelled { .. } => {
                                self.layar = LayarAplikasi::PilihDrive(self.known_drives.clone());
                                self.scan_rx = None;
                                self.cancel_token = None;
                                return Task::none();
                            }
                            ProgressEvent::Failed { error: _ } => {
                                self.layar = LayarAplikasi::PilihDrive(self.known_drives.clone());
                                self.scan_rx = None;
                                self.cancel_token = None;
                                return Task::none();
                            }
                            _ => {}
                        }
                    }
                }

                if let Some(lengkap) = finished_data {
                    let drive = if let LayarAplikasi::Memindai { drive, .. } = &self.layar {
                        drive.clone()
                    } else {
                        InformasiDrive {
                            id: "root".to_string(),
                            mount_point: PathBuf::from("/"),
                            label: "Drive".to_string(),
                            fs_type: "Disk".to_string(),
                            total_bytes: 0,
                            available_bytes: 0,
                            medium: MediumKind::Unknown,
                        }
                    };

                    let pohon = PohonSistemBerkas::baru(lengkap.root_path.clone(), lengkap.gudang, lengkap.root, lengkap.errors);
                    self.state.current_dir = Some(IdEntri(0));
                    self.state.nav_stack.clear();
                    self.state.selected_node = None;
                    self.state.context_menu_target = None;
                    self.state.scroll_offset_y = 0.0;
                    self.model = Some(pohon);
                    self.scan_rx = None;
                    self.cancel_token = None;
                    self.confirm_delete_target = None;
                    self.layar = LayarAplikasi::Explorer { drive };
                }

                Task::none()
            }
            Message::BackToDriveList => {
                self.model = None;
                self.state.context_menu_target = None;
                self.confirm_delete_target = None;
                self.layar = LayarAplikasi::PilihDrive(self.known_drives.clone());
                Task::none()
            }
            Message::NavigateTo(id) => {
                self.state.navigate_to(id);
                Task::none()
            }
            Message::NavigateUp => {
                self.state.navigate_up();
                Task::none()
            }
            Message::SelectNode(id) => {
                self.state.selected_node = Some(id);
                Task::none()
            }
            Message::ToggleSort(col) => {
                self.state.toggle_sort(col);
                Task::none()
            }
            Message::ToggleSizeMode => {
                self.state.size_mode = match self.state.size_mode {
                    SizeMode::UkuranBlok => SizeMode::UkuranAsli,
                    SizeMode::UkuranAsli => SizeMode::UkuranBlok,
                };
                Task::none()
            }
            Message::FilterChanged(q) => {
                self.state.filter_query = q;
                self.state.scroll_offset_y = 0.0;
                Task::none()
            }
            Message::ScrollTable(viewport) => {
                let offset_y = viewport.absolute_offset().y;
                let height = viewport.bounds().height;
                self.state.scroll_offset_y = offset_y;
                self.state.viewport_height = height;
                Task::none()
            }
            Message::RescanSubtree => {
                if let (Some(model), Some(curr_dir)) = (&self.model, self.state.current_dir) {
                    let path = model.fs_path_verbatim(curr_dir).0;
                    let (tx, rx) = crossbeam_channel::bounded(64);
                    let token = CancellationToken::new();
                    let (async_tx, async_rx) = iced::futures::channel::oneshot::channel();

                    std::thread::spawn(move || {
                        let _ = fast_disk_analyze_core::engine::pemindai::Pemindai::scan_parallel(
                            &path,
                            MediumKind::Unknown,
                            4,
                            &token,
                            Some(tx.clone()),
                        );
                        while let Ok(evt) = rx.recv() {
                            if let ProgressEvent::Finished(hasil) = evt {
                                match hasil {
                                    HasilScan::Complete(res) => {
                                        let _ = async_tx.send(Some((curr_dir, res)));
                                        return;
                                    }
                                    HasilScan::Incomplete { result, .. } => {
                                        let _ = async_tx.send(Some((curr_dir, result)));
                                        return;
                                    }
                                }
                            }
                        }
                        let _ = async_tx.send(None);
                    });

                    Task::future(async move {
                        match async_rx.await {
                            Ok(Some((id, lengkap))) => {
                                Message::SubtreeScanComplete(id, Box::new(lengkap))
                            }
                            _ => Message::DialogCancelled,
                        }
                    })
                } else {
                    Task::none()
                }
            }
            Message::SubtreeScanComplete(target_id, lengkap) => {
                if let Some(model) = &mut self.model {
                    model.terapkan_subtree(target_id, lengkap.gudang, lengkap.root);
                }
                Task::none()
            }
            Message::OpenContextMenu(id) => {
                self.state.context_menu_target = Some(id);
                Task::none()
            }
            Message::CloseContextMenu => {
                self.state.context_menu_target = None;
                Task::none()
            }
            Message::PromptDeleteToTrash(id) => {
                self.state.context_menu_target = None;
                self.confirm_delete_target = Some(id);
                Task::none()
            }
            Message::CancelDelete => {
                self.confirm_delete_target = None;
                Task::none()
            }
            Message::ConfirmDeleteToTrash => {
                if let Some(target_id) = self.confirm_delete_target.take() {
                    if let Some(model) = &mut self.model {
                        let ops = default_fs_ops();
                        let hasil = Penghapus::eksekusi(
                            model,
                            &[target_id],
                            ModeHapus::Sampah,
                            ops.as_ref(),
                        );
                        model.terapkan_penghapusan(&hasil);

                        if self.state.selected_node == Some(target_id) {
                            self.state.selected_node = None;
                        }

                        if self.state.current_dir == Some(target_id) {
                            self.state.navigate_up();
                        }
                    }
                }
                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if matches!(self.layar, LayarAplikasi::Memindai { .. }) {
            iced::time::every(std::time::Duration::from_millis(30))
                .map(|_| Message::PollScanProgress)
        } else {
            Subscription::none()
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = match &self.layar {
            LayarAplikasi::Loading => view_loading(),
            LayarAplikasi::PilihDrive(drives) => {
                view_pilih_drive(drives, Message::SelectDrive)
            }
            LayarAplikasi::Memindai {
                drive,
                items,
                bytes,
                elapsed_ms,
            } => view_scanning(
                &drive.label,
                *items,
                *bytes,
                *elapsed_ms,
                Message::CancelScan,
            ),
            LayarAplikasi::Explorer { drive } => {
                if let Some(model) = &self.model {
                    let explorer_view = view_explorer(
                        drive,
                        model,
                        &self.state,
                        Message::BackToDriveList,
                        Message::NavigateTo,
                        Message::NavigateUp,
                        Message::SelectNode,
                        Message::ToggleSort,
                        Message::ScrollTable,
                        Message::RescanSubtree,
                        Message::PromptDeleteToTrash,
                        Message::OpenContextMenu,
                        Message::CloseContextMenu,
                    );

                    if let Some(target_id) = self.confirm_delete_target {
                        iced::widget::stack![
                            explorer_view,
                            view_dialog_hapus(
                                model,
                                target_id,
                                Message::ConfirmDeleteToTrash,
                                Message::CancelDelete,
                            ),
                        ]
                        .into()
                    } else {
                        explorer_view
                    }
                } else {
                    view_loading()
                }
            }
        };

        iced::widget::container(content)
            .style(crate::theme::app_background)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    }

    pub fn run() -> iced::Result {
        iced::application("Fast Disk Analyze", Self::update, Self::view)
            .subscription(Self::subscription)
            .window_size(iced::Size::new(1080.0, 720.0))
            .theme(|_| iced::Theme::Dark)
            .font(crate::icons::ICONS_FONT_BYTES)
            .run_with(Self::new)
    }
}
