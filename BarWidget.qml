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
                clip: true
                Shape {
                    anchors.centerIn: parent
                    width: 24
                    height: 24
                    scale: Math.min(parent.width, parent.height) / 24

                    ShapePath {
                        strokeColor: "transparent"
                        fillColor: button.foreground
                        fillRule: ShapePath.OddEvenFill
                        PathSvg {
                            // Outer radius / inner radius = the golden ratio.
                            path: "M 22 12 A 10 10 0 1 1 2 12 A 10 10 0 1 1 22 12 Z "
                                + "M 18.18034 12 A 6.18034 6.18034 0 1 1 5.81966 12 "
                                + "A 6.18034 6.18034 0 1 1 18.18034 12 Z"
                        }
                    }
                    ShapePath {
                        strokeColor: "#e9ad42"
                        strokeWidth: 2.36068
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap
                        PathSvg {
                            path: "M 0.75 17.5 L 23.25 6.5"
                        }
                    }
                }
            }
        }
    }
}
