"""Bounded cover thumbnails, shared-art song groups, and gallery transitions."""
from dataclasses import dataclass, field
import hashlib
import math
from pathlib import Path
import random

from PySide6.QtCore import QRectF, Qt
from PySide6.QtGui import QImage

from .artwork import load_artwork


def load_gallery_artwork(path):
    image = load_artwork(path)
    if image.isNull():
        return image
    return image.scaled(320, 320, Qt.AspectRatioMode.KeepAspectRatio,
                        Qt.TransformationMode.SmoothTransformation)


@dataclass
class GalleryCover:
    image: QImage
    tracks: list = field(default_factory=list)

    def pick(self, current=None):
        alternatives = [path for path in self.tracks if path != current]
        return random.choice(alternatives or self.tracks) if self.tracks else None


@dataclass
class GalleryTile:
    key: str
    previous: str | None = None
    fade: float = 1.0


class CoverGallery:
    MAX_COVERS = 128
    INTERVAL = 5.0
    FADE_SECONDS = 1.2

    def __init__(self):
        self.covers = {}
        self.tiles = []
        self.hovered = None
        self.scanning = False
        self._elapsed = 0.0
        self._cached = 0

    def clear(self):
        self.covers.clear()
        self.tiles.clear()
        self.hovered = None
        self.scanning = False
        self._elapsed = 0.0
        self._cached = 0

    def add(self, path, image):
        if image.isNull():
            return
        image = image.scaled(320, 320, Qt.AspectRatioMode.KeepAspectRatio,
                             Qt.TransformationMode.SmoothTransformation)
        rgba = image.convertToFormat(QImage.Format.Format_RGBA8888)
        digest = hashlib.sha256(f'{rgba.width()}x{rgba.height()}:'.encode())
        digest.update(rgba.constBits())
        key = digest.hexdigest()
        cover = self.covers.get(key)
        if cover is None:
            # Sources are shuffled before scanning. Retain a random sample of
            # thumbnails, but still collect every song sharing a sampled cover.
            thumbnail = QImage(image) if self._cached < self.MAX_COVERS else QImage()
            self._cached += not thumbnail.isNull()
            cover = self.covers[key] = GalleryCover(thumbnail)
        path = Path(path)
        if path not in cover.tracks:
            cover.tracks.append(path)

    def layout(self, area):
        keys = [key for key, cover in self.covers.items() if not cover.image.isNull()]
        if area.width() < 40 or area.height() < 40 or not keys:
            self.tiles.clear()
            self.hovered = None
            return []
        gap = 18.0
        columns = min(3, max(1, int((area.width() + gap) / 158)))
        rows = min(2, max(1, int((area.height() + gap) / 158)))
        # Leave one cover offstage so even a small collection can keep changing.
        count = min(max(1, len(keys) - 1), columns * rows)
        if self.hovered is not None and self.tiles:
            count = min(count, len(self.tiles))
        if len(self.tiles) > count:
            self.tiles = self.tiles[:count]
            self.hovered = None
        while len(self.tiles) < count:
            visible = {key for tile in self.tiles for key in (tile.key, tile.previous)
                       if key is not None}
            candidates = [key for key in keys if key not in visible]
            if not candidates:
                break
            self.tiles.append(GalleryTile(random.choice(candidates)))
        count = len(self.tiles)
        columns = min(columns, count)
        rows = math.ceil(count / columns)
        side = min((area.width() - (columns - 1) * gap) / columns,
                   (area.height() - (rows - 1) * gap) / rows, 240.0)
        left = area.center().x() - (columns * side + (columns - 1) * gap) / 2
        top = area.center().y() - (rows * side + (rows - 1) * gap) / 2
        return [QRectF(left + (index % columns) * (side + gap),
                       top + (index // columns) * (side + gap), side, side)
                for index in range(count)]

    def hover(self, point, area):
        rectangles = self.layout(area)
        self.hovered = next((index for index, rect in enumerate(rectangles)
                             if point is not None and rect.contains(point)), None)
        if self.hovered is not None:
            tile = self.tiles[self.hovered]
            # Freeze the dominant image rather than leaving a half-faded,
            # ambiguous click target under the pointer.
            if tile.previous is not None and tile.fade < .5:
                tile.key = tile.previous
            tile.previous = None
            tile.fade = 1.0

    def hit(self, point, area):
        self.hover(point, area)
        return self.covers[self.tiles[self.hovered].key] if self.hovered is not None else None

    def tick(self, elapsed):
        for index, tile in enumerate(self.tiles):
            if index != self.hovered:
                tile.fade = min(1.0, tile.fade + elapsed / self.FADE_SECONDS)
                if tile.fade >= 1.0:
                    tile.previous = None
        self._elapsed += elapsed
        if self._elapsed < self.INTERVAL:
            return
        self._elapsed %= self.INTERVAL
        reserved = {key for tile in self.tiles for key in (tile.key, tile.previous) if key is not None}
        choices = [key for key, cover in self.covers.items()
                   if not cover.image.isNull() and key not in reserved]
        slots = [index for index, tile in enumerate(self.tiles)
                 if index != self.hovered and tile.previous is None]
        if choices and slots:
            tile = self.tiles[random.choice(slots)]
            tile.previous, tile.key, tile.fade = tile.key, random.choice(choices), 0.0
