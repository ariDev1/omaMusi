"""Read embedded or album-folder artwork without touching music files."""
import json
from pathlib import Path
import subprocess

from PySide6.QtCore import Qt
from PySide6.QtGui import QImage, QImageReader


def _read_image(path):
    if path.stat().st_size > 16 * 1024 * 1024:
        return QImage()
    reader = QImageReader(str(path))
    reader.setAutoTransform(True)
    size = reader.size()
    if not size.isValid() or size.width() * size.height() > 64_000_000:
        return QImage()
    if max(size.width(), size.height()) > 1200:
        reader.setScaledSize(size.scaled(1200, 1200, Qt.AspectRatioMode.KeepAspectRatio))
    return reader.read()


def load_artwork(path):
    """Prefer an attached picture, then conventional cover files; empty on failure.

    Called on a file-access worker. Decode size and subprocess duration are
    bounded; only the current track's image is retained by the window.
    """
    path = Path(path)
    try:
        result = subprocess.run(
            ["ffprobe", "-v", "error", "-select_streams", "v", "-show_entries",
             "stream=index:stream_disposition=attached_pic", "-of", "json", str(path)],
            capture_output=True, text=True, timeout=5, check=True)
        for stream in json.loads(result.stdout).get("streams", []):
            if stream.get("disposition", {}).get("attached_pic") != 1:
                continue
            index = int(stream["index"])
            result = subprocess.run(
                ["ffmpeg", "-nostdin", "-v", "error", "-max_alloc", "67108864", "-i", str(path),
                 "-map", f"0:{index}", "-frames:v", "1", "-vf",
                 "scale=w='min(1200,iw)':h='min(1200,ih)':force_original_aspect_ratio=decrease",
                 "-c:v", "png", "-threads", "1", "-f", "image2pipe", "pipe:1"],
                capture_output=True, timeout=5, check=True)
            if len(result.stdout) <= 8 * 1024 * 1024:
                image = QImage.fromData(result.stdout)
                if not image.isNull():
                    return image
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
        pass

    try:
        files = {entry.name.casefold(): entry for entry in path.parent.iterdir()}
        for name in ("cover", "folder", "front", "album"):
            for extension in ("jpg", "jpeg", "png", "webp"):
                candidate = files.get(f"{name}.{extension}")
                if candidate is None:
                    continue
                try:
                    image = _read_image(candidate)
                    if not image.isNull():
                        return image
                except OSError:
                    continue
    except OSError:
        pass
    return QImage()
