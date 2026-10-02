import unittest
import numpy as np

from omamusi.visual.analysis import MusicAnalyzer
from omamusi.visual.director import VisualDirector


class AetherTests(unittest.TestCase):
    def test_music_analyzer_detects_flux_and_stays_bounded(self):
        analyzer = MusicAnalyzer()
        bands = np.zeros(96, dtype=np.float32)
        quiet = analyzer.update(0.016, bands, 0.0, 0.0, 0.0)
        self.assertEqual(quiet.onset, 0.0)

        bands[:18] = 0.8
        hit = analyzer.update(0.016, bands, 0.4, 0.8, 0.1)
        self.assertGreater(hit.onset, 0.1)
        self.assertGreater(hit.low_transient, 0.1)
        for value in (
            hit.onset, hit.beat_phase, hit.beat_confidence, hit.density,
            hit.low_transient, hit.mid_transient, hit.high_transient,
        ):
            self.assertTrue(np.isfinite(value))
            self.assertGreaterEqual(value, 0.0)
            self.assertLessEqual(value, 1.0)

    def test_director_changes_scene_after_first_fibonacci_phrase(self):
        class Metrics:
            beat_confidence = 1.0
            beat_phase = 0.9
            density = 0.5
            onset = 0.5
            high_transient = 0.3

        director = VisualDirector()
        metrics = Metrics()
        for _ in range(13):
            metrics.beat_phase = 0.9
            director.update(metrics, 0.01)
            metrics.beat_phase = 0.1
            state = director.update(metrics, 0.01)

        self.assertEqual(state.scene_index, 1)
        self.assertTrue(np.isfinite(state.world_turn))
        self.assertGreater(state.world_turn, 0.0)


if __name__ == "__main__":
    unittest.main()
