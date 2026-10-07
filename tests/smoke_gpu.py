"""Run on the real desktop: GPU rendering, visual continuity, and audio isolation."""

from pathlib import Path
import subprocess
import tempfile
import time

import numpy as np
from PySide6.QtTest import QTest
from PySide6.QtGui import QColor, QImage
from PySide6.QtWidgets import QApplication

from omamusi.app import PlayerWindow
from omamusi.visualizer import Visualizer
from omamusi.visual.cover import cover_rectangle


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
            deadline = time.monotonic() + 5
            while window.player.sink is None and time.monotonic() < deadline:
                QTest.qWait(20)
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
                expected_alpha = 0 if expected == 2 else 255
                assert image.pixelColor(1, 1).alpha() == expected_alpha, (expected, image.pixelColor(1, 1))
                frames.append(bytes(image.bits()))
                window.cycle_view()
                QTest.qWait(30)
            assert len(set(frames)) == len(Visualizer.modes)
            # Freeze the clock: cycling out and back must reproduce the exact tunnel.
            image = visual._gpu.grabFramebuffer()
            returned = bytes(image.bits())
            assert returned == frames[0], "Warp star seeds changed when cycling views"
            # Exercise the density upload and real alpha-capable framebuffer.
            visual.mode = 2
            density = visual.wave_persistence
            density.reset()
            density.step(1 / 30, np.zeros(1024))
            density.latest_alpha = 0
            cold = visual._gpu.grabFramebuffer()
            x, y = cold.width() // 2, int(cold.height() * 0.47)
            cold_color = cold.pixelColor(x, y)
            assert cold_color.alpha() > 0 and cold_color.blue() > cold_color.red(), cold_color
            for _ in range(60):
                density.step(1 / 30, np.zeros(1024))
            density.latest_alpha = 0
            hot = visual._gpu.grabFramebuffer()
            hot_color = hot.pixelColor(x, y)
            assert hot_color.red() > hot_color.blue(), hot_color
            assert hot.pixelColor(1, 1).alpha() == 0
            assert window.grab().toImage().pixelColor(1, 1).alpha() == 0
            density.reset()
            cleared = visual._gpu.grabFramebuffer()
            assert cleared.pixelColor(x, y).alpha() == 0
            visual.mode = 0
            assert visual._gpu.grabFramebuffer().pixelColor(1, 1).alpha() == 255
            # QPainter on the GPU canvas must render real cover images, clear
            # them to black, and leave the original GL modes intact.
            visual.mode = Visualizer.modes.index("Cover Art")
            art = QImage(200, 100, QImage.Format.Format_RGB32)
            art.fill(QColor("red"))
            visual.set_cover_art(art)
            gallery = visual._gpu.grabFramebuffer()
            center = cover_rectangle(visual.width(), visual.height(), art.size(), visual.bass).center()
            ratio = visual._gpu.devicePixelRatioF()
            color = gallery.pixelColor(int(center.x() * ratio), int(center.y() * ratio))
            assert color == QColor("red"), (color.getRgb(), center, ratio, gallery.size())
            assert gallery.pixelColor(1, 1) == QColor("black")
            visual.set_cover_art(QImage())
            empty = visual._gpu.grabFramebuffer()
            assert empty.pixelColor(empty.width() // 2, empty.height() // 2) == QColor("black")
            # Real gallery thumbnails and the transition compositor must work
            # on the GPU canvas, including an opaque, balanced midpoint.
            visual.mode = Visualizer.modes.index("Cover Gallery")
            visual.gallery.clear()
            visual.gallery.add(path, art)
            blue = QImage(200, 100, QImage.Format.Format_RGB32)
            blue.fill(QColor("blue"))
            visual.gallery.add(path.with_name('other.wav'), blue)
            rectangle = visual.gallery.layout(window.gallery_bounds())[0]
            center = rectangle.center()
            gallery_image = visual._gpu.grabFramebuffer()
            color = gallery_image.pixelColor(int(center.x() * ratio), int(center.y() * ratio))
            assert color in (QColor("red"), QColor("blue")), color.getRgb()
            saved_bass = visual.bass
            visual.bass = 0
            quiet_gallery = visual._gpu.grabFramebuffer()
            visual.bass = 1
            loud_gallery = visual._gpu.grabFramebuffer()
            assert bytes(quiet_gallery.bits()) == bytes(loud_gallery.bits()), "Gallery covers pulse with music"
            visual.bass = saved_bass
            visual.gallery.tick(6)
            visual.gallery.tiles[0].fade = .5
            midpoint = visual._gpu.grabFramebuffer()
            color = midpoint.pixelColor(int(center.x() * ratio), int(center.y() * ratio))
            assert abs(color.red() - 128) <= 2 and abs(color.blue() - 128) <= 2, color.getRgb()
            assert color.alpha() == 255
            visual.mode = 0
            restored = visual._gpu.grabFramebuffer()
            assert bytes(restored.bits()) == frames[0], "Cover visuals changed the Warp rendering state"
            print(f"GPU: {visual.renderer_detail}")
            print(f"PASS: all GPU views, gallery thumbnails and crossfade, waveform density and transparency, persistent resources, {len(Visualizer.modes) * 8} switches, 650 ms UI stall, zero audio underruns")
        finally:
            window.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
