import inspect
import unittest
from pathlib import Path

from omamusi import gpu
from omamusi.visualizer import Visualizer


class EventHorizonM3AccretionTests(unittest.TestCase):
    def test_shader_uses_accretion_palette_and_disc_terms(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("vec3 ember", shader)
        self.assertIn("vec3 orange", shader)
        self.assertIn("vec3 hot", shader)
        self.assertIn("float discCore =", shader)
        self.assertIn("float heat =", shader)
        self.assertIn("float upperLens =", shader)
        self.assertIn("float lowerLens =", shader)

    def test_cpu_fallback_draws_accretion_disc(self):
        text = Path(inspect.getfile(Visualizer)).read_text()
        self.assertIn("band_count = 30", text)
        self.assertIn("# Rich accretion disc.", text)
        self.assertIn("# Upper and lower lens arcs.", text)


if __name__ == "__main__":
    unittest.main()
