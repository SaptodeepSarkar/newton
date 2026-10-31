pragma Singleton

import QtQuick

// One place for every number the ring uses. The state names here must match
// OrbState::name(): there is a test for that string, so a rename breaks CI.
QtObject {
    readonly property color surface: "#2b2b2d"

    readonly property color booting: "#ededed"
    readonly property color standby: "#22e37a"
    readonly property color standbyInner: "#9df5c0"
    readonly property color listening: "#f2e14c"
    readonly property color speaking: "#b07cf0"
    readonly property color error: "#e5484d"

    readonly property int ringDiameter: 300
    readonly property int fastMs: 150
    readonly property int baseMs: 200
    readonly property int slowMs: 300

    function colorFor(state) {
        switch (state) {
        case "booting":
            return booting;
        case "standby":
            return standby;
        case "listening":
            return listening;
        case "speaking":
            return speaking;
        case "error":
            return error;
        default:
            return booting;
        }
    }

    function strokeFor(state, busy) {
        switch (state) {
        case "booting":
            return 3;
        case "standby":
            return 40;
        case "listening":
            return busy ? 12 : 30;
        case "speaking":
            return 14;
        case "error":
            return 30;
        default:
            return 4;
        }
    }
}
