import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    visible: true
    width: 1100
    height: 700
    title: "LinuxFS Manager — UI Test"

    ListModel {
        id: sources
        ListElement {
            displayName: "ubuntu24-vdisk1.raw"
            filesystemType: "Ext4"
            sourceDescription: "Raw image (read-only test data)"
            status: "Compatible"
            mountPoint: ""
            canMount: true
            canUnmount: false
        }
    }

    function setMounted(index, mounted) {
        sources.setProperty(index, "status", mounted ? "Mounted" : "Compatible")
        sources.setProperty(index, "mountPoint", mounted ? "L:" : "")
        sources.setProperty(index, "canMount", !mounted)
        sources.setProperty(index, "canUnmount", mounted)
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16

        RowLayout {
            Layout.fillWidth: true
            Label { text: "LinuxFS Manager"; font.pixelSize: 28; font.bold: true }
            Item { Layout.fillWidth: true }
            Button { text: "Refresh"; onClicked: testMessage.text = "Test refresh completed" }
            Button { text: "Open Image…"; onClicked: testMessage.text = "Test image selected" }
        }

        Frame {
            Layout.fillWidth: true
            background: Rectangle { color: "#fff4d6"; radius: 6 }
            Label {
                anchors.fill: parent
                anchors.margins: 12
                text: "READ ONLY — this UI test does not write to the source filesystem."
                color: "#714f00"
            }
        }

        Label { id: testMessage; text: "UI test mode — no Rust backend connected" }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: sources
            delegate: Frame {
                width: ListView.view.width
                padding: 12
                RowLayout {
                    anchors.fill: parent
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: displayName; font.bold: true }
                        Label { text: filesystemType + "  ·  " + sourceDescription }
                        Label { text: status + (mountPoint ? "  ·  " + mountPoint : "") }
                    }
                    Button {
                        text: "Mount"
                        enabled: canMount
                        onClicked: { setMounted(index, true); testMessage.text = "Simulated read-only mount on L:" }
                    }
                    Button {
                        text: "Unmount"
                        enabled: canUnmount
                        onClicked: { setMounted(index, false); testMessage.text = "Simulated unmount completed" }
                    }
                    Button { text: "Details"; onClicked: testMessage.text = "Ext4 · read-only · source unchanged" }
                }
            }
        }
    }
}
