import unittest

from omamusi import gpu


class EventHorizonM9EdgefallTests(unittest.TestCase):
    def test_shader_has_edge_proximity_terms(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float edgeFeeling =", shader)
        self.assertIn("float edgeZoom =", shader)
        self.assertIn("float shadowRadius = 0.170", shader)

    def test_shader_has_large_energy_streams(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float streamA =", shader)
        self.assertIn("float streamB =", shader)
        self.assertIn("float streamC =", shader)
        self.assertIn("float energyStream =", shader)

    def test_shader_has_brighter_hero_bands(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float heroBandTop =", shader)
        self.assertNotIn("float heroBandBottom =", shader)
        self.assertIn("color += orange * heroBandTop", shader)

    def test_shader_keeps_m8_contract_markers(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        required = (
            "float phraseSurge =",
            "float beatSurge =",
            "float stressLines =",
            "float magmaFlow =",
            "float perspectiveDrift =",
            "float gravityPull =",
            "float tangentialBelt =",
            "float ringBreak =",
            "float bentContinuity =",
        )
        for marker in required:
            self.assertIn(marker, shader)


if __name__ == "__main__":
    unittest.main()
