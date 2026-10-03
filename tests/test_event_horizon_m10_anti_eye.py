import unittest

from omamusi import gpu


class EventHorizonM10AntiEyeTests(unittest.TestCase):
    def test_shader_has_off_axis_horizon(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("vec2 horizonCenter = vec2(", shader)
        self.assertIn("vec2 p = screen - horizonCenter;", shader)
        self.assertIn("float antiEyeBias =", shader)

    def test_shader_replaces_iris_with_sheet_streams(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float streamAsymmetry =", shader)
        self.assertIn("energyStream *= streamAsymmetry;", shader)
        self.assertIn("float photonRing =", shader)
        self.assertIn("float ringSide =", shader)

    def test_shader_keeps_prior_contract_markers(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        required = (
            "float phraseSurge =",
            "float beatSurge =",
            "float edgeFeeling =",
            "float edgeZoom =",
            "float shadowRadius = 0.170",
            "float streamA =",
            "float streamB =",
            "float streamC =",
            "float energyStream =",
            "float heroBandTop =",
            "float heroFlow =",
            "float stressLines =",
            "float magmaFlow =",
            "float perspectiveDrift =",
            "float gravityPull =",
            "screen.y += gravityPull",
            "float tangentialBelt =",
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
            "color += orange * heroBandTop",
        )
        for marker in required:
            self.assertIn(marker, shader)


if __name__ == "__main__":
    unittest.main()
