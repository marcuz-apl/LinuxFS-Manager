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
        ImageSourceProvider, MountService, SourceProvider, SourceViewModel,
        WindowsImageMountService,
        runtime::{BackgroundOperation, spawn_background},
    };
    use std::{
        env,
        sync::{Arc, Mutex},
        time::Duration,
    };

    enum PendingOperation {
        Probe(BackgroundOperation<SourceViewModel>, String),
        Mount(BackgroundOperation<String>),
        Unmount(BackgroundOperation<()>),
    }
    #[allow(clippy::large_enum_variant)]
    enum CompletedOperation {
        Probe(Result<(SourceViewModel, String), String>),
        Mount(Result<String, String>),
        Unmount(Result<(), String>),
    }

    let image = env::args_os()
        .nth(1)
        .map(|path| path.to_string_lossy().into_owned());
    let provider = Arc::new(Mutex::new(ImageSourceProvider::new()));
    let service = Arc::new(Mutex::new(WindowsImageMountService::new("L:")));
    let _winfsp = winfsp::winfsp_init()?;
    let window = MainWindow::new()?;
    let initial_path = image.unwrap_or_default();
    window.set_image_path(initial_path.clone().into());
    let state = Arc::new(Mutex::new(UiState::new(&initial_path)));
    let current_source = Arc::new(Mutex::new(None::<linuxfs_app::SourceViewModel>));
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
    let weak = window.as_weak();
    let state_for_refresh = Arc::clone(&state);
    let start_probe_for_refresh = start_probe.clone();
    window.on_refresh_clicked(move || {
        let path = state_for_refresh
            .lock()
            .expect("UI state lock")
            .image_path
            .clone();
        if let Some(window) = weak.upgrade() {
            window.set_status("Refreshing read-only source…".into());
        }
        start_probe_for_refresh(path);
    });
    let state_for_open = Arc::clone(&state);
    let start_probe_for_open = start_probe.clone();
    window.on_open_image_clicked(move || {
        let path = state_for_open
            .lock()
            .expect("UI state lock")
            .image_path
            .clone();
        start_probe_for_open(path);
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
            Some(PendingOperation::Mount(operation));
        if let Some(window) = weak.upgrade() {
            window.set_status("Mounting read-only…".into());
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
            Some(PendingOperation::Unmount(operation));
        if let Some(window) = weak.upgrade() {
            window.set_status("Unmounting…".into());
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
                PendingOperation::Mount(operation) => operation.try_receive().map(|result| {
                    CompletedOperation::Mount(result.map_err(|error| error.to_string()))
                }),
                PendingOperation::Unmount(operation) => operation.try_receive().map(|result| {
                    CompletedOperation::Unmount(result.map_err(|error| error.to_string()))
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
                        *source_for_timer.lock().expect("source lock") = Some(source);
                        window.set_status("Source refreshed read-only".into());
                    }
                    CompletedOperation::Mount(Ok(point)) => {
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
                    CompletedOperation::Unmount(Ok(())) => {
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
                        *source_for_timer.lock().expect("source lock") = None;
                    }
                    CompletedOperation::Mount(Err(error)) => {
                        window.set_status(format!("Mount failed: {error}").into())
                    }
                    CompletedOperation::Unmount(Err(error)) => {
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
