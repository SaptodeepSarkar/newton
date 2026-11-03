import QtQuick
import QtQuick.Shapes
import Newton

// The whole interface: one ring, five geometries. No text, no buttons.
// No shader, no MultiEffect: the bloom is stacked translucent strokes, so the
// software backend draws exactly what the GPU backend draws.
Item {
    id: root

    property string orbState: "booting"
    property bool busy: false
    property real level: 0.35

    readonly property bool working: orbState === "listening" && busy
    readonly property bool speaking: orbState === "speaking"
    readonly property real amplitude: speaking ? Math.max(0, Math.min(1, level)) : 0
    readonly property real size: Math.min(width, height)

    // Standby is the largest ring, near the rim; the active states are inset
    // with a wide dark margin. Booting floats its arc near the rim too.
    readonly property real outer: orbState === "standby" ? size * 0.74
                                : orbState === "booting" ? size * 0.64
                                : size * 0.52

    // Speaking is one state at two amplitudes: the stroke is the visualizer.
    readonly property real weight: speaking
        ? Theme.strokeFor("speaking", false, false) * (1 - amplitude)
          + Theme.strokeFor("speaking", false, true) * amplitude
        : Theme.strokeFor(orbState, busy, false)
    readonly property real stroke: weight * outer
    readonly property real radius: (outer - stroke) / 2
    readonly property real cx: width / 2
    readonly property real cy: height / 2

    // Booting is one short white arc, everything else is closed.
    readonly property real mainStart: orbState === "booting" ? -68 : -90
    readonly property real mainSweep: orbState === "booting" ? 47 : 359.99

    readonly property int blockCount: 7
    readonly property real blockStep: 360 / blockCount
    readonly property real blockSweep: blockStep * 0.32

    function mix(a, b, t) {
        return Qt.rgba(a.r + (b.r - a.r) * t,
                       a.g + (b.g - a.g) * t,
                       a.b + (b.b - a.b) * t, 1);
    }

    readonly property color tint: speaking
        ? mix(Theme.colorFor("speaking", false), Theme.colorFor("speaking", true), amplitude)
        : Theme.colorFor(orbState, false)

    // The ring itself. Booting's short arc drifts slowly round.
    Shape {
        anchors.fill: parent
        visible: !root.working

        ShapePath {
            strokeColor: root.tint
            strokeWidth: root.stroke
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap

            PathAngleArc {
                centerX: root.cx
                centerY: root.cy
                radiusX: root.radius
                radiusY: root.radius
                startAngle: root.mainStart
                sweepAngle: root.mainSweep
            }
        }

        // Standby is two-tone in the render: mint core, cyan rim. A thinner
        // stroke laid on the same circle, not a glow.
        ShapePath {
            strokeColor: root.orbState === "standby" ? Theme.standbyInner : "transparent"
            strokeWidth: root.stroke * 0.42
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap

            PathAngleArc {
                centerX: root.cx
                centerY: root.cy
                radiusX: root.radius
                radiusY: root.radius
                startAngle: root.mainStart
                sweepAngle: root.mainSweep
            }
        }

        NumberAnimation on rotation {
            running: root.orbState === "booting"
            from: 0
            to: 360
            duration: 4200
            loops: Animation.Infinite
        }
    }

    // Working: seven rounded blocks, gaps about twice the block, slowly turning.
    Item {
        id: blocks

        anchors.fill: parent
        visible: root.working

        Repeater {
            model: root.blockCount

            delegate: Shape {
                required property int index

                anchors.fill: parent

                ShapePath {
                    strokeColor: root.tint
                    strokeWidth: root.stroke
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap

                    PathAngleArc {
                        centerX: root.cx
                        centerY: root.cy
                        radiusX: root.radius
                        radiusY: root.radius
                        startAngle: -90 + index * root.blockStep
                        sweepAngle: root.blockSweep
                    }
                }
            }
        }

        NumberAnimation on rotation {
            running: root.working
            from: 0
            to: 360
            duration: 7000
            loops: Animation.Infinite
        }
    }

    // Error: the only failure signal a text-free UI has. Pulses, waits.
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
}
