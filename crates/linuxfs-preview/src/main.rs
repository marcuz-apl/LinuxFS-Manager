#![cfg_attr(windows, windows_subsystem = "windows")]

slint::slint! {
    import { Button, VerticalBox, HorizontalBox, LineEdit, ListView } from "std-widgets.slint";

    export component MainWindow inherits Window {
        title: "LinuxFS Manager";
        width: 1200px;
        height: 820px;
        preferred-width: 1200px;
        preferred-height: 820px;
        background: #f5f8fc;
        icon: @image-url("../../../assets/linuxfs-manager.png");

        in-out property <string> status: "Ready.";
        in-out property <string> engine_status: "WinFsp engine: checking…";
        in-out property <string> app_version: "";
        in-out property <string> source_name: "No source loaded";
        in-out property <string> source_details: "Open a raw Linux filesystem image to inspect it.";
        in-out property <string> image_path: "";
        in-out property <bool> can_mount: false;
        in-out property <bool> can_unmount: false;
        in-out property <int> selected_source: -1;
        in-out property <[string]> source_names: [];
        callback mount_clicked();
        callback unmount_clicked();
        callback open_explorer_clicked();
        callback details_clicked();
        callback refresh_clicked();
        callback scan_drives_clicked();
        callback open_image_clicked();
        callback source_selected(int);

        VerticalBox {
            padding: 20px;
            spacing: 16px;

            HorizontalBox {
                height: 58px;
                spacing: 13px;

                Image {
                    width: 44px;
                    height: 44px;
                    source: @image-url("../../../assets/linuxfs-manager.png");
                    image-fit: contain;
                }

                VerticalBox {
                    spacing: 2px;
                    Text { text: "LinuxFS Manager"; font-size: 25px; font-weight: 700; color: #102a43; }
                    Text { text: "Read Linux filesystems safely on Windows"; font-size: 13px; color: #58718a; }
                }
                Rectangle { horizontal-stretch: 1; }
                Button { width: 112px; text: "Scan Drives"; clicked => { root.scan_drives_clicked(); } }
                Button { width: 122px; text: "Open Image…"; clicked => { root.open_image_clicked(); } }
                Button { width: 82px; text: "About"; clicked => { about_popup.show(); } }
            }

            Rectangle {
                vertical-stretch: 1;
                background: #ffffff;
                border-radius: 14px;
                border-width: 1px;
                border-color: #d9e4ee;

                HorizontalBox {
                    spacing: 0px;

                    Rectangle {
                        width: 316px;
                        background: #102a43;
                        border-radius: 14px;

                        VerticalBox {
                            padding: 20px;
                            spacing: 8px;
                            Text { text: "Sources"; font-size: 19px; font-weight: 700; color: #ffffff; }
                            Text { text: "Partitions and image files"; font-size: 12px; color: #aec6d9; }
                            Rectangle { height: 1px; background: #2a4963; }
                            Rectangle { height: 4px; }

                            if (root.source_names.length == 0) : Text {
                                text: "Scan your drives or open an image to begin.";
                                color: #b8ccdc;
                                wrap: word-wrap;
                            }

                            ListView {
                                vertical-stretch: 1;
                                for name[index] in root.source_names : Rectangle {
                                    height: 46px;
                                    border-radius: 8px;
                                    background: root.selected_source == index ? #1e5c88 : #ffffff00;
                                    Text {
                                        x: 12px;
                                        width: parent.width - 24px;
                                        text: name;
                                        color: root.selected_source == index ? #ffffff : #d3e2ee;
                                        vertical-alignment: center;
                                        overflow: elide;
                                    }
                                    TouchArea { clicked => { root.source_selected(index); } }
                                }
                            }
                        }
                    }

                    VerticalBox {
                        horizontal-stretch: 1;
                        padding: 28px;
                        spacing: 18px;

                        Text { text: source_name; font-size: 24px; font-weight: 700; color: #102a43; overflow: elide; }

                        Rectangle {
                            vertical-stretch: 1;
                            background: #f4f8fb;
                            border-radius: 12px;
                            border-width: 1px;
                            border-color: #e2ebf2;

                            VerticalBox {
                                padding: 20px;
                                spacing: 9px;
                                Text { text: "Filesystem details"; font-size: 15px; font-weight: 700; color: #173b57; }
                                Text { text: source_details; color: #4f687d; wrap: word-wrap; }
                                Rectangle { vertical-stretch: 1; }
                            }
                        }

                        VerticalBox {
                            spacing: 7px;
                            Text { text: "Open a filesystem image"; font-size: 14px; font-weight: 700; color: #173b57; }
                            LineEdit { height: 40px; text <=> root.image_path; placeholder-text: "Image path (or use Open Image…)"; }
                        }

                        HorizontalBox {
                            height: 42px;
                            spacing: 10px;
                            Button { width: 118px; primary: true; text: "Mount"; enabled: root.can_mount; clicked => { root.mount_clicked(); } }
                            Button { width: 118px; text: "Unmount"; enabled: root.can_unmount; clicked => { root.unmount_clicked(); } }
                            Button { width: 172px; text: "Open in Explorer"; enabled: root.can_unmount; clicked => { root.open_explorer_clicked(); } }
                            Button { width: 118px; text: "Details"; clicked => { root.details_clicked(); } }
                            Rectangle { horizontal-stretch: 1; }
                        }
                    }
                }
            }

            Rectangle {
                height: 66px;
                background: #102a43;
                border-radius: 12px;

                Rectangle {
                    x: 18px;
                    y: 22px;
                    width: 9px;
                    height: 22px;
                    background: #48b981;
                    border-radius: 5px;
                }
                Text {
                    x: 42px;
                    y: 8px;
                    text: "READ ONLY — source filesystems are never modified.";
                    color: #ffffff;
                    font-size: 13px;
                    font-weight: 700;
                }
                Text {
                    x: 42px;
                    y: 34px;
                    width: parent.width - 62px;
                    text: engine_status + "  ·  " + status;
                    color: #b7cede;
                    font-size: 12px;
                    overflow: elide;
                }
            }
        }

        about_popup := PopupWindow {
            width: 560px;
            height: 420px;
            x: (root.width - self.width) / 2;
            y: (root.height - self.height) / 2;
            close-policy: PopupClosePolicy.no-auto-close;

            Rectangle {
                width: 100%;
                height: 100%;
                background: #ffffff;
                border-radius: 14px;
                border-width: 1px;
                border-color: #dce6f0;

                VerticalBox {
                    padding: 28px;
                    spacing: 18px;

                    HorizontalBox {
                        height: 86px;
                        spacing: 18px;
                        Image {
                            width: 86px;
                            height: 86px;
                            source: @image-url("../../../assets/linuxfs-manager.png");
                            image-fit: contain;
                        }
                        VerticalBox {
                            spacing: 4px;
                            Rectangle { vertical-stretch: 1; }
                            Text { text: "LinuxFS Manager"; font-size: 24px; font-weight: 700; color: #17324d; }
                            Text { text: "Version " + root.app_version; color: #64788f; }
                            Rectangle { vertical-stretch: 1; }
                        }
                    }

                    Rectangle { height: 1px; background: #e5edf5; }

                    Text {
                        text: "LinuxFS Manager provides safe, read-only access to Ext2/3/4, SquashFS, and supported XFS images from Windows physical disks, partitions, and raw image files.";
                        color: #405a72;
                        wrap: word-wrap;
                    }

                    Rectangle { vertical-stretch: 1; }

                    Text {
                        text: "LinuxFS Manager, @2026 Alfazen Inc. All rights reserved.";
                        color: #6b7c93;
                        horizontal-alignment: center;
                    }

                    HorizontalBox {
                        height: 40px;
                        Rectangle { horizontal-stretch: 1; }
                        Button { width: 100px; text: "Close"; clicked => { about_popup.close(); } }
                    }
                }
            }
        }
    }

    export component PrerequisiteWindow inherits Window {
        title: "LinuxFS Manager — WinFsp required";
        width: 680px;
        height: 470px;
        preferred-width: 680px;
        preferred-height: 470px;
        background: #f5f8fc;
        icon: @image-url("../../../assets/linuxfs-manager.png");

        in-out property <string> requirement_message: "WinFsp is required before LinuxFS Manager can open.";
        callback download_clicked();
        callback recheck_clicked();
        callback close_clicked();

        Rectangle {
            x: 0px;
            y: 0px;
            width: 100%;
            height: 100%;
            background: #f5f8fc;

            VerticalBox {
                padding: 34px;
                spacing: 18px;

                HorizontalBox {
                    height: 72px;
                    spacing: 16px;
                    Image {
                        width: 68px;
                        height: 68px;
                        source: @image-url("../../../assets/linuxfs-manager.png");
                        image-fit: contain;
                    }
                    VerticalBox {
                        spacing: 3px;
                        Rectangle { vertical-stretch: 1; }
                        Text { text: "WinFsp is required"; font-size: 25px; font-weight: 700; color: #17324d; }
                        Text { text: "A Windows filesystem framework prerequisite"; font-size: 13px; color: #61758b; }
                        Rectangle { vertical-stretch: 1; }
                    }
                }

                Rectangle {
                    height: 1px;
                    background: #dce6f0;
                }

                Text {
                    text: requirement_message;
                    color: #244c70;
                    font-size: 15px;
                    wrap: word-wrap;
                }

                Rectangle {
                    background: #eaf3ff;
                    border-radius: 12px;
                    border-width: 1px;
                    border-color: #cfe3f8;
                    vertical-stretch: 1;
                    VerticalBox {
                        padding: 18px;
                        spacing: 7px;
                        Text { text: "To continue"; font-size: 15px; font-weight: 700; color: #1d568b; }
                        Text { text: "1. Download WinFsp from its official release page."; color: #365a7c; }
                        Text { text: "2. Run the MSI installer and accept its driver installation."; color: #365a7c; }
                        Text { text: "3. Return here and select Recheck."; color: #365a7c; }
                    }
                }

                Text {
                    text: "LinuxFS Manager does not download, install, or modify WinFsp for you.";
                    color: #687b90;
                    font-size: 12px;
                    wrap: word-wrap;
                }

                HorizontalBox {
                    height: 42px;
                    spacing: 10px;
                    Button { width: 122px; text: "Close"; clicked => { root.close_clicked(); } }
                    Rectangle { horizontal-stretch: 1; }
                    Button { width: 136px; text: "Download WinFsp"; clicked => { root.download_clicked(); } }
                    Button { width: 100px; text: "Recheck"; clicked => { root.recheck_clicked(); } }
                }
            }
        }
    }
}

#[cfg(windows)]
fn source_items(sources: &[linuxfs_app::SourceViewModel]) -> slint::ModelRc<slint::SharedString> {
    use slint::{ModelRc, SharedString, VecModel};
    use std::rc::Rc;

    let items = sources
        .iter()
        .map(|source| SharedString::from(source.display_name.clone()))
        .collect::<Vec<_>>();
    ModelRc::from(Rc::new(VecModel::from(items)))
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn center_window(window: &slint::Window) {
    use slint::{PhysicalPosition, WindowPosition};
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SPI_GETWORKAREA, SystemParametersInfoW,
    };

    let window_size = window.size();
    let mut work_area = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: `work_area` is a valid writable RECT for the documented
    // SPI_GETWORKAREA query; Windows writes only within that structure.
    let got_work_area = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            (&mut work_area as *mut RECT).cast::<c_void>(),
            0,
        )
    } != 0;
    if !got_work_area {
        // SAFETY: GetSystemMetrics reads process-independent display metrics
        // and does not require caller-owned pointers or mutable system state.
        work_area.right = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        work_area.bottom = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    }
    let (x, y) = linuxfs_preview::centered_window_position(
        (
            i32::try_from(window_size.width).unwrap_or(i32::MAX),
            i32::try_from(window_size.height).unwrap_or(i32::MAX),
        ),
        (
            work_area.left,
            work_area.top,
            work_area.right - work_area.left,
            work_area.bottom - work_area.top,
        ),
    );
    window.set_position(WindowPosition::Physical(PhysicalPosition::new(x, y)));
}

#[cfg(windows)]
fn center_main_window(window: &MainWindow) {
    center_window(window.window());
}

#[derive(Debug, PartialEq, Eq)]
struct PrerequisiteState {
    visible: bool,
    can_continue: bool,
    message: String,
}

impl PrerequisiteState {
    fn from_assessment(assessment: &linuxfs_winfsp::WinFspAssessment) -> Self {
        use linuxfs_winfsp::WinFspRequirement;

        let message = match assessment.requirement() {
            WinFspRequirement::Ready => "WinFsp is ready.".to_owned(),
            WinFspRequirement::UnsupportedPlatform => {
                "LinuxFS Manager requires Windows with the WinFsp framework.".to_owned()
            }
            WinFspRequirement::InstallationNotRegistered => {
                "WinFsp is not installed or is not registered with Windows.".to_owned()
            }
            WinFspRequirement::RuntimeDllMissing => {
                "The installed WinFsp runtime DLL for this computer is missing.".to_owned()
            }
            WinFspRequirement::LauncherNotInstalled => {
                "The WinFsp Launcher service is not installed.".to_owned()
            }
            WinFspRequirement::LauncherNotRunning => {
                "The WinFsp Launcher service is installed but is not running.".to_owned()
            }
            WinFspRequirement::LauncherStatusUnavailable => {
                "Windows could not verify the WinFsp Launcher service.".to_owned()
            }
            WinFspRequirement::RuntimeInitializationFailed => {
                "The installed WinFsp runtime could not be initialized.".to_owned()
            }
        };
        Self {
            visible: !assessment.is_ready(),
            can_continue: assessment.is_ready(),
            message,
        }
    }
}

fn engine_status_text(assessment: &linuxfs_winfsp::WinFspAssessment) -> String {
    use linuxfs_winfsp::WinFspRequirement;

    match assessment.requirement() {
        WinFspRequirement::Ready => "WinFsp engine: Ready — installed, launcher running".to_owned(),
        WinFspRequirement::InstallationNotRegistered => {
            "WinFsp engine: Unavailable — installation not registered".to_owned()
        }
        WinFspRequirement::RuntimeDllMissing => {
            "WinFsp engine: Unavailable — runtime DLL missing".to_owned()
        }
        WinFspRequirement::LauncherNotInstalled => {
            "WinFsp engine: Unavailable — launcher not installed".to_owned()
        }
        WinFspRequirement::LauncherNotRunning => {
            "WinFsp engine: Unavailable — launcher not running".to_owned()
        }
        WinFspRequirement::LauncherStatusUnavailable => {
            "WinFsp engine: Unavailable — launcher status unavailable".to_owned()
        }
        WinFspRequirement::RuntimeInitializationFailed => {
            "WinFsp engine: Unavailable — initialization failed".to_owned()
        }
        WinFspRequirement::UnsupportedPlatform => {
            "WinFsp engine: Unavailable — Windows required".to_owned()
        }
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn open_winfsp_download_page() -> Result<(), Box<dyn std::error::Error>> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let url: Vec<u16> = "https://winfsp.dev/rel/"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let operation: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: the operation and URL are owned, NUL-terminated UTF-16 buffers. ShellExecuteW
    // is used only to request the user's configured browser open an official web page.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if (result as isize) <= 32 {
        return Err("Windows could not open the official WinFsp release page".into());
    }
    Ok(())
}

#[cfg(windows)]
fn run_prerequisite_gate(
    initial_assessment: &linuxfs_winfsp::WinFspAssessment,
) -> Result<bool, Box<dyn std::error::Error>> {
    use std::sync::{Arc, Mutex};

    let window = PrerequisiteWindow::new()?;
    let initial_state = PrerequisiteState::from_assessment(initial_assessment);
    window.set_requirement_message(initial_state.message.into());
    window.show()?;
    center_window(window.window());

    let ready_to_continue = Arc::new(Mutex::new(false));
    let weak = window.as_weak();
    window.on_download_clicked(move || {
        if let Err(error) = open_winfsp_download_page()
            && let Some(window) = weak.upgrade()
        {
            window.set_requirement_message(
                format!("Could not open the download page: {error}").into(),
            );
        }
    });
    let weak = window.as_weak();
    let ready_for_recheck = Arc::clone(&ready_to_continue);
    window.on_recheck_clicked(move || {
        let assessment = linuxfs_winfsp::assess_winfsp();
        let _ = linuxfs_app::runtime::record_winfsp_assessment(&assessment);
        let state = PrerequisiteState::from_assessment(&assessment);
        if state.can_continue {
            if let Ok(mut ready) = ready_for_recheck.lock() {
                *ready = true;
            }
            if let Some(window) = weak.upgrade() {
                let _ = window.hide();
            }
        } else if let Some(window) = weak.upgrade() {
            window.set_requirement_message(state.message.into());
        }
    });
    let weak = window.as_weak();
    window.on_close_clicked(move || {
        if let Some(window) = weak.upgrade() {
            let _ = window.hide();
        }
    });
    window.run()?;
    Ok(ready_to_continue
        .lock()
        .map(|ready| *ready)
        .unwrap_or(false))
}

#[derive(Debug, PartialEq, Eq)]
struct UiState {
    image_path: String,
    source_name: String,
    source_details: String,
    status: String,
    can_mount: bool,
    can_unmount: bool,
}

impl UiState {
    fn new(path: &str) -> Self {
        Self {
            image_path: path.to_owned(),
            source_name: "No source loaded".to_owned(),
            source_details: "Open a raw Linux filesystem image to inspect it.".to_owned(),
            status: "Ready.".to_owned(),
            can_mount: false,
            can_unmount: false,
        }
    }

    fn validate_path(path: &str) -> Result<(), &'static str> {
        if path.trim().is_empty() {
            Err("provide an image path")
        } else {
            Ok(())
        }
    }

    fn set_compatible(&mut self, filesystem: &str, description: &str) {
        self.source_details =
            format!("{filesystem} · {description} · Compatible · Read-only source");
        self.can_mount = true;
        self.can_unmount = false;
    }

    fn set_source(&mut self, source: &linuxfs_app::SourceViewModel) {
        let filesystem = source.filesystem_type.as_deref().unwrap_or("Unknown");
        self.source_name = source.display_name.clone();
        self.source_details = format!(
            "{filesystem} · {} · Read-only source",
            source.source_description
        );
        self.can_mount = source.can_mount();
        self.can_unmount = source.can_unmount();
    }

    fn set_mounted(&mut self, point: &str) {
        self.status = format!("Mounted read-only on {point} — source unchanged");
        self.can_mount = false;
        self.can_unmount = true;
    }

    fn set_unmounted(&mut self) {
        self.status = "Unmount completed".to_owned();
        self.can_mount = true;
        self.can_unmount = false;
    }
}

#[cfg(not(windows))]
fn main() -> Result<(), slint::PlatformError> {
    MainWindow::new()?.run()
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use linuxfs_app::{
        MountService, SourceProvider, SourceViewModel, WindowsImageMountService,
        WindowsSourceProvider,
        runtime::{BackgroundOperation, load_config, spawn_background},
    };
    use std::{
        env,
        sync::{Arc, Mutex},
        time::Duration,
    };

    enum PendingOperation {
        Probe(BackgroundOperation<SourceViewModel>, String),
        Refresh(BackgroundOperation<Vec<SourceViewModel>>),
        Mount(BackgroundOperation<String>, linuxfs_app::SourceId),
        Unmount(BackgroundOperation<()>, linuxfs_app::SourceId),
    }
    #[allow(clippy::large_enum_variant)]
    enum CompletedOperation {
        Probe(Result<(SourceViewModel, String), String>),
        Refresh(Result<Vec<SourceViewModel>, String>),
        Mount(Result<(linuxfs_app::SourceId, String), String>),
        Unmount(Result<linuxfs_app::SourceId, String>),
    }

    let mut assessment = linuxfs_winfsp::assess_winfsp();
    let _ = linuxfs_app::runtime::record_winfsp_assessment(&assessment);
    if !assessment.is_ready() && !run_prerequisite_gate(&assessment)? {
        return Ok(());
    }
    assessment = linuxfs_winfsp::assess_winfsp();
    let _ = linuxfs_app::runtime::record_winfsp_assessment(&assessment);
    if !assessment.is_ready() {
        return Ok(());
    }

    let config = load_config().unwrap_or_default();
    let preferred_mount_point = config
        .preferred_drive_letter
        .as_deref()
        .filter(|letter| letter.len() == 1 && letter.as_bytes()[0].is_ascii_alphabetic())
        .map(|letter| format!("{letter}:"))
        .unwrap_or_default();
    let image = env::args_os()
        .nth(1)
        .map(|path| path.to_string_lossy().into_owned());
    let provider = Arc::new(Mutex::new(WindowsSourceProvider::new()));
    let service = Arc::new(Mutex::new(WindowsImageMountService::new(
        preferred_mount_point,
    )));
    linuxfs_winfsp::prepare_runtime()?;
    let _winfsp = winfsp::winfsp_init()?;
    let window = MainWindow::new()?;
    window.show()?;
    center_main_window(&window);
    window.set_app_version(env!("LINUXFS_MANAGER_VERSION").into());
    window.set_engine_status(engine_status_text(&assessment).into());
    let initial_path = image.unwrap_or_default();
    window.set_image_path(initial_path.clone().into());
    let state = Arc::new(Mutex::new(UiState::new(&initial_path)));
    let current_source = Arc::new(Mutex::new(None::<linuxfs_app::SourceViewModel>));
    let sources_for_ui = Arc::new(Mutex::new(Vec::<linuxfs_app::SourceViewModel>::new()));
    let pending = Arc::new(Mutex::new(None::<PendingOperation>));

    let start_probe: Arc<dyn Fn(String)> = {
        let provider = Arc::clone(&provider);
        let pending = Arc::clone(&pending);
        let weak = window.as_weak();
        Arc::new(move |path: String| {
            if let Err(error) = UiState::validate_path(&path) {
                if let Some(window) = weak.upgrade() {
                    window.set_status(format!("Refresh failed: {error}").into());
                }
                return;
            }
            let provider_for_operation = Arc::clone(&provider);
            let probe_path = path.clone();
            let operation = spawn_background(move || {
                provider_for_operation
                    .lock()
                    .expect("provider lock poisoned")
                    .open_image(&probe_path)
            });
            *pending.lock().expect("pending operation lock") =
                Some(PendingOperation::Probe(operation, path));
            if let Some(window) = weak.upgrade() {
                window.set_status("Opening image read-only…".into());
            }
        })
    };
    let start_refresh: Arc<dyn Fn()> = {
        let provider = Arc::clone(&provider);
        let pending = Arc::clone(&pending);
        let weak = window.as_weak();
        Arc::new(move || {
            let provider_for_operation = Arc::clone(&provider);
            let operation = spawn_background(move || {
                let mut provider = provider_for_operation.lock().map_err(|_| {
                    linuxfs_core::Error::new(
                        linuxfs_core::ErrorCategory::Internal,
                        "provider lock poisoned",
                    )
                })?;
                provider.refresh()
            });
            if let Ok(mut pending) = pending.lock() {
                *pending = Some(PendingOperation::Refresh(operation));
            }
            if let Some(window) = weak.upgrade() {
                window.set_status("Scanning physical sources read-only…".into());
            }
        })
    };
    let weak = window.as_weak();
    let state_for_refresh = Arc::clone(&state);
    let start_probe_for_refresh = start_probe.clone();
    let start_refresh_for_refresh = start_refresh.clone();
    window.on_refresh_clicked(move || {
        let path = state_for_refresh
            .lock()
            .map(|state| state.image_path.clone())
            .unwrap_or_default();
        if path.trim().is_empty() {
            start_refresh_for_refresh();
            return;
        }
        if let Some(window) = weak.upgrade() {
            window.set_status("Refreshing read-only image source…".into());
        }
        start_probe_for_refresh(path);
    });
    let start_refresh_for_scan = start_refresh.clone();
    window.on_scan_drives_clicked(move || {
        start_refresh_for_scan();
    });
    let state_for_open = Arc::clone(&state);
    let start_probe_for_open = start_probe.clone();
    window.on_open_image_clicked(move || {
        let dialog = rfd::FileDialog::new()
            .set_title("Open Linux filesystem image")
            .add_filter("Disk images", &["img", "raw", "dd", "iso", "bin"])
            .add_filter("All files", &["*"]);
        if let Some(path) = dialog.pick_file() {
            let path = path.to_string_lossy().into_owned();
            if let Ok(mut state) = state_for_open.lock() {
                state.image_path = path.clone();
            }
            start_probe_for_open(path);
        }
    });
    let source_rows = Arc::clone(&sources_for_ui);
    let current_source_for_selection = Arc::clone(&current_source);
    let state_for_selection = Arc::clone(&state);
    let weak = window.as_weak();
    window.on_source_selected(move |index| {
        let source = source_rows.lock().ok().and_then(|sources| {
            usize::try_from(index)
                .ok()
                .and_then(|index| sources.get(index).cloned())
        });
        let Some(source) = source else {
            return;
        };
        if let Some(window) = weak.upgrade() {
            window.set_selected_source(index);
        }
        if let Ok(mut ui) = state_for_selection.lock() {
            ui.set_source(&source);
            if let Some(window) = weak.upgrade() {
                window.set_source_name(ui.source_name.clone().into());
                window.set_source_details(ui.source_details.clone().into());
                window.set_can_mount(ui.can_mount);
                window.set_can_unmount(ui.can_unmount);
                window.set_status("Source selected; source remains read-only".into());
            }
        }
        if let Ok(mut current_source) = current_source_for_selection.lock() {
            *current_source = Some(source);
        }
    });
    let weak = window.as_weak();
    let source_slot = Arc::clone(&current_source);
    let service_for_mount = Arc::clone(&service);
    let pending_for_mount = Arc::clone(&pending);
    window.on_mount_clicked(move || {
        let source = source_slot
            .lock()
            .expect("source lock")
            .as_ref()
            .cloned()
            .ok_or_else(|| "no source loaded".to_owned());
        let source_id = match &source {
            Ok(source) => source.id,
            Err(_) => linuxfs_app::SourceId(0),
        };
        let service_for_operation = Arc::clone(&service_for_mount);
        let operation = match source {
            Ok(source) => spawn_background(move || {
                service_for_operation
                    .lock()
                    .expect("mount service lock poisoned")
                    .mount(&source)
            }),
            Err(error) => {
                if let Some(window) = weak.upgrade() {
                    window.set_status(format!("Mount failed: {error}").into());
                }
                return;
            }
        };
        *pending_for_mount.lock().expect("pending operation lock") =
            Some(PendingOperation::Mount(operation, source_id));
        if let Some(window) = weak.upgrade() {
            window.set_status("Mounting read-only…".into());
            window.set_can_mount(false);
            window.set_can_unmount(false);
        }
    });
    let weak = window.as_weak();
    let source_slot = Arc::clone(&current_source);
    let service_for_unmount = Arc::clone(&service);
    let pending_for_unmount = Arc::clone(&pending);
    window.on_unmount_clicked(move || {
        let source = source_slot
            .lock()
            .expect("source lock")
            .as_ref()
            .cloned()
            .ok_or_else(|| "no source loaded".to_owned());
        let source_id = match &source {
            Ok(source) => source.id,
            Err(_) => linuxfs_app::SourceId(0),
        };
        let service_for_operation = Arc::clone(&service_for_unmount);
        let operation = match source {
            Ok(source) => spawn_background(move || {
                service_for_operation
                    .lock()
                    .expect("mount service lock poisoned")
                    .unmount(&source)
            }),
            Err(error) => {
                if let Some(window) = weak.upgrade() {
                    window.set_status(format!("Unmount failed: {error}").into());
                }
                return;
            }
        };
        *pending_for_unmount.lock().expect("pending operation lock") =
            Some(PendingOperation::Unmount(operation, source_id));
        if let Some(window) = weak.upgrade() {
            window.set_status("Unmounting…".into());
            window.set_can_mount(false);
            window.set_can_unmount(false);
        }
    });
    let weak = window.as_weak();
    let source_slot = Arc::clone(&current_source);
    let service_for_explorer = Arc::clone(&service);
    window.on_open_explorer_clicked(move || {
        let Some(source) = source_slot.lock().ok().and_then(|source| source.clone()) else {
            if let Some(window) = weak.upgrade() {
                window.set_status("Explorer failed: no source loaded".into());
            }
            return;
        };
        let Some(mount_point) = source.mount_point.as_deref() else {
            if let Some(window) = weak.upgrade() {
                window.set_status("Explorer unavailable: source is not mounted".into());
            }
            return;
        };
        let result = service_for_explorer
            .lock()
            .map_err(|_| "mount service lock poisoned".to_owned())
            .and_then(|mut service| {
                service
                    .open_in_explorer(mount_point)
                    .map_err(|error| error.to_string())
            });
        if let Some(window) = weak.upgrade() {
            match result {
                Ok(()) => window.set_status(format!("Opened {mount_point} in Explorer").into()),
                Err(error) => window.set_status(format!("Explorer failed: {error}").into()),
            }
        }
    });
    let weak = window.as_weak();
    let state_for_details = Arc::clone(&state);
    window.on_details_clicked(move || {
        if let Some(window) = weak.upgrade() {
            window.set_status(
                state_for_details
                    .lock()
                    .expect("UI state lock")
                    .source_details
                    .clone()
                    .into(),
            );
        }
    });

    let timer = slint::Timer::default();
    let pending_for_timer = Arc::clone(&pending);
    let state_for_timer = Arc::clone(&state);
    let source_for_timer = Arc::clone(&current_source);
    let sources_for_timer = Arc::clone(&sources_for_ui);
    let weak_for_timer = window.as_weak();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(50),
        move || {
            let Some(mut operation) = pending_for_timer
                .lock()
                .expect("pending operation lock")
                .take()
            else {
                return;
            };
            let completed = match &mut operation {
                PendingOperation::Probe(operation, path) => operation.try_receive().map(|result| {
                    CompletedOperation::Probe(
                        result
                            .map(|source| (source, path.clone()))
                            .map_err(|error| error.to_string()),
                    )
                }),
                PendingOperation::Refresh(operation) => operation.try_receive().map(|result| {
                    CompletedOperation::Refresh(result.map_err(|error| error.to_string()))
                }),
                PendingOperation::Mount(operation, source_id) => operation.try_receive().map(|result| {
                    CompletedOperation::Mount(
                        result
                            .map(|point| (*source_id, point))
                            .map_err(|error| error.to_string()),
                    )
                }),
                PendingOperation::Unmount(operation, source_id) => operation.try_receive().map(|result| {
                    CompletedOperation::Unmount(
                        result.map(|()| *source_id).map_err(|error| error.to_string()),
                    )
                }),
            };
            if completed.is_none() {
                *pending_for_timer.lock().expect("pending operation lock") = Some(operation);
                return;
            }
            let completed = completed.expect("completed operation");
            if let Some(window) = weak_for_timer.upgrade() {
                match completed {
                    CompletedOperation::Probe(Ok((source, path))) => {
                        let filesystem = source
                            .filesystem_type
                            .clone()
                            .unwrap_or_else(|| "Unknown".to_owned());
                        let mut ui = state_for_timer.lock().expect("UI state lock");
                        ui.image_path = path;
                        ui.source_name = source.display_name.clone();
                        ui.set_compatible(&filesystem, &source.source_description);
                        window.set_source_name(ui.source_name.clone().into());
                        window.set_source_details(ui.source_details.clone().into());
                        window.set_can_mount(ui.can_mount);
                        window.set_can_unmount(ui.can_unmount);
                        if let Ok(mut sources) = sources_for_timer.lock() {
                            *sources = vec![source.clone()];
                            window.set_source_names(source_items(&sources));
                        }
                        window.set_selected_source(0);
                        *source_for_timer.lock().expect("source lock") = Some(source);
                        window.set_status("Source refreshed read-only".into());
                    }
                    CompletedOperation::Refresh(Ok(sources)) => {
                        if let Ok(mut rows) = sources_for_timer.lock() {
                            *rows = sources.clone();
                            window.set_source_names(source_items(&rows));
                        }
                        if let Some(source) = sources.first().cloned() {
                            let mut ui = state_for_timer.lock().expect("UI state lock");
                            ui.image_path.clear();
                            ui.set_source(&source);
                            window.set_image_path("".into());
                            window.set_source_name(ui.source_name.clone().into());
                            window.set_source_details(ui.source_details.clone().into());
                            window.set_can_mount(ui.can_mount);
                            window.set_can_unmount(ui.can_unmount);
                            window.set_selected_source(0);
                            *source_for_timer.lock().expect("source lock") = Some(source);
                            window.set_status("Physical sources refreshed read-only".into());
                        } else {
                            window.set_source_name("No compatible physical source".into());
                            window.set_source_details(
                                "No supported Linux filesystem was found, or Windows denied raw-disk access. Run elevated to scan physical disks.".into(),
                            );
                            window.set_can_mount(false);
                            window.set_can_unmount(false);
                            window.set_selected_source(-1);
                            if let Ok(mut rows) = sources_for_timer.lock() {
                                rows.clear();
                                window.set_source_names(source_items(&rows));
                            }
                            *source_for_timer.lock().expect("source lock") = None;
                            window.set_status("No compatible physical source found".into());
                        }
                    }
                    CompletedOperation::Refresh(Err(error)) => {
                        let _ = linuxfs_app::write_physical_scan_log(&error.to_string());
                        window.set_source_name("Physical scan failed".into());
                        window.set_source_details(error.clone().into());
                        window.set_can_mount(false);
                        window.set_can_unmount(false);
                        window.set_selected_source(-1);
                        window.set_status(format!("Physical refresh failed: {error}").into());
                    }
                    CompletedOperation::Mount(Ok((source_id, point))) => {
                        if let (Ok(mut sources), Ok(mut current)) =
                            (sources_for_timer.lock(), source_for_timer.lock())
                        {
                            let _ = linuxfs_app::apply_source_mount_state(
                                &mut sources,
                                &mut current,
                                source_id,
                                linuxfs_app::SourceStatus::Mounted,
                                Some(point.clone()),
                            );
                        }
                        state_for_timer
                            .lock()
                            .expect("UI state lock")
                            .set_mounted(&point);
                        window.set_status(
                            format!("Mounted read-only on {point} — source unchanged").into(),
                        );
                        window.set_can_mount(false);
                        window.set_can_unmount(true);
                    }
                    CompletedOperation::Unmount(Ok(source_id)) => {
                        if let (Ok(mut sources), Ok(mut current)) =
                            (sources_for_timer.lock(), source_for_timer.lock())
                        {
                            let _ = linuxfs_app::apply_source_mount_state(
                                &mut sources,
                                &mut current,
                                source_id,
                                linuxfs_app::SourceStatus::Compatible,
                                None,
                            );
                        }
                        state_for_timer
                            .lock()
                            .expect("UI state lock")
                            .set_unmounted();
                        window.set_status("Unmount completed".into());
                        window.set_can_mount(true);
                        window.set_can_unmount(false);
                    }
                    CompletedOperation::Probe(Err(error)) => {
                        window.set_status(format!("Refresh failed: {error}").into());
                        window.set_source_name("No compatible source".into());
                        window.set_source_details("The image could not be opened safely.".into());
                        window.set_can_mount(false);
                        window.set_can_unmount(false);
                        window.set_selected_source(-1);
                        if let Ok(mut rows) = sources_for_timer.lock() {
                            rows.clear();
                            window.set_source_names(source_items(&rows));
                        }
                        *source_for_timer.lock().expect("source lock") = None;
                    }
                    CompletedOperation::Mount(Err(error)) => {
                        let capabilities = source_for_timer
                            .lock()
                            .ok()
                            .and_then(|source| source.as_ref().map(|source| (source.can_mount(), source.can_unmount())));
                        if let Some((can_mount, can_unmount)) = capabilities {
                            window.set_can_mount(can_mount);
                            window.set_can_unmount(can_unmount);
                        }
                        window.set_status(format!("Mount failed: {error}").into())
                    }
                    CompletedOperation::Unmount(Err(error)) => {
                        window.set_can_mount(false);
                        window.set_can_unmount(true);
                        window.set_status(format!("Unmount failed: {error}").into())
                    }
                }
            }
        },
    );
    if !initial_path.trim().is_empty() {
        start_probe(initial_path);
    }
    window.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_status_text_confirms_a_ready_winfsp_engine() {
        let assessment = linuxfs_winfsp::WinFspAssessment::from_checks(
            true,
            true,
            linuxfs_winfsp::WinFspLauncherStatus::Running,
            true,
        );

        assert_eq!(
            engine_status_text(&assessment),
            "WinFsp engine: Ready — installed, launcher running"
        );
    }

    #[test]
    fn prerequisite_state_blocks_startup_until_winfsp_is_ready() {
        let unavailable =
            PrerequisiteState::from_assessment(&linuxfs_winfsp::WinFspAssessment::from_checks(
                false,
                false,
                linuxfs_winfsp::WinFspLauncherStatus::NotInstalled,
                false,
            ));
        assert!(unavailable.visible);
        assert!(!unavailable.can_continue);

        let ready =
            PrerequisiteState::from_assessment(&linuxfs_winfsp::WinFspAssessment::from_checks(
                true,
                true,
                linuxfs_winfsp::WinFspLauncherStatus::Running,
                true,
            ));
        assert!(!ready.visible);
        assert!(ready.can_continue);
    }

    #[test]
    fn ui_state_tracks_source_and_mount_capabilities() {
        let mut state = UiState::new("disk.raw");
        assert_eq!(state.image_path, "disk.raw");
        assert!(!state.can_mount);
        state.set_compatible("ext4", "Raw image");
        assert!(state.can_mount);
        state.set_mounted("L:");
        assert!(!state.can_mount);
        assert!(state.can_unmount);
        state.set_unmounted();
        assert!(state.can_mount);
        assert!(!state.can_unmount);
    }

    #[test]
    fn ui_state_selection_preserves_mounted_source_capabilities() {
        let source = linuxfs_app::SourceViewModel {
            id: linuxfs_app::SourceId(1),
            kind: linuxfs_app::SourceKind::Image,
            display_name: "fixture.img".to_owned(),
            source_description: "Raw image".to_owned(),
            source_path: "fixture.img".to_owned(),
            partition_range: None,
            physical_disk_index: None,
            filesystem_type: Some("ext4".to_owned()),
            label: None,
            uuid: None,
            size_bytes: None,
            status: linuxfs_app::SourceStatus::Mounted,
            mount_point: Some("L:".to_owned()),
            read_only: true,
        };
        let mut state = UiState::new("");

        state.set_source(&source);

        assert!(!state.can_mount);
        assert!(state.can_unmount);
    }

    #[test]
    fn empty_cli_path_is_rejected_without_fake_success() {
        assert_eq!(UiState::validate_path(" "), Err("provide an image path"));
    }
}
