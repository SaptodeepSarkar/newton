pragma Singleton

import QtQuick

// One place for every number the ring uses. The state names here must match
// OrbState::name(): there is a test for that string, so a rename breaks CI.
// Colours are the measured renders in docs/UI/Visual Spec.md.
QtObject {
    readonly property color surface: "#1b1917"

    readonly property color booting: "#fbfaf8"
    readonly property color standby: "#44f7f4"
    readonly property color standbyInner: "#9df5c0"
    readonly property color listening: "#f1f59c"
    readonly property color speaking: "#776eec"
    readonly property color speakingLoud: "#9893f5"
    // No error render exists (Visual Spec, open item). Kept from before the pass.
    readonly property color error: "#e5484d"

    readonly property int ringDiameter: 300
    readonly property int fastMs: 150
    readonly property int baseMs: 200
    readonly property int slowMs: 300

    function colorFor(state, loud) {
        switch (state) {
        case "booting":
            return booting;
        case "standby":
            return standby;
        case "listening":
            return listening;
        case "speaking":
            return loud ? speakingLoud : speaking;
        case "error":
            return error;
        default:
            return booting;
        }
    }

    // Stroke as a ratio of the ring diameter, never pixels.
    function strokeFor(state, busy, loud) {
        switch (state) {
        case "booting":
            return 0.011;
        case "standby":
            return 0.22;
        case "listening":
            return busy ? 0.12 : 0.18;
        case "speaking":
            return loud ? 0.25 : 0.12;
        case "error":
            return 0.18;
        default:
            return 0.011;
        }
    }
}
