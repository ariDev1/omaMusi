"""GPU deletion requires the canvas context, including swarm resources."""
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from omamusi.gpu import GpuCanvas


class GpuCleanupTests(unittest.TestCase):
    def canvas(self, valid=True):
        context = SimpleNamespace(isValid=lambda: valid)
        self.current = None
        self.deleted = []
        def activate():
            self.current = context
        def release():
            self.current = None
        return SimpleNamespace(
            context=lambda: context, makeCurrent=activate, doneCurrent=release,
            swarm_vbos=[11, 12], swarm_vaos=[21, 22], swarm_update=31, swarm_render=32,
            textures=[41, 42, 43], vao=51, quad=61, particles=62, phi=63,
            phi_particles=64, event_horizon=65, ready=True)

    def delete(self, *args):
        self.assertIsNotNone(self.current, "GPU deletion without the canvas context")
        self.deleted.append(args)

    def cleanup(self, canvas):
        with patch("omamusi.gpu.GL.glDeleteBuffers", side_effect=self.delete), \
             patch("omamusi.gpu.GL.glDeleteVertexArrays", side_effect=self.delete), \
             patch("omamusi.gpu.GL.glDeleteTextures", side_effect=self.delete), \
             patch("omamusi.gpu.GL.glDeleteProgram", side_effect=self.delete), \
             patch("omamusi.gpu.QOpenGLContext.currentContext", side_effect=lambda: self.current):
            GpuCanvas.cleanup(canvas)

    def test_all_resources_deleted_with_current_context_only_once(self):
        canvas = self.canvas()
        self.cleanup(canvas)
        self.assertTrue(self.deleted)
        count = len(self.deleted)
        self.cleanup(canvas)
        self.assertEqual(len(self.deleted), count)
        self.assertFalse(canvas.ready)
        self.assertIsNone(self.current)

    def test_invalid_context_skips_gl_calls(self):
        canvas = self.canvas(valid=False)
        self.cleanup(canvas)
        self.assertEqual(self.deleted, [])
        self.assertFalse(canvas.ready)

    def test_failed_context_activation_skips_gl_calls(self):
        canvas = self.canvas()
        canvas.makeCurrent = lambda: None
        self.cleanup(canvas)
        self.assertEqual(self.deleted, [])
        self.assertFalse(canvas.ready)

    def test_deletion_failure_still_releases_context(self):
        canvas = self.canvas()
        with patch("omamusi.gpu.QOpenGLContext.currentContext", side_effect=lambda: self.current), \
             patch("omamusi.gpu.GL.glDeleteBuffers", side_effect=RuntimeError("driver failure")):
            with self.assertRaisesRegex(RuntimeError, "driver failure"):
                GpuCanvas.cleanup(canvas)
        self.assertIsNone(self.current)
        self.assertFalse(canvas.ready)
