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

    def test_black_sphere_has_glossy_shading(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float ballMask =", shader)
        self.assertNotIn("specGloss", shader)
        self.assertIn("float rimLight =", shader)
        self.assertIn("float ballShade =", shader)

    def test_shiny_border_sits_on_the_bottom(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float bottomGate =", shader)
        self.assertIn("float lowerBorder =", shader)
        self.assertIn("cos(a + 1.5708)", shader)

    def test_gargantua_thin_arcs_and_spine(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float diskSpine =", shader)
        self.assertIn("float diskGlow =", shader)
        self.assertIn("float haloGlow =", shader)
        self.assertIn("mix(0.10, 0.24, 0.5 + 0.5 * sin(fly * 0.84", shader)
        self.assertIn("topArcRadius - 0.008", shader)
        self.assertIn("a*7.0 - phase", shader)


if __name__ == "__main__":
    unittest.main()
