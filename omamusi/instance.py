"""One player per user session; subsequent launches activate its window."""
import os
from pathlib import Path
import tempfile
import time

from PySide6.QtCore import QLockFile, QObject, QProcess
from PySide6.QtNetwork import QLocalServer, QLocalSocket


class SingleInstance(QObject):
    def __init__(self, parent=None):
        super().__init__(parent)
        runtime = Path(os.environ.get("XDG_RUNTIME_DIR") or tempfile.gettempdir())
        self.name = str(runtime / f"omamusi-{os.getuid()}")
        self.lock = QLockFile(self.name + ".lock")
        # A long-running player is not a stale lock. Dead PIDs are still detected.
        self.lock.setStaleLockTime(0)
        self.server = QLocalServer(self)
        self.server.setSocketOptions(QLocalServer.SocketOption.UserAccessOption)
        self.server.newConnection.connect(self.activate)
        self.window = None

    def start_or_activate(self):
        if self.lock.tryLock(0):
            QLocalServer.removeServer(self.name)
            if not self.server.listen(self.name):
                self.lock.unlock()
                raise RuntimeError(self.server.errorString())
            return True
        # The first launch may still be starting its server.
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            socket = QLocalSocket()
            socket.connectToServer(self.name)
            if socket.waitForConnected(100):
                socket.disconnectFromServer()
                return False
            time.sleep(0.05)
        raise RuntimeError("The running player could not be contacted. Please try again.")

    def activate(self):
        while self.server.hasPendingConnections():
            socket = self.server.nextPendingConnection()
            socket.disconnectFromServer()
            socket.deleteLater()
        if self.window is None:
            return
        if self.window.isMinimized():
            self.window.showNormal()
        else:
            self.window.show()
        self.window.raise_()
        self.window.activateWindow()
        # Wayland compositors can reject activation without an input token.
        # Hyprland's dispatcher also switches to the player's workspace.
        if os.environ.get("HYPRLAND_INSTANCE_SIGNATURE"):
            QProcess.startDetached("hyprctl", ["dispatch", "focuswindow", f"pid:{os.getpid()}"])

    def close(self):
        if self.lock.isLocked():
            self.server.close()
            self.lock.unlock()
