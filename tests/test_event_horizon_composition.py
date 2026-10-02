import unittest

from omamusi import gpu


class EventHorizonCompositionTests(unittest.TestCase):
    def test_horizon_is_center_based_with_camera_offsets(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("vec2 p = uv - (vec2(0.5) + cameraLoop + cameraKick);", shader)
        self.assertIn("vec2 cameraLoop = vec2(", shader)
        self.assertIn("vec2 cameraKick = vec2(", shader)

    def test_flyby_has_zoom_foreshortening_and_shadow(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float cameraZoom =", shader)
        self.assertIn("float foreshorten =", shader)
        self.assertIn("float shadowRadius =", shader)
        self.assertIn("float shadowMask =", shader)

    def test_old_corner_origin_is_not_used(self):
        self.assertNotIn("vec2 p = uv;\n", gpu.EVENT_HORIZON_FRAGMENT)


if __name__ == "__main__":
    unittest.main()
