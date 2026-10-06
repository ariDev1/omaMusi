"""One album cover on the right, with a restrained audio-reactive pulse."""
from PySide6.QtCore import QPointF, QRectF, Qt
from PySide6.QtGui import QColor, QPainter, QPen


def cover_rectangle(width, height, image_size, bass=0.0):
    if width <= 0 or height <= 0 or image_size.isEmpty():
        return QRectF()
    scale = min(width * .40 / image_size.width(), height * .65 / image_size.height())
    scale *= 1.0 + .055 * max(0.0, min(1.0, bass))
    rectangle = QRectF(0, 0, image_size.width() * scale, image_size.height() * scale)
    rectangle.moveCenter(QPointF(width * .74, height * .46))
    return rectangle


def paint_cover(painter, image, width, height, bass=0.0):
    painter.fillRect(QRectF(0, 0, width, height), Qt.GlobalColor.black)
    if image.isNull():
        return
    rectangle = cover_rectangle(width, height, image.size(), bass)
    if rectangle.isEmpty():
        return
    painter.save()
    try:
        painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        painter.drawImage(rectangle, image, QRectF(image.rect()))
        painter.setPen(QPen(QColor(255, 255, 255, 28), 1))
        painter.setBrush(Qt.BrushStyle.NoBrush)
        painter.drawRect(rectangle)
    finally:
        painter.restore()
