import QtQuick
import QtQuick.Window
import Newton

Window {
    id: win

    property var orb: null

    width: 420
    height: 420
    visible: true
    color: Theme.surface
    title: qsTr("Newton")

    Ring {
        id: ring

        width: Math.min(parent.width, parent.height) * 0.72
        height: width
        anchors.centerIn: parent
        orbState: win.orb ? win.orb.state : "booting"
        busy: win.orb ? win.orb.busy : false
    }

    Item {
        anchors.fill: parent
        focus: true

        Keys.onPressed: function (event) {
            if (event.key === Qt.Key_Space) {
                if (win.orb)
                    win.orb.advance();
                event.accepted = true;
            } else if (event.key === Qt.Key_Escape) {
                Qt.quit();
            }
        }
    }
}
