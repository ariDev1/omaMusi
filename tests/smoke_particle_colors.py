"""Hardware regression: loud Particle Dance must stay colorful, not white."""

import sys
import math
import time
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import numpy as np
from PySide6.QtGui import QImage
from PySide6.QtTest import QTest
from PySide6.QtWidgets import QApplication

from omamusi.visualizer import Visualizer


def pixels(image):
    image = image.convertToFormat(QImage.Format.Format_RGBA8888)
    rows = np.frombuffer(image.constBits(), dtype=np.uint8).reshape(
        image.height(), image.bytesPerLine())
    return rows[:, :image.width() * 4].reshape(image.height(), image.width(), 4)[..., :3].copy()


app = QApplication([])
visual = Visualizer()
visual.timer.stop()
visual.mode = 6
visual.resize(960, 540)
visual.colors["background"] = "#000000"
visual.show()
QTest.qWait(100)
assert visual.renderer == "gpu", visual.renderer_detail
canvas = visual._gpu
assert canvas.ready, visual.renderer_detail
# Freeze geometry: compare quiet and loud uniforms on the same swarm.
update_swarm = canvas._update_aether_swarm
canvas._update_aether_swarm = lambda: None
metrics = ("energy", "bass", "treble", "phi_impulse", "phi_event",
           "aether_onset", "aether_density", "aether_beat_pulse")
for name, level in (("quiet", 0.0), ("loud", 1.0)):
    for metric in metrics:
        setattr(visual, metric, level)
    visual.bands.fill(level)
    visual.time = 12.0
    visual.aether_beat_confidence = level
    image = canvas.grabFramebuffer()
    rgb = pixels(image)
    near_white = (rgb.min(axis=2) > 220).mean()
    lit = rgb.max(axis=2) > 30
    colorful = lit & ((rgb.max(axis=2).astype(float) - rgb.min(axis=2)) > 25)
    color_fraction = colorful.sum() / max(1, lit.sum())
    assert near_white < 0.001, (name, "white-out", near_white)
    assert color_fraction > 0.65, (name, "lost saturation", color_fraction)
    assert lit.mean() > 0.001, (name, "particles disappeared")
    # Isolate a real particle using the same compiled shader and frozen
    # geometry. Loud input must not grow it into a large blob.
    with patch('omamusi.gpu.AETHER_SWARM_COUNT', 1):
        isolated = pixels(canvas.grabFramebuffer())
    ys, xs = np.nonzero(isolated.max(axis=2) > 2)
    assert len(xs), (name, 'isolated particle disappeared')
    limit = math.ceil(3 * canvas.devicePixelRatioF())
    assert np.ptp(xs) + 1 <= limit and np.ptp(ys) + 1 <= limit, (name, 'particle grew', np.ptp(xs) + 1, np.ptp(ys) + 1, limit)
    if len(sys.argv) > 1:
        assert image.save(str(Path(sys.argv[1]) / f"particle-{name}.png"))
    print(f"{name}: white={near_white:.3%}, colorful={color_fraction:.1%}")
canvas._update_aether_swarm = update_swarm
# Settle the actual physics under sustained maximum input. The gold ring
# must remain visible, without the surrounding swarm bleaching white.
for frame in range(240):
    visual.time = 12 + frame / 30 * .65
    visual.aether_world_turn = frame / 30 * .12
    visual.aether_beat_phase = (frame / 30 * 1.8) % 1
    canvas.swarm_clock = time.monotonic() - 1 / 30
    settled = canvas.grabFramebuffer()
rgb = pixels(settled).astype(float)
if len(sys.argv) > 1:
    assert settled.save(str(Path(sys.argv[1]) / 'particle-settled-loud.png'))
gold = (rgb[..., 0] > 15) & (rgb[..., 0] > rgb[..., 1] * 1.25) & (rgb[..., 0] > rgb[..., 2] * 1.8)
assert gold.mean() > .001, ('gold ring disappeared', gold.mean())
edge = gold.copy()
height, width = gold.shape
edge[int(height * .05):int(height * .95), int(width * .05):int(width * .95)] = False
assert edge.sum() / gold.sum() < .10, ('ring crowds frame edges', edge.sum() / gold.sum())
assert (rgb.min(axis=2) > 220).mean() < .001, 'sustained loud white-out'
print(f'sustained loud: gold pixels={gold.mean():.2%}, small-particle ring remains visible')
visual.close()
print("PASS: quiet and maximum-audio Particle Dance retain color")
