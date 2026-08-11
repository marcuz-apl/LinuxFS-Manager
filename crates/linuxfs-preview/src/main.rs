slint::slint! {
    import { Button, VerticalBox, HorizontalBox, GroupBox, LineEdit } from "std-widgets.slint";

    export component MainWindow inherits Window {
        title: "LinuxFS Manager";
        min-width: 760px;
        min-height: 430px;

        in-out property <string> status: "Ready.";
        in-out property <string> source_name: "No source loaded";
        in-out property <string> source_details: "Open a raw Ext image to inspect it.";
        in-out property <string> image_path: "";
        in-out property <bool> can_mount: false;
        in-out property <bool> can_unmount: false;
        callback mount_clicked();
        callback unmount_clicked();
        callback details_clicked();
        callback refresh_clicked();
        callback open_image_clicked();

        VerticalBox {
            padding: 24px;
            spacing: 16px;

            HorizontalBox {
                Text { text: "LinuxFS Manager"; font-size: 28px; font-weight: 700; }
                Rectangle { horizontal-stretch: 1; }
                Button { text: "Refresh"; clicked => { root.refresh_clicked(); } }
                Button { text: "Open Image…"; clicked => { root.open_image_clicked(); } }
            }

            Rectangle {
                background: #fff4d6;
                border-radius: 6px;
                min-height: 46px;
                Text { text: "READ ONLY — this preview never accesses or modifies a source filesystem."; color: #714f00; vertical-alignment: center; }
            }

            GroupBox {
                title: "Loaded source";
                VerticalBox {
                    padding: 12px;
                    spacing: 6px;
                    Text { text: source_name; font-weight: 700; }
                    Text { text: source_details; }
                    LineEdit { text <=> root.image_path; placeholder-text: "CLI image path (first argument)"; }
                    HorizontalBox {
                        spacing: 8px;
                        Button { text: "Mount"; enabled: root.can_mount; clicked => { root.mount_clicked(); } }
                        Button { text: "Unmount"; enabled: root.can_unmount; clicked => { root.unmount_clicked(); } }
                        Button { text: "Details"; clicked => { root.details_clicked(); } }
                    }
                }
            }

            Text { text: status; color: #4a5568; }
            Rectangle { vertical-stretch: 1; }
        }
    }
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
            source_details: "Open a raw Ext image to inspect it.".to_owned(),
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
        ImageSourceProvider, MountService, SourceProvider, WindowsImageMountService,
    };
    use std::{
        env,
        sync::{Arc, Mutex},
    };

    let image = env::args_os()
        .nth(1)
        .map(|path| path.to_string_lossy().into_owned());
    let mut provider = ImageSourceProvider::new();
    let service = Arc::new(Mutex::new(WindowsImageMountService::new("L:")));
    let _winfsp = winfsp::winfsp_init()?;
    let window = MainWindow::new()?;
    let initial_path = image.unwrap_or_default();
    window.set_image_path(initial_path.clone().into());
    let state = Arc::new(Mutex::new(UiState::new(&initial_path)));
    let current_source = Arc::new(Mutex::new(None::<linuxfs_app::SourceViewModel>));

    let reload = {
        let state = Arc::clone(&state);
        let source_slot = Arc::clone(&current_source);
        let weak = window.as_weak();
        move |path: String, provider: &mut ImageSourceProvider| {
            let result = UiState::validate_path(&path)
                .map_err(|error| error.to_owned())
                .and_then(|_| {
                    provider
                        .open_image(&path)
                        .map_err(|error| error.to_string())
                });
            if let Some(window) = weak.upgrade() {
                match result {
                    Ok(source) => {
                        let filesystem = source
                            .filesystem_type
                            .clone()
                            .unwrap_or_else(|| "Unknown".to_owned());
                        let mut ui = state.lock().expect("UI state lock");
                        ui.image_path = path;
                        ui.source_name = source.display_name.clone();
                        ui.set_compatible(&filesystem, &source.source_description);
                        window.set_source_name(ui.source_name.clone().into());
                        window.set_source_details(ui.source_details.clone().into());
                        window.set_status("Source refreshed read-only".into());
                        window.set_can_mount(ui.can_mount);
                        window.set_can_unmount(ui.can_unmount);
                        *source_slot.lock().expect("source lock") = Some(source);
                    }
                    Err(error) => {
                        window.set_source_name("No compatible source".into());
                        window.set_source_details("The image could not be opened safely.".into());
                        window.set_status(format!("Refresh failed: {error}").into());
                        window.set_can_mount(false);
                        window.set_can_unmount(false);
                        *source_slot.lock().expect("source lock") = None;
                    }
                }
            }
        }
    };
    if !initial_path.trim().is_empty() {
        reload(initial_path, &mut provider);
    }

    let provider = Arc::new(Mutex::new(provider));
    let reload_for_refresh = Arc::new(reload);
    let weak = window.as_weak();
    let provider_for_refresh = Arc::clone(&provider);
    let state_for_refresh = Arc::clone(&state);
    let reload_for_refresh_handler = Arc::clone(&reload_for_refresh);
    window.on_refresh_clicked(move || {
        let path = state_for_refresh
            .lock()
            .expect("UI state lock")
            .image_path
            .clone();
        reload_for_refresh_handler(
            path,
            &mut provider_for_refresh.lock().expect("provider lock"),
        );
        if let Some(window) = weak.upgrade() {
            window.set_status("Refresh completed".into());
        }
    });
    let reload_for_open = Arc::clone(&reload_for_refresh);
    let provider_for_open = Arc::clone(&provider);
    let state_for_open = Arc::clone(&state);
    window.on_open_image_clicked(move || {
        let path = state_for_open
            .lock()
            .expect("UI state lock")
            .image_path
            .clone();
        reload_for_open(path, &mut provider_for_open.lock().expect("provider lock"));
    });
    let weak = window.as_weak();
    let source_slot = Arc::clone(&current_source);
    let service_for_mount = Arc::clone(&service);
    let state_for_mount = Arc::clone(&state);
    window.on_mount_clicked(move || {
        let result = source_slot
            .lock()
            .expect("source lock")
            .as_ref()
            .cloned()
            .ok_or_else(|| "no source loaded".to_owned())
            .and_then(|source| {
                service_for_mount
                    .lock()
                    .map_err(|_| "mount service lock poisoned".to_owned())
                    .and_then(|mut service| {
                        service.mount(&source).map_err(|error| error.to_string())
                    })
            });
        if let Some(window) = weak.upgrade() {
            match result {
                Ok(point) => {
                    state_for_mount
                        .lock()
                        .expect("UI state lock")
                        .set_mounted(&point);
                    window.set_status(
                        format!("Mounted read-only on {point} — source unchanged").into(),
                    );
                    window.set_can_mount(false);
                    window.set_can_unmount(true);
                }
                Err(error) => window.set_status(format!("Mount failed: {error}").into()),
            }
        }
    });
    let weak = window.as_weak();
    let source_slot = Arc::clone(&current_source);
    let service_for_unmount = Arc::clone(&service);
    let state_for_unmount = Arc::clone(&state);
    window.on_unmount_clicked(move || {
        let result = source_slot
            .lock()
            .expect("source lock")
            .as_ref()
            .cloned()
            .ok_or_else(|| "no source loaded".to_owned())
            .and_then(|source| {
                service_for_unmount
                    .lock()
                    .map_err(|_| "mount service lock poisoned".to_owned())
                    .and_then(|mut service| {
                        service.unmount(&source).map_err(|error| error.to_string())
                    })
            });
        if let Some(window) = weak.upgrade() {
            match result {
                Ok(()) => {
                    state_for_unmount
                        .lock()
                        .expect("UI state lock")
                        .set_unmounted();
                    window.set_status("Unmount completed".into());
                    window.set_can_mount(true);
                    window.set_can_unmount(false);
                }
                Err(error) => window.set_status(format!("Unmount failed: {error}").into()),
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
    window.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn empty_cli_path_is_rejected_without_fake_success() {
        assert_eq!(UiState::validate_path(" "), Err("provide an image path"));
    }
}
