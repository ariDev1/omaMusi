import unittest
from pathlib import Path

class NavigationIdleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = (Path(__file__).parents[1] / "omamusi" / "app.py").read_text()

    def test_navigation_fade_keeps_song_widgets_out_of_fade_group(self):
        start = self.source.index("        for widget in (")
        end = self.source.index("        ):", start)
        fade_group = self.source[start:end]
        self.assertIn("self.panel", fade_group)
        self.assertIn("self.hint", fade_group)
        self.assertNotIn("self.title", fade_group)
        self.assertNotIn("self.subtitle", fade_group)
        self.assertNotIn("self.transport", fade_group)

    def test_navigation_returns_on_interaction(self):
        self.assertIn("QEvent.Type.KeyPress", self.source)
        self.assertIn("QEvent.Type.MouseButtonPress", self.source)
        self.assertIn("QEvent.Type.MouseMove", self.source)
        self.assertIn("QEvent.Type.Wheel", self.source)

    def test_idle_timeout_is_four_seconds(self):
        self.assertIn("self.navigation_idle_timer.setInterval(4000)", self.source)
        self.assertIn("self.navigation_idle_timer.timeout.connect(self._fade_navigation)", self.source)

if __name__ == "__main__":
    unittest.main()
