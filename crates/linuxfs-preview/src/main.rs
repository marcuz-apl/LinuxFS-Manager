slint::slint! {
    import { Button, VerticalBox, HorizontalBox, GroupBox } from "std-widgets.slint";

    export component MainWindow inherits Window {
        title: "LinuxFS Manager — Lightweight Preview";
        min-width: 760px;
        min-height: 430px;

        in-out property <string> status: "Ready for a simulated read-only mount.";
        callback mount_clicked();
        callback unmount_clicked();
        callback details_clicked();

        VerticalBox {
            padding: 24px;
            spacing: 16px;

            HorizontalBox {
                Text { text: "LinuxFS Manager"; font-size: 28px; font-weight: 700; }
                Rectangle { horizontal-stretch: 1; }
                Button { text: "Refresh"; clicked => { status = "Test refresh completed"; } }
                Button { text: "Open Image…"; clicked => { status = "Test image selected"; } }
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
                    Text { text: "ubuntu24-vdisk1.raw"; font-weight: 700; }
                    Text { text: "Ext4  ·  Raw image  ·  Compatible  ·  Read-only source"; }
                    HorizontalBox {
                        spacing: 8px;
                        Button { text: "Mount"; clicked => { root.mount_clicked(); } }
                        Button { text: "Unmount"; clicked => { root.unmount_clicked(); } }
                        Button { text: "Details"; clicked => { root.details_clicked(); } }
                    }
                }
            }

            Text { text: status; color: #4a5568; }
            Rectangle { vertical-stretch: 1; }
        }
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
        .unwrap_or_else(|| "E:\\vmbox\\wsl-disks\\ubuntu24-vdisk1.raw".into());
    let mut provider = ImageSourceProvider::new();
    let source = provider.open_image(&image.to_string_lossy())?;
    let service = Arc::new(Mutex::new(WindowsImageMountService::new("L:")));
    let _winfsp = winfsp::winfsp_init()?;
    let window = MainWindow::new()?;
    let weak = window.as_weak();
    let source_for_mount = source.clone();
    let service_for_mount = Arc::clone(&service);
    window.on_mount_clicked(move || {
        let result = service_for_mount
            .lock()
            .map_err(|_| "mount service lock poisoned".to_owned())
            .and_then(|mut service| {
                service
                    .mount(&source_for_mount)
                    .map_err(|error| error.to_string())
            });
        if let Some(window) = weak.upgrade() {
            window.set_status(match result {
                Ok(point) => format!("Mounted read-only on {point} — source unchanged").into(),
                Err(error) => format!("Mount failed: {error}").into(),
            });
        }
    });
    let weak = window.as_weak();
    let source_for_unmount = source.clone();
    let service_for_unmount = Arc::clone(&service);
    window.on_unmount_clicked(move || {
        let result = service_for_unmount
            .lock()
            .map_err(|_| "mount service lock poisoned".to_owned())
            .and_then(|mut service| {
                service
                    .unmount(&source_for_unmount)
                    .map_err(|error| error.to_string())
            });
        if let Some(window) = weak.upgrade() {
            window.set_status(match result {
                Ok(()) => "Unmount completed".into(),
                Err(error) => format!("Unmount failed: {error}").into(),
            });
        }
    });
    let weak = window.as_weak();
    window.on_details_clicked(move || {
        if let Some(window) = weak.upgrade() {
            window.set_status("Ext4 · read-only · source integrity protected".into());
        }
    });
    window.run()?;
    Ok(())
}
