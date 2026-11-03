import QtQuick
import QtQuick.Window
import Newton

// The device, not a window: a puck that sits on the desktop, bottom right.
// No frame, no title bar, always on top, transparent around the disc.
Window {
    id: win

    property var orb: null
    property var device: null

    width: 200
    height: 200
    visible: true
    color: "transparent"
    title: qsTr("Newton")
    // BypassWindowManagerHint: the window manager never decorates it, never
    // moves it, never resizes it and never lists it. It is painted at the
    // position main.cpp gives it and stays there.
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint | Qt.Tool
           | Qt.WindowDoesNotAcceptFocus | Qt.BypassWindowManagerHint

    // The puck: one matte disc. No gradient, no glow, no rim highlight - the
    // only light on it is the ring.
    Rectangle {
        id: puck

        anchors.fill: parent
        radius: width / 2
        color: "#0b0a09"

        // The machined groove the render shows just inside the edge. One
        // hairline, darker than the face: depth without a gradient.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 9
            radius: width / 2
            color: "transparent"
            border.width: 1
            border.color: "#1a1715"
        }
    }

    Ring {
        id: ring

        width: Math.min(parent.width, parent.height) * 0.74
        height: width
        anchors.centerIn: parent
        orbState: win.orb ? win.orb.state : "booting"
        busy: win.orb ? win.orb.busy : false
        level: win.device ? win.device.level : 0
    }

    MouseArea {
        id: grip

        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: function (mouse) {
            if (mouse.button === Qt.RightButton) {
                Qt.quit();
                return;
            }
            if (win.device)
                win.device.talk();
        }
    }

    Item {
        anchors.fill: parent
        focus: true

        Keys.onSpacePressed: if (win.device) win.device.talk()
        Keys.onEscapePressed: Qt.quit()
    }
}
