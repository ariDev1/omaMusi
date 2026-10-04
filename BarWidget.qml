pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
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

    Ui.BarIconButton {
        id: button
        anchors.fill: parent
        bar: root.bar
        tooltipText: "omaMusi · Open Event Horizon"
        onPressed: function(buttonCode) {
            if (buttonCode === Qt.LeftButton)
                root.launch();
        }

        iconComponent: Component {
            Item {
                Shape {
                    anchors.centerIn: parent
                    width: 24
                    height: 24
                    scale: Math.min(parent.width, parent.height) / 24
                    rotation: -24

                    ShapePath {
                        strokeColor: button.foreground
                        strokeWidth: 1.6
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap
                        joinStyle: ShapePath.RoundJoin
                        PathSvg {
                            path: "M 19 12 A 7 7 0 1 1 5 12 A 7 7 0 1 1 19 12 "
                                + "M 22.5 12 A 10.5 2.8 0 1 1 1.5 12 A 10.5 2.8 0 1 1 22.5 12"
                        }
                    }
                }
            }
        }
    }
}
