import inspect
import unittest
from pathlib import Path

from omamusi import gpu
from omamusi.visualizer import Visualizer


class EventHorizonFlybyTests(unittest.TestCase):
    def test_shader_has_camera_flyby_terms(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float fly =", shader)
        self.assertIn("vec2 cameraLoop = vec2(", shader)
        self.assertIn("float cameraRoll =", shader)
        self.assertIn("float inclination =", shader)
        self.assertIn("float foreshorten =", shader)

    def test_cpu_fallback_has_cinematic_accetion_lens(self):
        text = Path(inspect.getfile(Visualizer)).read_text()
        self.assertIn('"""CPU fallback for the Event Horizon cinematic accretion-lens visual."""', text)
        self.assertIn("band_count = 30", text)
        self.assertIn("# Rich accretion disc.", text)

    def test_event_horizon_shader_symbol_is_isolated(self):
        text = Path(inspect.getfile(gpu)).read_text()
        self.assertEqual(text.count("EVENT_HORIZON_FRAGMENT"), 2)


if __name__ == "__main__":
    unittest.main()
