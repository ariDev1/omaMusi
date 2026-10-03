import unittest

from omamusi import gpu


class EventHorizonM11TidalShearTests(unittest.TestCase):
    def test_shader_has_tidal_shear_terms(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float tidalShear =", shader)
        self.assertIn("float tidalShearB =", shader)
        self.assertIn("float tidalShearC =", shader)
        self.assertIn("float foregroundSheet =", shader)
        self.assertIn("float foregroundSheetB =", shader)

    def test_shader_softens_flat_shadow(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float edgeFray =", shader)
        self.assertIn("float shadowCore =", shader)
        self.assertIn("float penumbra =", shader)
        self.assertIn("color *= 1.0 - shadowCore * 0.996;", shader)

    def test_shader_keeps_existing_contract_markers(self):
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
            "float streamAsymmetry =",
            "energyStream *= streamAsymmetry;",
            "float heroBandTop =",
            "float heroFlow =",
            "float stressLines =",
            "float magmaFlow =",
            "float perspectiveDrift =",
            "float gravityPull =",
            "screen.y += gravityPull",
            "vec2 horizonCenter = vec2(",
            "vec2 p = screen - horizonCenter;",
            "float antiEyeBias =",
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
            "float photonRing =",
            "float ringSide =",
            "color += orange * heroBandTop",
        )
        for marker in required:
            self.assertIn(marker, shader)


if __name__ == "__main__":
    unittest.main()
