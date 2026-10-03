import unittest
from pathlib import Path

from omamusi import gpu
from omamusi.visualizer import Visualizer


class EventHorizonRegressionTests(unittest.TestCase):
    def test_cpu_fallback_has_dedicated_event_horizon_path(self):
        source = Path(Visualizer.__module__.replace(".", "/") + ".py")
        # Module path may not be relative to cwd in all runners; inspect actual file.
        import inspect
        text = Path(inspect.getfile(Visualizer)).read_text()
        self.assertIn("def paint_event_horizon", text)
        self.assertIn("elif self.mode == 4:", text)
        self.assertIn("self.paint_reference_horizon(p, w, h)", text)

    def test_gpu_program_is_registered_for_common_uniforms(self):
        import inspect
        text = Path(inspect.getfile(gpu)).read_text()
        self.assertIn(
            "(self.quad, self.particles, self.phi, self.phi_particles, self.event_horizon)",
            text,
        )

    def test_event_horizon_uses_fullscreen_triangle(self):
        import inspect
        text = Path(inspect.getfile(gpu)).read_text()
        start = text.index("elif state.mode == 5:")
        block = text[start:text.index("elif state.mode == 6:", start)]
        self.assertIn("GL.glDrawArrays(GL.GL_TRIANGLES, 0, 3)", block)


if __name__ == "__main__":
    unittest.main()
