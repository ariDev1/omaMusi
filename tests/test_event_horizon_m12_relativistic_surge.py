import unittest

from omamusi import gpu


class EventHorizonM12RelativisticSurgeTests(unittest.TestCase):
    def test_shader_has_left_side_activity(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float leftSurge =", shader)
        self.assertIn("float leftWake =", shader)
        self.assertIn("energyStream += leftSurge * 0.52 + leftWake * 0.42;", shader)

    def test_shader_has_foreground_depth_layers(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float foregroundDustVeil =", shader)
        self.assertIn("float foregroundDustStreak =", shader)
        self.assertIn("float dustFront = 0.0;", shader)

    def test_shader_has_lensing_and_disc_coherence_refinement(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float lensEdgeComplexity =", shader)
        self.assertIn("float discLaneCoherence =", shader)
        self.assertIn("float discLaneContrast =", shader)

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
            "float heroBandBottom =",
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
            "float tidalShear =",
            "float tidalShearB =",
            "float tidalShearC =",
            "float foregroundSheet =",
            "float foregroundSheetB =",
            "float edgeFray =",
            "float shadowFray =",
            "color *= 1.0 - shadowFray * 0.992;",
            "color += orange * heroBandTop",
        )
        for marker in required:
            self.assertIn(marker, shader)


if __name__ == "__main__":
    unittest.main()
