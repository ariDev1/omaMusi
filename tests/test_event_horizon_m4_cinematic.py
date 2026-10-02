import inspect
import unittest
from pathlib import Path

from omamusi import gpu
from omamusi.visualizer import Visualizer


class EventHorizonM4CinematicTests(unittest.TestCase):
    def test_shader_uses_warm_cinematic_palette(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("vec3 ember", shader)
        self.assertIn("vec3 orange", shader)
        self.assertIn("vec3 hot", shader)
        self.assertIn("vec3 whiteHot", shader)

    def test_cpu_fallback_draws_lens_arcs_and_shadow(self):
        text = Path(inspect.getfile(Visualizer)).read_text()
        self.assertIn("# Upper and lower lens arcs.", text)
        self.assertIn("# Shadow and photon ring.", text)
        self.assertIn("shadow_r =", text)


if __name__ == "__main__":
    unittest.main()
