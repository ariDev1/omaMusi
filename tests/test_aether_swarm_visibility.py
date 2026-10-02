import unittest
from omamusi import gpu

class AetherSwarmVisibilityTests(unittest.TestCase):
    def test_swarm_render_uses_directional_streaks(self):
        self.assertIn("out vec2 streakDir;", gpu.AETHER_SWARM_RENDER_VERTEX)
        self.assertIn("out float streakMix;", gpu.AETHER_SWARM_RENDER_VERTEX)
        self.assertIn("in vec2 streakDir;", gpu.AETHER_SWARM_RENDER_FRAGMENT)
        self.assertIn("gl_PointCoord", gpu.AETHER_SWARM_RENDER_FRAGMENT)

    def test_swarm_update_uses_shell_coherence(self):
        self.assertIn("shellForce", gpu.AETHER_SWARM_UPDATE_VERTEX)
        self.assertIn("float shell =", gpu.AETHER_SWARM_UPDATE_VERTEX)
        self.assertIn("float phrase =", gpu.AETHER_SWARM_UPDATE_VERTEX)

if __name__ == "__main__":
    unittest.main()
