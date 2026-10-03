import unittest

import numpy as np

from omamusi.visual.horizon import HorizonHotspots, wave_payload
from omamusi.visual.analysis import MusicMetrics


class HorizonWaveTests(unittest.TestCase):
    def test_quiet_frames_have_no_wave_emitters(self):
        payload = wave_payload([])
        self.assertEqual(payload.shape, (5, 2))
        self.assertEqual(payload.dtype, np.float32)
        self.assertTrue(np.all(payload[:, 0] < 0))
        self.assertTrue(np.all(payload[:, 1] == 0))

    def test_wave_ages_follow_transients_without_restarting_on_upload(self):
        bursts = [[0.35, 0.8], [0.10, 0.6]]
        first = wave_payload(bursts)
        aged = wave_payload([[age + 0.016, strength] for age, strength in bursts])
        np.testing.assert_allclose(aged[:2, 0] - first[:2, 0], 0.016, atol=1e-7)
        np.testing.assert_array_equal(aged[:2, 1], first[:2, 1])
        self.assertTrue(np.all(aged[2:, 0] < 0))
        self.assertEqual(bursts, [[0.35, 0.8], [0.10, 0.6]])

    def test_upload_keeps_only_the_latest_five_hits(self):
        bursts = [[i * 0.1, i * 0.1] for i in range(7)]
        np.testing.assert_allclose(wave_payload(bursts), bursts[-5:])


class HorizonHotspotTests(unittest.TestCase):
    def test_notes_spawn_but_sustained_onset_does_not_retrigger(self):
        spots = HorizonHotspots()
        spots.update(0.016, MusicMetrics(onset=0.6, mid_transient=0.4), True)
        self.assertEqual(len(spots.spots), 1)
        for _ in range(40):
            spots.update(0.016, MusicMetrics(onset=0.6), True)
        self.assertEqual(len(spots.spots), 1)
        self.assertGreater(spots.spots[0][0], 0.6)

    def test_inactive_audio_cannot_emit_and_events_expire(self):
        spots = HorizonHotspots()
        note = MusicMetrics(mid_transient=0.5)
        spots.update(0.016, note, False)
        self.assertTrue(np.all(spots.payload()[:, 0] < 0))
        spots.update(0.016, note, True)
        spots.update(4.3, MusicMetrics(), False)
        self.assertEqual(spots.spots, [])
        self.assertTrue(np.all(spots.payload()[:, 0] < 0))

    def test_slot_changes_preserve_orbit_identities_and_reset_clears(self):
        spots = HorizonHotspots()
        for _ in range(5):
            spots.update(0.25, MusicMetrics(mid_transient=0.4), True)
        self.assertEqual(len(spots.spots), 4)
        identity = spots.payload()[1:, 2].copy()
        spots.update(0.25, MusicMetrics(mid_transient=0.4), True)
        np.testing.assert_array_equal(spots.payload()[:3, 2], identity)
        self.assertEqual(len(set(spots.payload()[:, 2])), 4)
        spots.reset()
        self.assertTrue(np.all(spots.payload()[:, 0] < 0))
        self.assertEqual(spots.serial, 0)
