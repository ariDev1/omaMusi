"""An asymmetric, responsive gallery of the current album's artwork."""
from PySide6.QtCore import QRectF, Qt
from PySide6.QtGui import QPainter


def gallery_rectangles(width, height, image_size):
    if width <= 0 or height <= 0 or image_size.isEmpty():
        return []
    if width >= height:
        slots = ((.26, .10, .43, .76), (.035, .04, .18, .31),
                 (.065, .47, .16, .42), (.735, .10, .23, .38), (.76, .57, .19, .31))
    else:
        slots = ((.06, .25, .64, .50), (.06, .04, .36, .18),
                 (.50, .06, .44, .17), (.53, .78, .41, .18), (.77, .35, .19, .35))
    rectangles = []
    for x, y, w, h in slots:
        cell = QRectF(x * width, y * height, w * width, h * height)
        scale = min(cell.width() / image_size.width(), cell.height() / image_size.height())
        fitted = QRectF(0, 0, image_size.width() * scale, image_size.height() * scale)
        fitted.moveCenter(cell.center())
        rectangles.append(fitted)
    return rectangles


def paint_gallery(painter, image, width, height):
    painter.fillRect(QRectF(0, 0, width, height), Qt.GlobalColor.black)
    if image.isNull():
        return
    painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)
    source = QRectF(image.rect())
    for rectangle in gallery_rectangles(width, height, image.size()):
        painter.drawImage(rectangle, image, source)
