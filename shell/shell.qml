import QtQuick
import QtQuick.Shapes
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// Newton's face: a puck pinned to the bottom-right of every workspace. This is
// a layer-shell surface, not a window - the compositor never decorates it,
// never moves it, and never hides it when the workspace changes.
ShellRoot {
    id: root

    property string state: "standby"
    property bool busy: false
    property real level: 0.0

    readonly property bool working: state === "listening" && busy
    readonly property bool speaking: state === "speaking"
    readonly property real size: 88
    readonly property string sockPath: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/newton.sock"

    function apply(line) {
        try {
            const m = JSON.parse(line);
            if (m.state !== undefined)
                root.state = m.state;
            if (m.busy !== undefined)
                root.busy = m.busy;
            if (m.level !== undefined)
                root.level = m.level;
        } catch (e) {
            // A torn line is not worth killing the orb over.
        }
    }

    // The daemon may start after the shell, so reconnection is the normal case.
    Socket {
        id: sock

        path: root.sockPath
        connected: true
        parser: SplitParser {
            splitMarker: "\n"
            onRead: data => root.apply(data)
        }
        onConnectionStateChanged: {
            if (!connected)
                retry.start();
        }
    }

    Timer {
        id: retry

        interval: 1200
        repeat: false
        onTriggered: sock.connected = true
    }

    PanelWindow {
        id: panel

        implicitWidth: root.size
        implicitHeight: root.size
        color: "transparent"
        anchors {
            bottom: true
            right: true
        }
        margins {
            bottom: 28
            right: 28
        }
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.exclusionMode: ExclusionMode.Ignore
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

        // Matte black puck. One material, no gradient.
        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: "#0b0a09"

            Rectangle {
                anchors.fill: parent
                anchors.margins: 5
                radius: width / 2
                color: "transparent"
                border.width: 1
                border.color: "#171412"
            }
        }

        // The light: a thin line on the face. Thickness is the only visualizer,
        // and it never glows.
        Shape {
            id: ring

            anchors.fill: parent
            visible: !root.working

            readonly property real outer: root.state === "standby" ? 62
                                        : root.state === "booting" ? 56
                                        : 50
            readonly property real thickness: root.speaking ? 2.2 + 2.6 * Math.min(1, root.level) : 2.6
            readonly property real radius: (outer - thickness) / 2
            readonly property real cx: width / 2
            readonly property real cy: height / 2
            readonly property color tint: root.state === "standby" ? "#44f7f4"
                                        : root.state === "listening" ? "#f1f59c"
                                        : root.state === "speaking" ? (root.level > 0.6 ? "#9893f5" : "#776eec")
                                        : root.state === "error" ? "#e5484d"
                                        : "#fbfaf8"

            ShapePath {
                strokeColor: ring.tint
                strokeWidth: ring.thickness
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap

                PathAngleArc {
                    centerX: ring.cx
                    centerY: ring.cy
                    radiusX: ring.radius
                    radiusY: ring.radius
                    startAngle: root.state === "booting" ? -60 : -90
                    sweepAngle: root.state === "booting" ? 44 : 359.99
                }
            }

            RotationAnimation on rotation {
                running: root.state === "booting"
                from: 0
                to: 360
                duration: 2400
                loops: Animation.Infinite
            }
        }

        // Working: seven short blocks, turning slowly. Same weight as the ring.
        Item {
            anchors.fill: parent
            visible: root.working

            Repeater {
                model: 7

                Shape {
                    required property int index

                    anchors.fill: parent

                    ShapePath {
                        strokeColor: "#f5f594"
                        strokeWidth: 2.6
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap

                        PathAngleArc {
                            centerX: parent.width / 2
                            centerY: parent.height / 2
                            radiusX: 24.4
                            radiusY: 24.4
                            startAngle: index * (360 / 7) - 90
                            sweepAngle: 15
                        }
                    }
                }
            }

            RotationAnimation on rotation {
                running: root.working
                from: 0
                to: 360
                duration: 3000
                loops: Animation.Infinite
            }
        }

        MouseArea {
            anchors.fill: parent

            onClicked: function (mouse) {
                if (mouse.button === Qt.RightButton) {
                    sock.write("{\"cmd\":\"quit\"}\n");
                    return;
                }
                sock.write("{\"cmd\":\"talk\"}\n");
            }
        }
    }
}
