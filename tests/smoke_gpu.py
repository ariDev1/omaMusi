"""Run on the real desktop: GPU rendering, visual continuity, and audio isolation."""

from pathlib import Path
import subprocess
import tempfile
import time

import numpy as np
from PySide6.QtTest import QTest
from PySide6.QtWidgets import QApplication

from omamusi.app import PlayerWindow
from omamusi.visualizer import Visualizer


def main():
    app = QApplication([])
    with tempfile.TemporaryDirectory(prefix="omamusi-gpu-") as directory:
        path = Path(directory) / "tone.wav"
        subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i",
                        "sine=frequency=110:duration=12", str(path)], check=True)
        window = PlayerWindow([])
        window.tracks = [path]
        window.populate()
        window.player.set_volume(0)
        window.show()
        QTest.qWait(300)
        try:
            visual = window.visualizer
            assert visual.renderer == "gpu", visual.renderer_detail
            assert visual._gpu.ready, "GPU shaders did not initialize"
            window.play_track(0)
            QTest.qWait(300)
            sink = window.player.sink
            assert sink is not None
            programs = (visual._gpu.quad, visual._gpu.particles, visual._gpu.phi, visual._gpu.phi_particles, visual._gpu.event_horizon, visual._gpu.vao)
            geometry = visual.particle_vertices.copy()
            for _ in range(len(Visualizer.modes) * 8):
                window.cycle_view()
                QTest.qWait(25)
                assert visual.renderer == "gpu", visual.renderer_detail
                assert visual._gpu.ready
                assert window.player.sink is sink
            before = window.player.worker_snapshot
            time.sleep(0.65)
            after = window.player.worker_snapshot
            assert after["position"] > before["position"] + 0.5, (before, after)
            assert after["written"] > before["written"]
            assert after["underruns"] == 0, after
            np.testing.assert_array_equal(geometry, visual.particle_vertices)
            assert programs == (visual._gpu.quad, visual._gpu.particles, visual._gpu.phi, visual._gpu.phi_particles, visual._gpu.event_horizon, visual._gpu.vao)
            window.player.toggle_pause()
            QTest.qWait(100)
            visual.timer.stop()
            visual._gpu.update()
            QTest.qWait(50)
            frames = []
            for expected in range(len(Visualizer.modes)):
                assert visual.mode == expected
                image = visual._gpu.grabFramebuffer()
                assert not image.isNull()
                frames.append(bytes(image.bits()))
                window.cycle_view()
                QTest.qWait(30)
            assert len(set(frames)) == len(Visualizer.modes)
            # Freeze the clock: cycling out and back must reproduce the exact tunnel.
            image = visual._gpu.grabFramebuffer()
            returned = bytes(image.bits())
            assert returned == frames[0], "Warp star seeds changed when cycling views"
            print(f"GPU: {visual.renderer_detail}")
            print(f"PASS: all GPU views including Phi Cathedral, persistent resources, {len(Visualizer.modes) * 8} switches, 650 ms UI stall, zero audio underruns")
        finally:
            window.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
