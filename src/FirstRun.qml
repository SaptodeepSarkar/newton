import QtQuick
import Newton

Rectangle {
    id: screen

    signal done()
    property bool finished: false

    // The MouseArea delegates here so a headless test can drive the same path.
    function activate() {
        finished = true;
        done();
    }

    color: Theme.surface
    implicitWidth: 420
    implicitHeight: 420

    Column {
        anchors.centerIn: parent
        width: parent.width * 0.8
        spacing: 22

        Text {
            width: parent.width
            color: Theme.booting
            font.pixelSize: 20
            wrapMode: Text.WordWrap
            text: qsTr("Three things before we start")
        }

        Column {
            width: parent.width
            spacing: 12

            Repeater {
                model: [
                    qsTr("Newton needs your microphone to hear the wake word."),
                    qsTr("Say \"Newton\" to wake it. Nothing before that is recorded."),
                    qsTr("Google is optional and read only. Turning it off changes nothing else.")
                ]

                Text {
                    required property string modelData

                    width: screen.width * 0.8
                    color: "#b8b8ba"
                    font.pixelSize: 14
                    wrapMode: Text.WordWrap
                    text: modelData
                }
            }
        }

        Rectangle {
            width: 140
            height: 40
            radius: 20
            color: Theme.standby

            Text {
                anchors.centerIn: parent
                color: Theme.surface
                font.pixelSize: 14
                text: qsTr("Continue")
            }

            MouseArea {
                objectName: "continueButton"
                anchors.fill: parent
                onClicked: screen.activate()
            }
        }
    }
}
