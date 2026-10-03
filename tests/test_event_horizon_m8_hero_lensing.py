import unittest

from omamusi import gpu


class EventHorizonM8HeroLensingTests(unittest.TestCase):
    def test_shader_has_hero_lensing_bands(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float heroBandTop =", shader)
        self.assertNotIn("float heroBandBottom =", shader)
        self.assertIn("float heroFlow =", shader)

    def test_shader_has_plasma_surge_layers(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float phraseSurge =", shader)
        self.assertIn("float beatSurge =", shader)
        self.assertIn("float stressLines =", shader)
        self.assertIn("float magmaFlow =", shader)

    def test_shader_has_cinematic_camera_drift(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float perspectiveDrift =", shader)
        self.assertIn("float gravityPull =", shader)
        self.assertIn("screen.y += gravityPull", shader)

    def test_shader_keeps_m7_contract_markers(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        required = (
            "float tangentialBelt =",
            "float tangentialFlow =",
            "float shearFlow =",
            "float ringBreak =",
            "float bentContinuity =",
            "float asymmetry =",
            "float starBendMask =",
            "float lensWarp =",
            "float starsHalo =",
            "float upperWarp =",
            "float lowerWarp =",
            "float discPlane =",
            "float discHalfThickness =",
        )
        for marker in required:
            self.assertIn(marker, shader)


if __name__ == "__main__":
    unittest.main()
