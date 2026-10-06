"""One album cover on the right, with a restrained audio-reactive pulse."""
from PySide6.QtCore import QPointF, QRectF, Qt
from PySide6.QtGui import QColor, QPainter, QPen, QRadialGradient


def artwork_color(image):
    """Pick a warm/cool light from the artwork once, rather than per frame."""
    if image.isNull():
        return QColor("white")
    sample = image.scaled(16, 16, Qt.AspectRatioMode.KeepAspectRatio,
                          Qt.TransformationMode.SmoothTransformation)
    red = green = blue = total = 0.0
    for y in range(sample.height()):
        for x in range(sample.width()):
            color = sample.pixelColor(x, y)
            weight = color.saturationF() * color.valueF() * color.alphaF()
            red += color.redF() * weight
            green += color.greenF() * weight
            blue += color.blueF() * weight
            total += weight
    return QColor.fromRgbF(red / total, green / total, blue / total) if total else QColor("white")


def cover_rectangle(width, height, image_size, bass=0.0):
    if width <= 0 or height <= 0 or image_size.isEmpty():
        return QRectF()
    scale = min(width * .40 / image_size.width(), height * .65 / image_size.height())
    scale *= 1.0 + .055 * max(0.0, min(1.0, bass))
    rectangle = QRectF(0, 0, image_size.width() * scale, image_size.height() * scale)
    rectangle.moveCenter(QPointF(width * .74, height * .46))
    return rectangle


def paint_cover(painter, image, width, height, bass=0.0, energy=0.0, accent=None):
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
        # Light grows with loudness; the image itself keeps its original colors.
        light = QColor(accent if accent is not None else QColor("white"))
        light.setAlpha(int(36 + 88 * max(0.0, min(1.0, energy))))
        transparent = QColor(light)
        transparent.setAlpha(0)
        radius = max(rectangle.width(), rectangle.height()) * .95
        glow = QRadialGradient(rectangle.center(), radius)
        glow.setColorAt(0, light)
        glow.setColorAt(.45, light)
        glow.setColorAt(1, transparent)
        painter.fillRect(QRectF(0, 0, width, height), glow)
        painter.drawImage(rectangle, image, QRectF(image.rect()))
        painter.setPen(QPen(QColor(255, 255, 255, 28), 1))
        painter.setBrush(Qt.BrushStyle.NoBrush)
        painter.drawRect(rectangle)
    finally:
        painter.restore()
