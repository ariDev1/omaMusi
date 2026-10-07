"""A quiet, clickable mosaic of album covers on a black stage."""
from PySide6.QtCore import QRectF, Qt
from PySide6.QtGui import QColor, QImage, QPainter, QPen


def _draw_cover(painter, image, rectangle, opacity):
    if opacity <= 0:
        return
    size = image.size().scaled(rectangle.size().toSize(), Qt.AspectRatioMode.KeepAspectRatio)
    target = QRectF(0, 0, size.width(), size.height())
    target.moveCenter(rectangle.center())
    painter.setOpacity(opacity)
    painter.drawImage(target, image, QRectF(image.rect()))


def paint_gallery(painter, gallery, width, height, area, bass=0.0):
    painter.fillRect(QRectF(0, 0, width, height), Qt.GlobalColor.black)
    rectangles = gallery.layout(area)
    painter.save()
    try:
        painter.setRenderHint(painter.RenderHint.SmoothPixmapTransform)
        painter.setRenderHint(painter.RenderHint.Antialiasing)
        if not rectangles:
            painter.setPen(QColor(255, 255, 255, 110))
            message = ('Finding album covers…' if gallery.scanning else
                       'No album artwork found\nAdd a cover image to your music folders')
            painter.drawText(area, Qt.AlignmentFlag.AlignCenter | Qt.TextFlag.TextWordWrap, message)
            return
        for index, (tile, rectangle) in enumerate(zip(gallery.tiles, rectangles)):
            # The cover breathes inside a fixed tile, keeping its hit area still.
            inset = rectangle.width() * (.025 - .012 * min(1, max(0, bass)))
            image_rectangle = rectangle.adjusted(inset, inset, -inset, -inset)
            painter.setPen(Qt.PenStyle.NoPen)
            painter.setBrush(QColor('#111114'))
            painter.drawRoundedRect(rectangle, 5, 5)
            fade = tile.fade * tile.fade * (3 - 2 * tile.fade)
            cover = gallery.covers[tile.key]
            if tile.previous is not None:
                # Add weighted premultiplied images on a transparent layer.
                # SourceOver twice would dim the outgoing cover twice.
                layer = QImage(image_rectangle.size().toSize(), QImage.Format.Format_ARGB32_Premultiplied)
                layer.fill(Qt.GlobalColor.transparent)
                blend = QPainter(layer)
                try:
                    blend.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)
                    target = QRectF(layer.rect())
                    _draw_cover(blend, gallery.covers[tile.previous].image, target, 1 - fade)
                    incoming = QImage(layer.size(), layer.format())
                    incoming.fill(Qt.GlobalColor.transparent)
                    incoming_painter = QPainter(incoming)
                    try:
                        incoming_painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)
                        _draw_cover(incoming_painter, cover.image, target, fade)
                    finally:
                        incoming_painter.end()
                    blend.setOpacity(1)
                    blend.setCompositionMode(QPainter.CompositionMode.CompositionMode_Plus)
                    blend.drawImage(0, 0, incoming)
                finally:
                    blend.end()
                painter.drawImage(image_rectangle, layer)
            else:
                _draw_cover(painter, cover.image, image_rectangle, 1)
            painter.setOpacity(1)
            hovered = index == gallery.hovered
            painter.setPen(QPen(QColor(255, 255, 255, 150 if hovered else 28), 1))
            painter.setBrush(Qt.BrushStyle.NoBrush)
            painter.drawRoundedRect(rectangle, 5, 5)
            if hovered:
                caption = QRectF(rectangle.left(), rectangle.bottom() - min(52, rectangle.height()),
                                 rectangle.width(), min(52, rectangle.height()))
                painter.fillRect(caption, QColor(0, 0, 0, 215))
                painter.setPen(QColor('#eeeeee'))
                path = cover.tracks[0]
                label = path.stem if len(cover.tracks) == 1 else path.parent.name
                label = painter.fontMetrics().elidedText(label, Qt.TextElideMode.ElideRight,
                                                        max(1, int(caption.width()) - 14))
                detail = ('Click to play' if len(cover.tracks) == 1 else
                          f'{len(cover.tracks)} songs · click for random')
                detail = painter.fontMetrics().elidedText(detail, Qt.TextElideMode.ElideRight,
                                                         max(1, int(caption.width()) - 14))
                painter.drawText(caption.adjusted(7, 3, -7, -3),
                                 Qt.AlignmentFlag.AlignCenter, f'{label}\n{detail}')
    finally:
        painter.restore()
