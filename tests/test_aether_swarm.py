import unittest
from pathlib import Path
from omamusi import gpu

class AetherSwarmTests(unittest.TestCase):
    def test_fibonacci_swarm_count(self):
        self.assertEqual(gpu.AETHER_SWARM_COUNT, 2584)

    def test_persistent_swarm_uses_transform_feedback(self):
        source = Path(gpu.__file__).read_text()
        self.assertIn("glBeginTransformFeedback", source)
        self.assertIn("GL_RASTERIZER_DISCARD", source)
        self.assertIn("swarm_vbos", source)

    def test_swarm_has_separate_update_and_render_shaders(self):
        self.assertIn("outPosition", gpu.AETHER_SWARM_UPDATE_VERTEX)
        self.assertIn("outVelocity", gpu.AETHER_SWARM_UPDATE_VERTEX)
        self.assertIn("inPosition", gpu.AETHER_SWARM_RENDER_VERTEX)
        self.assertIn("inVelocity", gpu.AETHER_SWARM_RENDER_VERTEX)

if __name__ == "__main__":
    unittest.main()
