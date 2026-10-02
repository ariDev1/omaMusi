import unittest

from omamusi import gpu


class EventHorizonPerspectiveTests(unittest.TestCase):
    def test_shader_has_disc_plane_and_lensing_arcs(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float discCore =", shader)
        self.assertIn("float upperLens =", shader)
        self.assertIn("float lowerLens =", shader)
        self.assertIn("float doppler =", shader)

    def test_shader_has_plasma_texture_terms(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float plasmaBands =", shader)
        self.assertIn("float fineFilaments =", shader)
        self.assertIn("float hotKnots =", shader)


if __name__ == "__main__":
    unittest.main()
