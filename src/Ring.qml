import QtQuick
import QtQuick.Shapes
import Newton

// The whole interface: one ring, five looks. No text, no buttons.
Item {
    id: root

    property string orbState: "booting"
    property bool busy: false
    property real level: 0.35

    property color tint: Theme.colorFor(orbState)
    readonly property real stroke: Theme.strokeFor(orbState, busy)
    readonly property real radius: Math.min(width, height) / 2 - stroke
    readonly property real cx: width / 2
    readonly property real cy: height / 2

    // Spinner geometry (arc + gap) for booting, dashed chase while working.
    readonly property bool spinning: orbState === "booting" || (orbState === "listening" && busy)
    readonly property bool dotted: orbState === "listening" && busy

    // Soft glow: three translucent rings. Cheaper and steadier than a blur
    // pass, and it works on every Qt 6 backend including software.
    Repeater {
        model: 3

        delegate: Shape {
            required property int index

            readonly property real spread: [2.8, 1.9, 1.35][index]

            anchors.fill: parent
            opacity: 0.16 - index * 0.05

            ShapePath {
                strokeColor: root.tint
                strokeWidth: root.stroke * spread
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap

                PathAngleArc {
                    centerX: root.cx
                    centerY: root.cy
                    radiusX: root.radius
                    radiusY: root.radius
                    startAngle: -90
                    sweepAngle: 359.99
                }
            }
        }
    }

    Shape {
        id: mainRing

        anchors.fill: parent

        ShapePath {
            strokeColor: root.tint
            strokeWidth: root.stroke
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            strokeStyle: root.dotted ? ShapePath.DashLine : ShapePath.SolidLine
            dashPattern: [1, 2.5]

            PathAngleArc {
                centerX: root.cx
                centerY: root.cy
                radiusX: root.radius
                radiusY: root.radius
                startAngle: -90
                sweepAngle: root.spinning ? 300 : 359.99
            }
        }

        NumberAnimation on rotation {
            running: root.spinning
            from: 0
            to: 360
            duration: 1600
            loops: Animation.Infinite
        }
    }

    // Two-tone standby ring, the "two gradients of green" from the renders.
    Shape {
        anchors.fill: parent
        visible: root.orbState === "standby"

        ShapePath {
            strokeColor: Theme.standbyInner
            strokeWidth: Math.max(2, root.stroke * 0.22)
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap

            PathAngleArc {
                centerX: root.cx
                centerY: root.cy
                radiusX: root.radius
                radiusY: root.radius
                startAngle: -110
                sweepAngle: 300
            }
        }
    }

    // Speaking: the ring becomes the visualizer of its own voice.
    Item {
        id: bars

        property real phase: 0

        anchors.fill: parent
        visible: root.orbState === "speaking"

        NumberAnimation on phase {
            running: root.orbState === "speaking"
            from: 0
            to: 6.2832
            duration: 1100
            loops: Animation.Infinite
        }

        Repeater {
            model: 48

            delegate: Rectangle {
                required property int index

                readonly property real angle: index * 2 * Math.PI / 48
                readonly property real ring: root.radius + root.stroke * 0.6
                readonly property real wave: Math.max(0, Math.sin(bars.phase + index * 0.9))

                width: 3
                height: 4 + 26 * wave * (0.3 + root.level)
                radius: 1.5
                color: root.tint
                x: root.cx + Math.cos(angle) * ring - width / 2
                y: root.cy + Math.sin(angle) * ring - height / 2
                rotation: index * 7.5
            }
        }
    }

    // Error: the only failure signal a text-free UI has. Pulses twice, waits.
    SequentialAnimation on scale {
        running: root.orbState === "error"
        loops: Animation.Infinite

        NumberAnimation {
            to: 1.06
            duration: Theme.fastMs
            easing.type: Easing.OutCubic
        }
        NumberAnimation {
            to: 1.0
            duration: Theme.slowMs
            easing.type: Easing.InOutCubic
        }
        PauseAnimation {
            duration: 900
        }
    }

    Behavior on tint {
        ColorAnimation {
            duration: Theme.baseMs
        }
    }
}
