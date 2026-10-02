import unittest

from omamusi import gpu


class EventHorizonM7TangentialFlowTests(unittest.TestCase):
    def test_shader_has_tangential_disc_terms(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float tangentialBelt =", shader)
        self.assertIn("float tangentialFlow =", shader)
        self.assertIn("float shearFlow =", shader)

    def test_shader_softens_photon_ring(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float ringBreak =", shader)
        self.assertIn("float photonRing =", shader)
        self.assertIn("* ringBreak *", shader)

    def test_shader_bends_far_side_as_disc_extensions(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float bentContinuity =", shader)
        self.assertIn("upperLens *=", shader)
        self.assertIn("lowerLens *=", shader)

    def test_shader_keeps_m6_contract_markers(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        required = (
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
