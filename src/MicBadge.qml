import QtQuick
import Newton

// Quiet, static warning that the microphone is off. Only there while muted.
Row {
    id: badge

    property bool muted: false
    property color tint: Theme.error

    visible: muted
    opacity: 0.9
    spacing: 6

    Rectangle {
        width: 14
        height: 14
        radius: 7
        color: badge.tint
        anchors.verticalCenter: parent.verticalCenter
    }

    Text {
        text: qsTr("Muted")
        color: badge.tint
        font.pixelSize: 12
        anchors.verticalCenter: parent.verticalCenter
    }
}
