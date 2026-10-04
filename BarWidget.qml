import QtQuick
import Quickshell
import qs.Ui as Ui

Ui.BarWidget {
    id: root
    moduleName: "io.github.aridev1.omamusi"
    implicitWidth: button.implicitWidth
    implicitHeight: button.implicitHeight

    function launch() {
        const script = Qt.resolvedUrl("scripts/launch-player.sh").toString();
        Quickshell.execDetached(["bash", decodeURIComponent(script.replace(/^file:\/\//, ""))]);
    }

    Ui.WidgetButton {
        id: button
        anchors.fill: parent
        bar: root.bar
        hasVisualContent: true
        labelVisible: false
        fixedWidth: root.barSize
        fixedHeight: root.barSize
        tooltipText: "omaMusi · Open Event Horizon"
        onPressed: function(buttonCode) {
            if (buttonCode === Qt.LeftButton)
                root.launch();
        }

        Image {
            anchors.centerIn: parent
            width: Math.max(16, root.barSize - 8)
            height: width
            source: Qt.resolvedUrl("assets/event-horizon.png")
            fillMode: Image.PreserveAspectFit
            smooth: true
            mipmap: true
        }
    }
}
