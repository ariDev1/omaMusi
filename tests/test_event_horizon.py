import unittest

from omamusi.visualizer import Visualizer
from omamusi import gpu


class EventHorizonTests(unittest.TestCase):
    def test_event_horizon_mode_exists(self):
        self.assertIn("Event Horizon", Visualizer.modes)
        self.assertEqual(Visualizer.modes[5], "Event Horizon")
        self.assertEqual(Visualizer.modes[-1], "Particle Dance")

    def test_event_horizon_render_path_exists(self):
        from pathlib import Path
        source = Path(gpu.__file__).read_text()
        self.assertIn("elif state.mode == 5:", source)
        self.assertIn("self.event_horizon", source)

    def test_event_horizon_shader_exists(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("shadowRadius", shader)
        self.assertIn("upperLens", shader)
        self.assertIn("lowerLens", shader)
        self.assertIn("discCore", shader)
        self.assertIn("shockRadius", shader)
        self.assertIn("glimpseGate", shader)


if __name__ == "__main__":
    unittest.main()
