import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window
    visible: true
    width: 1100
    height: 700
    title: "LinuxFS Manager"

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16

        RowLayout {
            Layout.fillWidth: true
            Label {
                text: "LinuxFS Manager"
                font.pixelSize: 28
                font.bold: true
            }
            Item { Layout.fillWidth: true }
            Button { text: "Refresh"; onClicked: appCommands.refresh() }
            Button { text: "Open Image…"; onClicked: appCommands.openImage() }
        }

        Frame {
            Layout.fillWidth: true
            background: Rectangle { color: "#fff4d6"; radius: 6 }
            Label {
                anchors.fill: parent
                anchors.margins: 12
                text: "READ ONLY — LinuxFS Manager never writes to the Linux source filesystem."
                color: "#714f00"
            }
        }

        Label {
            visible: appModel.busy
            text: appModel.message || "Working…"
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: appModel.sources
            delegate: Frame {
                width: ListView.view.width
                padding: 12
                RowLayout {
                    anchors.fill: parent
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: model.displayName; font.bold: true }
                        Label { text: model.filesystemType + "  ·  " + model.sourceDescription }
                        Label { text: model.status + (model.mountPoint ? "  ·  " + model.mountPoint : "") }
                    }
                    Button { text: "Mount"; enabled: model.canMount; onClicked: appCommands.mount(model.id) }
                    Button { text: "Unmount"; enabled: model.canUnmount; onClicked: appCommands.unmount(model.id) }
                    Button { text: "Details"; onClicked: appCommands.showDetails(model.id) }
                }
            }
        }
    }
}