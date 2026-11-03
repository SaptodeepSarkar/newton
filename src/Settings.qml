import QtQuick
import Newton

Rectangle {
    id: screen

    signal muteChanged(bool muted)
    signal forgetAll()

    function toggleMute() {
        muted = !muted;
        muteChanged(muted);
    }

    // One button, two taps: the first arms it, the second actually forgets.
    function forget() {
        if (!confirming) {
            confirming = true;
            return;
        }
        confirming = false;
        forgetAll();
    }

    property bool muted: false
    property bool confirming: false

    color: Theme.surface
    implicitWidth: 420
    implicitHeight: 420

    Column {
        anchors.centerIn: parent
        width: parent.width * 0.8
        spacing: 24

        Row {
            width: parent.width
            spacing: 14

            Text {
                anchors.verticalCenter: parent.verticalCenter
                color: Theme.booting
                font.pixelSize: 15
                text: qsTr("Mute microphone")
            }

            Rectangle {
                width: 46
                height: 24
                radius: 12
                color: screen.muted ? Theme.standby : "#4a4a4d"

                MouseArea {
                    objectName: "muteToggle"
                    anchors.fill: parent
                    onClicked: screen.toggleMute()
                }
            }
        }

        Text {
            objectName: "localOnly"
            width: parent.width
            color: "#b8b8ba"
            font.pixelSize: 14
            wrapMode: Text.WordWrap
            text: qsTr("Memory and models stay on this device. Nothing is sent anywhere.")
        }

        Rectangle {
            width: 180
            height: 40
            radius: 20
            color: screen.confirming ? Theme.error : "#4a4a4d"

            Text {
                anchors.centerIn: parent
                color: Theme.booting
                font.pixelSize: 14
                text: screen.confirming ? qsTr("Tap again to confirm")
                                       : qsTr("Forget everything")
            }

            MouseArea {
                objectName: "forgetButton"
                anchors.fill: parent
                onClicked: screen.forget()
            }
        }
    }
}
