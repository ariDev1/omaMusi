import unittest

from omamusi import gpu


class EventHorizonM6CinematicDiscTests(unittest.TestCase):
    def test_shader_has_directional_disc_asymmetry(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float asymmetry =", shader)
        self.assertIn("float doppler =", shader)
        self.assertIn("float observerSide =", shader)

    def test_shader_strengthens_background_lensing(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float starBendMask =", shader)
        self.assertIn("float lensWarp =", shader)
        self.assertIn("float starsHalo =", shader)

    def test_shader_keeps_bent_disc_topology(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float upperWarp =", shader)
        self.assertIn("float lowerWarp =", shader)
        self.assertIn("float discPlane =", shader)
        self.assertIn("float discHalfThickness =", shader)

    def test_shader_keeps_m5_contract_markers(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        required = (
            "vec2 lensedBackground =",
            "float starsFine =",
            "float starsCoarse =",
            "float directDisc = annulus(",
            "float lensFlow =",
            "vec3(1.0) - exp(-color * exposure)",
            "uniform float aetherBeatPulse;",
        )
        for marker in required:
            self.assertIn(marker, shader)


if __name__ == "__main__":
    unittest.main()
