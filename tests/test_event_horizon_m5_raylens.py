import inspect
import unittest
from pathlib import Path

from omamusi import gpu


class EventHorizonM5RayLensTests(unittest.TestCase):
    def test_shader_lenses_background_stars(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("vec2 lensedBackground =", shader)
        self.assertIn("float starsFine =", shader)
        self.assertIn("float starsCoarse =", shader)

    def test_shader_projects_annular_disc(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float directDisc = annulus(", shader)
        self.assertIn("float discInner =", shader)
        self.assertIn("float discOuter =", shader)

    def test_secondary_disc_images_share_flow_texture(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("float lensFlow =", shader)
        self.assertIn("upperLens *=", shader)
        self.assertIn("lowerLens *=", shader)

    def test_shader_uses_aether_choreography(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        for name in (
            "aetherOnset",
            "aetherBeatPhase",
            "aetherDensity",
            "aetherSceneMorph",
            "aetherWorldTurn",
            "aetherBeatPulse",
        ):
            self.assertIn("uniform float " + name, shader)

    def test_shader_has_filmic_tonemapping(self):
        shader = gpu.EVENT_HORIZON_FRAGMENT
        self.assertIn("vec3(1.0) - exp(-color * exposure)", shader)
        self.assertIn("float grain =", shader)

    def test_gpu_smoke_tracks_event_horizon_program(self):
        text = Path(inspect.getfile(gpu)).parent.parent.joinpath("tests", "smoke_gpu.py").read_text()
        self.assertIn("visual._gpu.event_horizon", text)


if __name__ == "__main__":
    unittest.main()
