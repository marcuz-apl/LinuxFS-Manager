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
                title: "Detected source (mock data)";
                VerticalBox {
                    padding: 12px;
                    spacing: 6px;
                    Text { text: "ubuntu24-vdisk1.raw"; font-weight: 700; }
                    Text { text: "Ext4  ·  Raw image  ·  Compatible  ·  Read-only"; }
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

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;
    let weak = window.as_weak();
    window.on_mount_clicked(move || {
        if let Some(window) = weak.upgrade() {
            window.set_status("Simulated read-only mount on L: — source unchanged".into());
        }
    });
    let weak = window.as_weak();
    window.on_unmount_clicked(move || {
        if let Some(window) = weak.upgrade() {
            window.set_status("Simulated unmount completed".into());
        }
    });
    let weak = window.as_weak();
    window.on_details_clicked(move || {
        if let Some(window) = weak.upgrade() {
            window.set_status("Ext4 · read-only · mock source · no disk access".into());
        }
    });
    window.run()
}
