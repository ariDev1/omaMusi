import unittest

import numpy as np

from omamusi.visual.horizon import wave_payload


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
