#![cfg_attr(windows, windows_subsystem = "windows")]

slint::slint! {
    import { Button, ComboBox, VerticalBox, HorizontalBox, LineEdit, ListView } from "std-widgets.slint";

    export component MainWindow inherits Window {
        title: root.app_title;
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
        in-out property <[string]> language_options: [];
        in-out property <int> selected_language_index: 0;
        in-out property <string> app_title: "LinuxFS Manager";
        in-out property <string> app_subtitle: "Read Linux filesystems safely on Windows";
        in-out property <string> scan_drives_text: "Scan Drives";
        in-out property <string> open_image_text: "Open Image…";
        in-out property <string> about_text: "About";
        in-out property <string> sources_text: "Sources";
        in-out property <string> sources_subtitle_text: "Partitions and image files";
        in-out property <string> sources_empty_text: "Scan your drives or open an image to begin.";
        in-out property <string> filesystem_details_text: "Filesystem details";
        in-out property <string> open_filesystem_image_text: "Open a filesystem image";
        in-out property <string> image_placeholder_text: "Image path (or use Open Image…)";
        in-out property <string> mount_text: "Mount";
        in-out property <string> unmount_text: "Unmount";
        in-out property <string> open_in_explorer_text: "Open in Explorer";
        in-out property <string> details_text: "Details";
        in-out property <string> read_only_warning_text: "READ ONLY — source filesystems are never modified.";
        in-out property <string> version_text: "Version";
        in-out property <string> about_description_text: "LinuxFS Manager provides safe, read-only access to Ext2/3/4, SquashFS, and supported XFS images from Windows physical disks, partitions, and raw image files.";
        in-out property <string> copyright_text: "LinuxFS Manager, @2026 Alfazen Inc. All rights reserved.";
        in-out property <string> close_text: "Close";
        callback mount_clicked();
        callback unmount_clicked();
        callback open_explorer_clicked();
        callback details_clicked();
        callback refresh_clicked();
        callback scan_drives_clicked();
        callback open_image_clicked();
        callback source_selected(int);
        callback language_selected(string);

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
                    Text { text: root.app_title; font-size: 25px; font-weight: 700; color: #17324d; }
                    Text { text: root.app_subtitle; font-size: 13px; color: #6b7c93; }
                }
                Rectangle { horizontal-stretch: 1; }
                ComboBox { width: 190px; model: root.language_options; current-index: root.selected_language_index; selected(value) => { root.language_selected(value); } }
                Button { width: 132px; text: root.scan_drives_text; clicked => { root.scan_drives_clicked(); } }
                Button { width: 142px; text: root.open_image_text; clicked => { root.open_image_clicked(); } }
                Button { width: 96px; text: root.about_text; clicked => { about_popup.show(); } }
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
                        background: #f7fafe;
                        border-radius: 14px;
                        border-width: 1px;
                        border-color: #e3edf7;

                        VerticalBox {
                            padding: 20px;
                            spacing: 8px;
                            Text { text: root.sources_text; font-size: 19px; font-weight: 700; color: #17324d; }
                            Text { text: root.sources_subtitle_text; font-size: 12px; color: #71849a; }
                            Rectangle { height: 1px; background: #e3edf7; }
                            Rectangle { height: 4px; }

                            if (root.source_names.length == 0) : Text {
                                text: root.sources_empty_text;
                                color: #71849a;
                                wrap: word-wrap;
                            }

                            ListView {
                                vertical-stretch: 1;
                                for name[index] in root.source_names : Rectangle {
                                    height: 46px;
                                    border-radius: 8px;
                                    background: root.selected_source == index ? #dbeafe : #ffffff00;
                                    Text {
                                        x: 12px;
                                        width: parent.width - 24px;
                                        text: name;
                                        color: root.selected_source == index ? #124f86 : #38536d;
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

                        Text { text: source_name; font-size: 24px; font-weight: 700; color: #17324d; overflow: elide; }

                        Rectangle {
                            vertical-stretch: 1;
                            background: #f9fbfd;
                            border-radius: 12px;
                            border-width: 1px;
                            border-color: #e6edf5;

                            VerticalBox {
                                padding: 20px;
                                spacing: 9px;
                                Text { text: root.filesystem_details_text; font-size: 15px; font-weight: 700; color: #17324d; }
                                Text { text: source_details; color: #526a83; wrap: word-wrap; }
                                Rectangle { vertical-stretch: 1; }
                            }
                        }

                        VerticalBox {
                            spacing: 7px;
                            Text { text: root.open_filesystem_image_text; font-size: 14px; font-weight: 700; color: #17324d; }
                            LineEdit { height: 40px; text <=> root.image_path; placeholder-text: root.image_placeholder_text; }
                        }

                        HorizontalBox {
                            height: 42px;
                            spacing: 10px;
                            Button { width: 132px; primary: true; text: root.mount_text; enabled: root.can_mount; clicked => { root.mount_clicked(); } }
                            Button { width: 132px; text: root.unmount_text; enabled: root.can_unmount; clicked => { root.unmount_clicked(); } }
                            Button { width: 218px; text: root.open_in_explorer_text; enabled: root.can_unmount; clicked => { root.open_explorer_clicked(); } }
                            Button { width: 132px; text: root.details_text; clicked => { root.details_clicked(); } }
                            Rectangle { horizontal-stretch: 1; }
                        }
                    }
                }
            }

            Rectangle {
                height: 66px;
                background: #edf5fb;
                border-radius: 12px;
                border-width: 1px;
                border-color: #d8e5f0;

                Rectangle {
                    x: 18px;
                    y: 22px;
                    width: 9px;
                    height: 22px;
                    background: #267d53;
                    border-radius: 5px;
                }
                Text {
                    x: 42px;
                    y: 8px;
                    text: root.read_only_warning_text;
                    color: #245f47;
                    font-size: 13px;
                    font-weight: 700;
                }
                Text {
                    x: 42px;
                    y: 34px;
                    width: parent.width - 62px;
                    text: engine_status + "  ·  " + status;
                    color: #46627a;
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
                            Text { text: root.app_title; font-size: 24px; font-weight: 700; color: #17324d; }
                            Text { text: root.version_text + " " + root.app_version; color: #64788f; }
                            Rectangle { vertical-stretch: 1; }
                        }
                    }

                    Rectangle { height: 1px; background: #e5edf5; }

                    Text {
                        text: root.about_description_text;
                        color: #405a72;
                        wrap: word-wrap;
                    }

                    Rectangle { vertical-stretch: 1; }

                    Text {
                        text: root.copyright_text;
                        color: #6b7c93;
                        horizontal-alignment: center;
                    }

                    HorizontalBox {
                        height: 40px;
                        Rectangle { horizontal-stretch: 1; }
                        Button { width: 100px; text: root.close_text; clicked => { about_popup.close(); } }
                    }
                }
            }
        }
    }

    export component PrerequisiteWindow inherits Window {
        title: root.app_title + " — " + root.prerequisite_title;
        width: 680px;
        height: 470px;
        preferred-width: 680px;
        preferred-height: 470px;
        background: #f5f8fc;
        icon: @image-url("../../../assets/linuxfs-manager.png");

        in-out property <string> requirement_message: "WinFsp is required before LinuxFS Manager can open.";
        in-out property <string> app_title: "LinuxFS Manager";
        in-out property <string> prerequisite_title: "WinFsp is required";
        in-out property <string> prerequisite_subtitle: "A Windows filesystem framework prerequisite";
        in-out property <string> to_continue_text: "To continue";
        in-out property <string> prerequisite_step_one_text: "1. Download WinFsp from its official release page.";
        in-out property <string> prerequisite_step_two_text: "2. Run the MSI installer and accept its driver installation.";
        in-out property <string> prerequisite_step_three_text: "3. Return here and select Recheck.";
        in-out property <string> prerequisite_notice_text: "LinuxFS Manager does not download, install, or modify WinFsp for you.";
        in-out property <string> close_text: "Close";
        in-out property <string> download_winfsp_text: "Download WinFsp";
        in-out property <string> recheck_text: "Recheck";
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
                        Text { text: root.prerequisite_title; font-size: 25px; font-weight: 700; color: #17324d; }
                        Text { text: root.prerequisite_subtitle; font-size: 13px; color: #61758b; }
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
                        Text { text: root.to_continue_text; font-size: 15px; font-weight: 700; color: #1d568b; }
                        Text { text: root.prerequisite_step_one_text; color: #365a7c; wrap: word-wrap; }
                        Text { text: root.prerequisite_step_two_text; color: #365a7c; wrap: word-wrap; }
                        Text { text: root.prerequisite_step_three_text; color: #365a7c; wrap: word-wrap; }
                    }
                }

                Text {
                    text: root.prerequisite_notice_text;
                    color: #687b90;
                    font-size: 12px;
                    wrap: word-wrap;
                }

                HorizontalBox {
                    height: 42px;
                    spacing: 10px;
                    Button { width: 122px; text: root.close_text; clicked => { root.close_clicked(); } }
                    Rectangle { horizontal-stretch: 1; }
                    Button { width: 136px; text: root.download_winfsp_text; clicked => { root.download_clicked(); } }
                    Button { width: 100px; text: root.recheck_text; clicked => { root.recheck_clicked(); } }
                }
            }
        }
    }
}

#[cfg(windows)]
fn language_items(
    catalog: &linuxfs_preview::localization::LocalizedCatalog,
) -> slint::ModelRc<slint::SharedString> {
    use slint::{ModelRc, SharedString, VecModel};
    use std::rc::Rc;

    ModelRc::from(Rc::new(VecModel::from(
        catalog
            .language_options()
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    )))
}

#[cfg(windows)]
fn apply_localized_copy(
    window: &MainWindow,
    catalog: &linuxfs_preview::localization::LocalizedCatalog,
) {
    let copy = catalog.copy;
    window.set_app_title(catalog.text("app_title", copy.app_title).into());
    window.set_app_subtitle(catalog.text("app_subtitle", copy.app_subtitle).into());
    window.set_scan_drives_text(catalog.text("scan_drives", copy.scan_drives).into());
    window.set_open_image_text(catalog.text("open_image", copy.open_image).into());
    window.set_about_text(catalog.text("about", copy.about).into());
    window.set_sources_text(catalog.text("sources", copy.sources).into());
    window.set_sources_subtitle_text(
        catalog
            .text("sources_subtitle", copy.sources_subtitle)
            .into(),
    );
    window.set_sources_empty_text(catalog.text("sources_empty", copy.sources_empty).into());
    window.set_filesystem_details_text(
        catalog
            .text("filesystem_details", copy.filesystem_details)
            .into(),
    );
    window.set_open_filesystem_image_text(
        catalog
            .text("open_filesystem_image", copy.open_filesystem_image)
            .into(),
    );
    window.set_image_placeholder_text(
        catalog
            .text("image_placeholder", copy.image_placeholder)
            .into(),
    );
    window.set_mount_text(catalog.text("mount", copy.mount).into());
    window.set_unmount_text(catalog.text("unmount", copy.unmount).into());
    window.set_open_in_explorer_text(
        catalog
            .text("open_in_explorer", copy.open_in_explorer)
            .into(),
    );
    window.set_details_text(catalog.text("details", copy.details).into());
    window.set_read_only_warning_text(
        catalog
            .text("read_only_warning", copy.read_only_warning)
            .into(),
    );
    window.set_version_text(catalog.text("version", copy.version).into());
    window.set_about_description_text(
        catalog
            .text("about_description", copy.about_description)
            .into(),
    );
    window.set_copyright_text(catalog.text("copyright", copy.copyright).into());
    window.set_close_text(catalog.text("close", copy.close).into());
    window.set_language_options(language_items(catalog));
}

#[cfg(windows)]
fn apply_prerequisite_copy(
    window: &PrerequisiteWindow,
    catalog: &linuxfs_preview::localization::LocalizedCatalog,
) {
    let copy = catalog.copy;
    window.set_app_title(catalog.text("app_title", copy.app_title).into());
    window.set_prerequisite_title(
        catalog
            .text("prerequisite_title", copy.prerequisite_title)
            .into(),
    );
    window.set_prerequisite_subtitle(
        catalog
            .text("prerequisite_subtitle", copy.prerequisite_subtitle)
            .into(),
    );
    window.set_to_continue_text(catalog.text("to_continue", copy.to_continue).into());
    window.set_prerequisite_step_one_text(
        catalog
            .text("prerequisite_step_one", copy.prerequisite_step_one)
            .into(),
    );
    window.set_prerequisite_step_two_text(
        catalog
            .text("prerequisite_step_two", copy.prerequisite_step_two)
            .into(),
    );
    window.set_prerequisite_step_three_text(
        catalog
            .text("prerequisite_step_three", copy.prerequisite_step_three)
            .into(),
    );
    window.set_prerequisite_notice_text(
        catalog
            .text("prerequisite_notice", copy.prerequisite_notice)
            .into(),
    );
    window.set_close_text(catalog.text("close", copy.close).into());
    window.set_download_winfsp_text(catalog.text("download_winfsp", copy.download_winfsp).into());
    window.set_recheck_text(catalog.text("recheck", copy.recheck).into());
}

#[cfg(windows)]
fn active_ui_copy(
    copy: &std::sync::Mutex<linuxfs_preview::localization::UiCopy>,
) -> linuxfs_preview::localization::UiCopy {
    copy.lock().map(|copy| *copy).unwrap_or_else(|_| {
        linuxfs_preview::localization::catalog(linuxfs_preview::localization::UiLanguage::English)
    })
}

#[cfg(windows)]
fn packaged_locales_directory() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("locales")))
        .unwrap_or_else(|| std::path::PathBuf::from("locales"))
}

fn start_pending_operation<T>(
    pending: &std::sync::Mutex<Option<T>>,
    start: impl FnOnce() -> T,
) -> bool {
    let mut pending = pending.lock().expect("pending operation lock");
    if pending.is_some() {
        return false;
    }
    *pending = Some(start());
    true
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

#[cfg(windows)]
#[allow(unsafe_code)]
fn enable_dark_caption(window: &slint::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;

    let window_handle = window.window_handle();
    let Ok(native_handle) = window_handle.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(native_handle) = native_handle.as_raw() else {
        return;
    };
    let enabled: i32 = 1;
    // SAFETY: `native_handle.hwnd` belongs to the live Slint window, and the
    // pointer/size pair describes the local `enabled` i32 for this call only.
    let _ = unsafe {
        DwmSetWindowAttribute(
            native_handle.hwnd.get() as *mut std::ffi::c_void,
            linuxfs_preview::dark_caption_attribute(),
            (&enabled as *const i32).cast(),
            u32::try_from(std::mem::size_of_val(&enabled)).unwrap_or(4),
        )
    };
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
    catalog: &linuxfs_preview::localization::LocalizedCatalog,
) -> Result<bool, Box<dyn std::error::Error>> {
    use std::sync::{Arc, Mutex};

    let window = PrerequisiteWindow::new()?;
    apply_prerequisite_copy(&window, catalog);
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
    fn new(path: &str, copy: linuxfs_preview::localization::UiCopy) -> Self {
        Self {
            image_path: path.to_owned(),
            source_name: copy.no_source_loaded().to_owned(),
            source_details: copy.open_raw_image_hint().to_owned(),
            status: copy.ready(),
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
        runtime::{BackgroundOperation, config_store, load_config, spawn_background},
    };
    use linuxfs_preview::localization::{
        language_from_self_name, load_catalog, resolve_language, windows_user_locale,
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

    let config = load_config().unwrap_or_default();
    let windows_locale = windows_user_locale();
    let resolved_language = resolve_language(config.ui_language.as_deref(), &windows_locale);
    let locales_directory = packaged_locales_directory();
    let localized_catalog = load_catalog(resolved_language, &locales_directory);
    let copy = localized_catalog.copy;
    let selected_language_index = config
        .ui_language
        .as_deref()
        .map(|language| resolve_language(Some(language), &windows_locale).selector_index())
        .unwrap_or(0);

    let mut assessment = linuxfs_winfsp::assess_winfsp();
    let _ = linuxfs_app::runtime::record_winfsp_assessment(&assessment);
    if !assessment.is_ready() && !run_prerequisite_gate(&assessment, &localized_catalog)? {
        return Ok(());
    }
    assessment = linuxfs_winfsp::assess_winfsp();
    let _ = linuxfs_app::runtime::record_winfsp_assessment(&assessment);
    if !assessment.is_ready() {
        return Ok(());
    }

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
    apply_localized_copy(&window, &localized_catalog);
    window.set_selected_language_index(selected_language_index);
    window.show()?;
    center_main_window(&window);
    let dark_caption_window = window.as_weak();
    slint::Timer::single_shot(Duration::from_millis(0), move || {
        if let Some(window) = dark_caption_window.upgrade() {
            enable_dark_caption(window.window());
        }
    });
    window.set_app_version(env!("LINUXFS_MANAGER_VERSION").into());
    window.set_engine_status(engine_status_text(&assessment).into());
    window.set_status(copy.ready().into());
    let initial_path = image.unwrap_or_default();
    window.set_image_path(initial_path.clone().into());
    let state = Arc::new(Mutex::new(UiState::new(&initial_path, copy)));
    let current_source = Arc::new(Mutex::new(None::<linuxfs_app::SourceViewModel>));
    let sources_for_ui = Arc::new(Mutex::new(Vec::<linuxfs_app::SourceViewModel>::new()));
    let pending = Arc::new(Mutex::new(None::<PendingOperation>));
    let config = Arc::new(Mutex::new(config));
    let active_copy = Arc::new(Mutex::new(copy));

    let weak = window.as_weak();
    let config_for_language = Arc::clone(&config);
    let active_copy_for_language = Arc::clone(&active_copy);
    let windows_locale_for_language = windows_locale.clone();
    let locales_directory_for_language = locales_directory.clone();
    window.on_language_selected(move |value| {
        let preference =
            language_from_self_name(value.as_str()).map(|language| language.tag().to_owned());
        let localized_catalog = load_catalog(
            resolve_language(preference.as_deref(), &windows_locale_for_language),
            &locales_directory_for_language,
        );
        let copy = localized_catalog.copy;
        if let Ok(mut active_copy) = active_copy_for_language.lock() {
            *active_copy = copy;
        }
        let save_result = config_for_language
            .lock()
            .map_err(|_| "configuration lock poisoned".to_owned())
            .and_then(|mut config| {
                config.ui_language = preference.clone();
                config_store()
                    .save(&config)
                    .map_err(|error| error.to_string())
            });
        if let Some(window) = weak.upgrade() {
            apply_localized_copy(&window, &localized_catalog);
            window.set_selected_language_index(
                preference
                    .as_deref()
                    .map(|language| {
                        resolve_language(Some(language), &windows_locale_for_language)
                            .selector_index()
                    })
                    .unwrap_or(0),
            );
            if let Err(error) = save_result {
                window.set_status(copy.language_save_failed(&error).into());
            }
        }
    });

    let start_probe: Arc<dyn Fn(String)> = {
        let provider = Arc::clone(&provider);
        let pending = Arc::clone(&pending);
        let copy = Arc::clone(&active_copy);
        let weak = window.as_weak();
        Arc::new(move |path: String| {
            let copy = active_ui_copy(&copy);
            if let Err(error) = UiState::validate_path(&path) {
                if let Some(window) = weak.upgrade() {
                    window.set_status(copy.refresh_failed(error).into());
                }
                return;
            }
            let provider_for_operation = Arc::clone(&provider);
            let probe_path = path.clone();
            let started = start_pending_operation(&pending, move || {
                let operation = spawn_background(move || {
                    provider_for_operation
                        .lock()
                        .expect("provider lock poisoned")
                        .open_image(&probe_path)
                });
                PendingOperation::Probe(operation, path)
            });
            if let Some(window) = weak.upgrade() {
                window.set_status(
                    if started {
                        "Opening image read-only…"
                    } else {
                        "Wait for the current operation to finish before opening another source"
                    }
                    .into(),
                );
            }
        })
    };
    let start_refresh: Arc<dyn Fn()> = {
        let provider = Arc::clone(&provider);
        let pending = Arc::clone(&pending);
        let weak = window.as_weak();
        Arc::new(move || {
            let provider_for_operation = Arc::clone(&provider);
            let started = start_pending_operation(&pending, move || {
                let operation = spawn_background(move || {
                    let mut provider = provider_for_operation.lock().map_err(|_| {
                        linuxfs_core::Error::new(
                            linuxfs_core::ErrorCategory::Internal,
                            "provider lock poisoned",
                        )
                    })?;
                    provider.refresh()
                });
                PendingOperation::Refresh(operation)
            });
            if let Some(window) = weak.upgrade() {
                window.set_status(
                    if started {
                        "Scanning physical sources read-only…"
                    } else {
                        "Wait for the current operation to finish before scanning again"
                    }
                    .into(),
                );
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
    let pending_for_open = Arc::clone(&pending);
    let weak = window.as_weak();
    window.on_open_image_clicked(move || {
        if pending_for_open
            .lock()
            .map(|pending| pending.is_some())
            .unwrap_or(true)
        {
            if let Some(window) = weak.upgrade() {
                window.set_status(
                    "Wait for the current operation to finish before opening another source".into(),
                );
            }
            return;
        }
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
    let pending_for_selection = Arc::clone(&pending);
    let weak = window.as_weak();
    window.on_source_selected(move |index| {
        if pending_for_selection
            .lock()
            .map(|pending| pending.is_some())
            .unwrap_or(true)
        {
            if let Some(window) = weak.upgrade() {
                window.set_status(
                    "Wait for the current mount operation to finish before selecting another source"
                        .into(),
                );
            }
            return;
        }
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
    let copy_for_mount = Arc::clone(&active_copy);
    window.on_mount_clicked(move || {
        let copy = active_ui_copy(&copy_for_mount);
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
        let started = match source {
            Ok(source) => {
                let service_for_operation = Arc::clone(&service_for_mount);
                start_pending_operation(&pending_for_mount, move || {
                    let operation = spawn_background(move || {
                        service_for_operation
                            .lock()
                            .expect("mount service lock poisoned")
                            .mount(&source)
                    });
                    PendingOperation::Mount(operation, source_id)
                })
            }
            Err(error) => {
                if let Some(window) = weak.upgrade() {
                    window.set_status(copy.mount_failed(&error).into());
                }
                return;
            }
        };
        if let Some(window) = weak.upgrade() {
            if started {
                window.set_status(copy.mounting().into());
                window.set_can_mount(false);
                window.set_can_unmount(false);
            } else {
                window.set_status(
                    "Wait for the current operation to finish before mounting another source"
                        .into(),
                );
            }
        }
    });
    let weak = window.as_weak();
    let source_slot = Arc::clone(&current_source);
    let service_for_unmount = Arc::clone(&service);
    let pending_for_unmount = Arc::clone(&pending);
    let copy_for_unmount = Arc::clone(&active_copy);
    window.on_unmount_clicked(move || {
        let copy = active_ui_copy(&copy_for_unmount);
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
        let started = match source {
            Ok(source) => {
                let service_for_operation = Arc::clone(&service_for_unmount);
                start_pending_operation(&pending_for_unmount, move || {
                    let operation = spawn_background(move || {
                        service_for_operation
                            .lock()
                            .expect("mount service lock poisoned")
                            .unmount(&source)
                    });
                    PendingOperation::Unmount(operation, source_id)
                })
            }
            Err(error) => {
                if let Some(window) = weak.upgrade() {
                    window.set_status(copy.unmount_failed(&error).into());
                }
                return;
            }
        };
        if let Some(window) = weak.upgrade() {
            if started {
                window.set_status(copy.unmounting().into());
                window.set_can_mount(false);
                window.set_can_unmount(false);
            } else {
                window.set_status(
                    "Wait for the current operation to finish before unmounting another source"
                        .into(),
                );
            }
        }
    });
    let weak = window.as_weak();
    let source_slot = Arc::clone(&current_source);
    let service_for_explorer = Arc::clone(&service);
    let copy_for_explorer = Arc::clone(&active_copy);
    window.on_open_explorer_clicked(move || {
        let copy = active_ui_copy(&copy_for_explorer);
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
                Ok(()) => window.set_status(copy.explorer_opened(mount_point).into()),
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
    let copy_for_timer = Arc::clone(&active_copy);
    let weak_for_timer = window.as_weak();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(50),
        move || {
            let copy = active_ui_copy(&copy_for_timer);
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
                        let mut selected_index = 0;
                        if let Ok(mut sources) = sources_for_timer.lock() {
                            *sources = linuxfs_app::reconcile_sources_preserving_mount_state(
                                &sources,
                                vec![source.clone()],
                            );
                            selected_index = sources
                                .iter()
                                .position(|candidate| candidate.id == source.id)
                                .and_then(|index| i32::try_from(index).ok())
                                .unwrap_or(0);
                            window.set_source_names(source_items(&sources));
                        }
                        window.set_selected_source(selected_index);
                        *source_for_timer.lock().expect("source lock") = Some(source);
                        window.set_status("Source refreshed read-only".into());
                    }
                    CompletedOperation::Refresh(Ok(sources)) => {
                        let sources = if let Ok(mut rows) = sources_for_timer.lock() {
                            *rows = linuxfs_app::reconcile_sources_preserving_mount_state(
                                &rows,
                                sources,
                            );
                            window.set_source_names(source_items(&rows));
                            rows.clone()
                        } else {
                            sources
                        };
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
                            window.set_source_name(copy.no_compatible_source().into());
                            window.set_source_details(copy.physical_scan_empty_details().into());
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
                        window.set_status(copy.mounted(&point).into());
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
                        window.set_status(copy.unmounted().into());
                        window.set_can_mount(true);
                        window.set_can_unmount(false);
                    }
                    CompletedOperation::Probe(Err(error)) => {
                        let mounted_source = if let Ok(mut rows) = sources_for_timer.lock() {
                            *rows = linuxfs_app::reconcile_sources_preserving_mount_state(
                                &rows,
                                Vec::new(),
                            );
                            window.set_source_names(source_items(&rows));
                            rows.iter().enumerate().find_map(|(index, source)| {
                                source.can_unmount().then(|| (index, source.clone()))
                            })
                        } else {
                            None
                        };
                        if let Some((index, source)) = mounted_source {
                            let mut ui = state_for_timer.lock().expect("UI state lock");
                            ui.set_source(&source);
                            window.set_source_name(ui.source_name.clone().into());
                            window.set_source_details(ui.source_details.clone().into());
                            window.set_can_mount(ui.can_mount);
                            window.set_can_unmount(ui.can_unmount);
                            window.set_selected_source(i32::try_from(index).unwrap_or(-1));
                            *source_for_timer.lock().expect("source lock") = Some(source);
                            window.set_status(
                                format!(
                                    "Image open failed: {error}; the existing mount remains available to unmount"
                                )
                                .into(),
                            );
                        } else {
                            window.set_status(copy.refresh_failed(&error).into());
                            window.set_source_name(copy.no_compatible_source().into());
                            window.set_source_details(copy.image_open_failed_details().into());
                            window.set_can_mount(false);
                            window.set_can_unmount(false);
                            window.set_selected_source(-1);
                            *source_for_timer.lock().expect("source lock") = None;
                        }
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
                        window.set_status(copy.mount_failed(&error).into())
                    }
                    CompletedOperation::Unmount(Err(error)) => {
                        window.set_can_mount(false);
                        window.set_can_unmount(true);
                        window.set_status(copy.unmount_failed(&error).into())
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
        let mut state = UiState::new(
            "disk.raw",
            linuxfs_preview::localization::catalog(
                linuxfs_preview::localization::UiLanguage::English,
            ),
        );
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
        let mut state = UiState::new(
            "",
            linuxfs_preview::localization::catalog(
                linuxfs_preview::localization::UiLanguage::English,
            ),
        );

        state.set_source(&source);

        assert!(!state.can_mount);
        assert!(state.can_unmount);
    }

    #[test]
    fn empty_cli_path_is_rejected_without_fake_success() {
        assert_eq!(UiState::validate_path(" "), Err("provide an image path"));
    }

    #[test]
    fn pending_operation_start_does_not_replace_an_active_operation() {
        let pending = std::sync::Mutex::new(None);

        assert!(start_pending_operation(&pending, || "first".to_owned()));
        assert!(!start_pending_operation(&pending, || "second".to_owned()));
        assert_eq!(
            pending.lock().expect("pending lock").as_deref(),
            Some("first")
        );
    }
}
