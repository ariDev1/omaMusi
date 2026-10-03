"""A text-only, keyboard-first overlay on the live audio visualization."""

import argparse
import os
from pathlib import Path
import shutil
import sys

from PySide6.QtCore import QEvent, QObject, Qt, QTimer
from PySide6.QtWidgets import (
    QApplication, QFileDialog, QFrame, QHBoxLayout, QLabel, QLineEdit,
    QListWidget, QListWidgetItem, QVBoxLayout, QWidget,
)

from .audio import AudioPlayer, probe
from .library import AUDIO_EXTENSIONS, discover, natural_key
from .theme import load_theme, stylesheet
from .visualizer import Visualizer


def clock(seconds):
    seconds = max(0, int(seconds))
    hours, remainder = divmod(seconds, 3600)
    minutes, seconds = divmod(remainder, 60)
    return f"{hours}:{minutes:02}:{seconds:02}" if hours else f"{minutes}:{seconds:02}"


class SearchFilter(QObject):
    def eventFilter(self, watched, event):
        if event.type() == QEvent.Type.ShortcutOverride:
            if event.key() != Qt.Key.Key_Escape and not event.modifiers() & (
                    Qt.KeyboardModifier.ControlModifier | Qt.KeyboardModifier.AltModifier):
                event.accept()
                return True
        return False


class PlayerWindow(QWidget):
    def __init__(self, tracks, mode=0):
        super().__init__()
        self.setWindowTitle("omaMusi")
        self.setWindowFlag(Qt.WindowType.FramelessWindowHint)
        self.resize(1060, 680)
        self.setMinimumSize(640, 420)
        self.setAcceptDrops(True)
        self.setFocusPolicy(Qt.FocusPolicy.StrongFocus)
        self.tracks = list(tracks)
        self.current_folder = self.tracks[0].parent if self.tracks else Path.cwd()
        self.browser_folder = None
        self.current = -1
        self.duration = 0
        self.history = []
        self.metadata = {}
        self.last_theme = None
        self.visualizer = Visualizer(self)
        self.visualizer.mode = mode
        self.player = AudioPlayer(self)
        self.player.samples.connect(self.visualizer.feed)
        self.player.positionChanged.connect(self.update_position)
        self.player.finished.connect(self.on_finished)
        self.player.failed.connect(self.on_error)
        self.build_ui()
        self._setup_navigation_idle()
        self.volume_notice_timer = QTimer(self)
        self.volume_notice_timer.setSingleShot(True)
        self.volume_notice_timer.timeout.connect(self.settings_label.hide)
        self.visual_notice_timer = QTimer(self)
        self.visual_notice_timer.setSingleShot(True)
        self.visual_notice_timer.timeout.connect(self.visual_notice.hide)
        self.populate()
        self.refresh_theme()
        self.state_timer = QTimer(self)
        self.state_timer.setInterval(100)
        self.state_timer.timeout.connect(self.update_state)
        self.state_timer.start()
        self.theme_timer = QTimer(self)
        self.theme_timer.setInterval(2000)
        self.theme_timer.timeout.connect(self.refresh_theme)
        self.theme_timer.start()
        if tracks:
            QTimer.singleShot(0, lambda: self.play_track(0))
        self.navigation_idle_timer.start()

    def label(self, text="", role="", wrap=False):
        label = QLabel(text)
        label.setObjectName(role)
        label.setTextFormat(Qt.TextFormat.PlainText)
        label.setWordWrap(wrap)
        return label

    def build_ui(self):
        root = QVBoxLayout(self)
        root.setContentsMargins(30, 26, 30, 24)
        root.setSpacing(8)
        header = QHBoxLayout()
        self.logo = self.label("omaMusi", "accent")
        header.addWidget(self.logo)
        header.addStretch()
        self.visual_notice = self.label("", "hint")
        self.visual_notice.hide()
        header.addWidget(self.visual_notice)
        root.addLayout(header)
        self.folder_label = self.label(str(self.current_folder), "muted")
        self.folder_label.setWordWrap(True)
        self.folder_label.hide()
        root.addWidget(self.folder_label)
        self.folder_prompt = QWidget()
        folder_layout = QHBoxLayout(self.folder_prompt)
        folder_layout.setContentsMargins(0, 0, 0, 0)
        folder_layout.addWidget(self.label("cd", "accent"))
        self.folder_input = QLineEdit()
        self.folder_input.setPlaceholderText("folder path · enter to change · esc to cancel")
        self.folder_input.returnPressed.connect(self.change_folder)
        self.folder_input_filter = SearchFilter(self.folder_input)
        self.folder_input.installEventFilter(self.folder_input_filter)
        folder_layout.addWidget(self.folder_input, 1)
        self.folder_prompt.hide()
        root.addWidget(self.folder_prompt)
        root.addSpacing(10)

        self.panel = QWidget()
        self.panel.setMaximumWidth(520)
        panel_layout = QVBoxLayout(self.panel)
        panel_layout.setContentsMargins(0, 0, 0, 0)
        panel_layout.setSpacing(6)
        self.count = self.label("", "muted")
        panel_layout.addWidget(self.count)
        self.search = QLineEdit()
        self.search.setPlaceholderText("/ type to filter")
        self.search.setClearButtonEnabled(False)
        self.search.setAttribute(Qt.WidgetAttribute.WA_MacShowFocusRect, False)
        self.search.textChanged.connect(self.filter_tracks)
        self.search.hide()
        self.search_filter = SearchFilter(self.search)
        self.search.installEventFilter(self.search_filter)
        panel_layout.addWidget(self.search)
        self.playlist = QListWidget()
        self.playlist.setFrameShape(QFrame.Shape.NoFrame)
        self.playlist.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.playlist.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.playlist.setFocusPolicy(Qt.FocusPolicy.NoFocus)
        self.playlist.itemClicked.connect(lambda item: self.play_track(self.playlist.row(item)))
        self.playlist.itemActivated.connect(lambda item: self.play_track(self.playlist.row(item)))
        panel_layout.addWidget(self.playlist)
        self.folder_list = QListWidget()
        self.folder_list.setFrameShape(QFrame.Shape.NoFrame)
        self.folder_list.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.folder_list.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.folder_list.setFocusPolicy(Qt.FocusPolicy.NoFocus)
        self.folder_list.itemClicked.connect(lambda item: self.activate_folder_item())
        self.folder_list.hide()
        panel_layout.addWidget(self.folder_list)
        root.addWidget(self.panel, 1, Qt.AlignmentFlag.AlignLeft)
        root.addStretch(0)

        self.title = self.label("Drop some music here.", "title", True)
        self.subtitle = self.label("", "muted", True)
        self.status = self.label("ready", "muted", True)
        root.addWidget(self.title)
        root.addWidget(self.subtitle)
        root.addWidget(self.status)
        self.status.hide()
        transport = QHBoxLayout()
        self.transport = self.label("stopped", "accent")
        transport.addWidget(self.transport)
        transport.addStretch()
        self.settings_label = self.label("", "muted")
        self.settings_label.hide()
        transport.addWidget(self.settings_label)
        root.addLayout(transport)
        self.hint = self.label("space pause · ↑↓ select · enter play · c folders · v visuals · +/− volume", "hint", True)
        root.addWidget(self.hint)

    def _setup_navigation_idle(self):
        """Fade navigation chrome while leaving playing-song information visible."""
        from PySide6.QtCore import QEasingCurve, QPropertyAnimation
        from PySide6.QtWidgets import QGraphicsOpacityEffect

        self.navigation_idle_timer = QTimer(self)
        self.navigation_idle_timer.setSingleShot(True)
        self.navigation_idle_timer.setInterval(4000)
        self.navigation_idle_timer.timeout.connect(self._fade_navigation)

        self._navigation_effects = []
        self._navigation_animations = []

        for widget in (
            self.logo,
            self.folder_label,
            self.folder_prompt,
            self.panel,
            self.status,
            self.hint,
        ):
            effect = QGraphicsOpacityEffect(widget)
            effect.setOpacity(1.0)
            widget.setGraphicsEffect(effect)
            animation = QPropertyAnimation(effect, b"opacity", self)
            animation.setEasingCurve(QEasingCurve.Type.InOutCubic)
            self._navigation_effects.append((widget, effect))
            self._navigation_animations.append(animation)

        application = QApplication.instance()
        if application is not None:
            application.installEventFilter(self)

    def _animate_navigation(self, opacity, duration):
        for animation, (_widget, effect) in zip(
                self._navigation_animations, self._navigation_effects):
            animation.stop()
            animation.setDuration(duration)
            animation.setStartValue(effect.opacity())
            animation.setEndValue(opacity)
            animation.start()

    def _fade_navigation(self):
        if self.current < 0:
            return
        self.panel.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents, True)
        self._animate_navigation(0.0, 900)

    def _show_navigation(self, restart_timer=True):
        if not hasattr(self, "_navigation_animations"):
            return
        self.panel.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents, False)
        self._animate_navigation(1.0, 140)
        if restart_timer:
            self.navigation_idle_timer.start()

    def eventFilter(self, watched, event):
        if hasattr(self, "navigation_idle_timer"):
            interaction_events = (
                QEvent.Type.KeyPress,
                QEvent.Type.MouseButtonPress,
                QEvent.Type.MouseButtonDblClick,
                QEvent.Type.MouseMove,
                QEvent.Type.Wheel,
                QEvent.Type.Enter,
                QEvent.Type.HoverMove,
                QEvent.Type.TouchBegin,
                QEvent.Type.WindowActivate,
            )
            if event.type() in interaction_events:
                self._show_navigation()
        return super().eventFilter(watched, event)

    def refresh_theme(self):
        colors, family = load_theme()
        signature = (colors, family)
        if signature != self.last_theme:
            self.last_theme = signature
            self.setStyleSheet(stylesheet(colors, family))
            self.visualizer.colors = colors
            self.visualizer.update()
            self.fit_list()

    def populate(self):
        self.playlist.clear()
        for i, path in enumerate(self.tracks):
            item = QListWidgetItem()
            item.setToolTip(str(path))
            self.playlist.addItem(item)
        self.update_markers()
        self.count.setText(f"playlist · {len(self.tracks)} files")
        if self.current >= 0:
            self.playlist.setCurrentRow(self.current)
        self.filter_tracks(self.search.text())
        self.fit_list()

    def update_markers(self):
        for i, path in enumerate(self.tracks):
            marker = "› " if i == self.current else "  "
            self.playlist.item(i).setText(f"{marker}{path.stem}")

    def filter_tracks(self, text):
        for i, path in enumerate(self.tracks):
            self.playlist.item(i).setHidden(text.casefold() not in path.name.casefold())
        self.fit_list()

    def fit_list(self):
        """Use available height for long lists; keep short lists content-sized."""
        browsing = self.browser_folder is not None
        listing = self.folder_list if browsing else self.playlist
        visible = sum(not listing.item(i).isHidden() for i in range(listing.count()))
        row_height = max(26, listing.sizeHintForRow(0))
        content_height = max(1, visible) * row_height + 2
        listing.setMinimumHeight(row_height + 2)
        listing.setMaximumHeight(content_height)
        self.count.setVisible(browsing)
        extras = [widget for widget in (self.count, self.search) if not widget.isHidden()]
        self.panel.setMaximumHeight(content_height + sum(widget.sizeHint().height() for widget in extras)
                                    + len(extras) * self.panel.layout().spacing())
        width = max((listing.fontMetrics().horizontalAdvance(listing.item(i).text())
                     for i in range(listing.count())), default=240)
        self.panel.setFixedWidth(min(self.width() - 60, 520, max(260, width + 20)))
        self.panel.setVisible(browsing or len(self.tracks) > 1 or self.search.isVisible())

    def play_track(self, index, remember=True):
        if not 0 <= index < len(self.tracks):
            return
        if remember and self.current >= 0 and index != self.current:
            self.history.append(self.current)
        self.current = index
        path = self.tracks[index]
        if path not in self.metadata:
            self.metadata[path] = probe(path)
        data = self.metadata[path]
        self.duration = data["duration"]
        self.title.setText(data["title"])
        self.subtitle.setText(" · ".join(x for x in (data["artist"], data["album"]) if x))
        self.subtitle.setVisible(bool(self.subtitle.text()))
        self.current_folder = path.parent
        if self.browser_folder is None:
            self.folder_label.setText(str(self.current_folder))
        self.status.setText("")
        self.status.hide()
        self.status.setToolTip("")
        self.setWindowTitle(f"{data['title']} — omaMusi")
        self.playlist.setCurrentRow(index)
        self.update_markers()
        self.visualizer.reset()
        self.player.play(path, sample_rate=data["sample_rate"])
        self.visualizer.sample_rate = self.player.sample_rate
        self.update_state()
        self._show_navigation()

    def update_state(self):
        playing = self.player.sink is not None and not self.player.paused
        self.visualizer.active = playing
        state = "" if playing else "paused · " if self.player.paused else ""
        self.transport.setText(f"{state}{clock(self.player.position)} / {clock(self.duration)}")
        self.transport.setVisible(self.current >= 0)
        self.settings_label.setText(f"vol {self.player.volume:.0%}")

    def update_position(self, seconds):
        self.update_state()

    def cycle_view(self, step=1):
        self.visual_notice.setText(self.visualizer.cycle(step))
        self.visual_notice.show()
        self.visual_notice_timer.start(2000)

    def toggle_play(self):
        if self.player.sink:
            self.player.toggle_pause()
        elif self.tracks:
            self.play_track(max(0, self.current))
        self.update_state()

    def next_index(self, automatic=False):
        if not self.tracks:
            return None
        if self.current + 1 < len(self.tracks):
            return self.current + 1
        if not automatic:
            return 0
        return None

    def next_track(self):
        index = self.next_index()
        if index is not None:
            self.play_track(index)

    def previous_track(self):
        if self.player.sink and self.player.position > 3:
            self.player.seek(0)
            self.visualizer.reset()
        elif self.history:
            self.play_track(self.history.pop(), remember=False)
        elif self.tracks:
            self.play_track((self.current - 1) % len(self.tracks), remember=False)

    def on_finished(self):
        index = self.next_index(automatic=True)
        if index is not None:
            self.play_track(index)
        else:
            self.status.setText("playlist finished · space to replay")
            self.status.show()
            self.update_state()

    def on_error(self, message):
        self.status.setText(f"error: {message[:220]}")
        self.status.show()
        self.status.setToolTip(message)
        self.update_state()

    def add_paths(self, paths):
        try:
            new = discover(paths)
        except ValueError as error:
            self.on_error(str(error))
            return
        seen = set(self.tracks)
        added = [path for path in new if path not in seen]
        start = len(self.tracks)
        self.tracks.extend(added)
        self.populate()
        if added and not self.player.sink:
            self.play_track(start)

    def open_files(self):
        extensions = " ".join(f"*{ext}" for ext in sorted(AUDIO_EXTENSIONS))
        paths, _ = QFileDialog.getOpenFileNames(self, "Add music", str(self.current_folder),
                                               f"Audio files ({extensions});;All files (*)")
        if paths:
            self.add_paths(paths)

    def begin_folder_change(self):
        self.folder_label.show()
        self.folder_prompt.show()
        self.folder_input.setText(str(self.browser_folder or self.current_folder))
        self.folder_input.setFocus()
        self.folder_input.selectAll()

    def change_folder(self):
        value = self.folder_input.text().strip()
        if not value:
            self.status.setText("folder: enter a path · esc to cancel")
            self.status.show()
            return
        try:
            folder = Path(value).expanduser()
            if not folder.is_absolute():
                folder = (self.browser_folder or self.current_folder) / folder
            folder = folder.resolve()
            if not folder.is_dir():
                raise ValueError(f"Not a folder: {folder}")
            tracks = discover([str(folder)])
        except (OSError, ValueError, RuntimeError) as error:
            self.status.setText(f"folder: {error}")
            self.status.show()
            return

        self.load_folder(folder, tracks)

    def load_folder(self, folder, tracks=None, start_path=None):
        if tracks is None:
            try:
                tracks = discover([str(folder)])
            except (OSError, ValueError) as error:
                self.status.setText(f"folder: {error}")
                self.status.show()
                return

        self.player.stop(clear=True)
        self.current_folder = folder
        self.tracks = tracks
        self.current = -1
        self.duration = 0
        self.history.clear()
        self.metadata.clear()
        self.search.clear()
        self.search.hide()
        self.folder_prompt.hide()
        self.end_folder_browse()
        self.setFocus()
        self.panel.show()
        self.populate()
        self.folder_label.setText(str(folder))
        self.visualizer.reset()
        if tracks:
            self.play_track(tracks.index(start_path) if start_path in tracks else 0)
        else:
            self.setWindowTitle("omaMusi")
            self.title.setText("No audio files in this folder.")
            self.subtitle.setText("")
            self.subtitle.hide()
            self.status.setText("press c to browse folders · o to open files")
            self.status.show()
            self.status.setToolTip("")
            self.update_state()

    def begin_folder_browse(self):
        self._show_navigation()
        self.folder_prompt.hide()
        self.search.clearFocus()
        self.panel.show()
        self.show_folder(self.browser_folder or self.current_folder)
        self.setFocus()

    def show_folder(self, folder):
        try:
            entries = list(folder.iterdir())
            folders = sorted((path for path in entries if path.is_dir()), key=natural_key)
            files = sorted((path for path in entries if path.is_file()
                            and path.suffix.lower() in AUDIO_EXTENSIONS), key=natural_key)
        except OSError as error:
            self.status.setText(f"folder: {error}")
            self.status.show()
            return
        self.browser_folder = folder
        self.folder_label.show()
        self.folder_label.setText(str(folder))
        self.search.hide()
        self.playlist.hide()
        self.folder_list.clear()
        play_here = QListWidgetItem("  ./   play this folder")
        play_here.setData(Qt.ItemDataRole.UserRole, "play")
        self.folder_list.addItem(play_here)
        if folder.parent != folder:
            parent = QListWidgetItem("  ../")
            parent.setData(Qt.ItemDataRole.UserRole, folder.parent)
            self.folder_list.addItem(parent)
        for path in folders:
            item = QListWidgetItem(f"  {path.name}/")
            item.setData(Qt.ItemDataRole.UserRole, path)
            self.folder_list.addItem(item)
        for path in files:
            item = QListWidgetItem(f"  {path.name}")
            item.setData(Qt.ItemDataRole.UserRole, path)
            self.folder_list.addItem(item)
        self.folder_list.setCurrentRow(0)
        self.folder_list.show()
        self.count.setText(f"browse · {len(folders)} folders · {len(files)} audio files")
        self.hint.setText("↑↓ select · →/enter open/play · ← parent · v visuals · esc back")
        self.fit_list()

    def activate_folder_item(self):
        if self.browser_folder is None:
            self.play_track(self.playlist.currentRow())
            return
        item = self.folder_list.currentItem()
        if item is None:
            return
        target = item.data(Qt.ItemDataRole.UserRole)
        if target == "play":
            self.load_folder(self.browser_folder)
        elif target.is_dir():
            self.show_folder(target)
        else:
            self.load_folder(self.browser_folder, start_path=target)

    def folder_parent(self):
        if self.browser_folder is not None:
            self.show_folder(self.browser_folder.parent)

    def play_browsed_folder(self):
        if self.browser_folder is not None:
            self.load_folder(self.browser_folder)

    def end_folder_browse(self):
        self.browser_folder = None
        self.folder_list.hide()
        self.playlist.show()
        self.search.setVisible(bool(self.search.text()))
        self.folder_label.setText(str(self.current_folder))
        self.folder_label.hide()
        self.count.setText(f"playlist · {len(self.tracks)} files")
        self.hint.setText("space pause · ↑↓ select · enter play · c folders · v visuals · +/− volume")
        self.fit_list()
        self.setFocus()

    def dragEnterEvent(self, event):
        if event.mimeData().hasUrls() and any(url.isLocalFile() for url in event.mimeData().urls()):
            event.acceptProposedAction()

    def dropEvent(self, event):
        self.add_paths([url.toLocalFile() for url in event.mimeData().urls() if url.isLocalFile()])
        event.acceptProposedAction()

    def select_relative(self, delta):
        if self.browser_folder is not None:
            count = self.folder_list.count()
            if count:
                self.folder_list.setCurrentRow((self.folder_list.currentRow() + delta) % count)
            return
        visible = [i for i in range(len(self.tracks)) if not self.playlist.item(i).isHidden()]
        if not visible:
            return
        row = self.playlist.currentRow()
        index = visible.index(row) if row in visible else (-1 if delta > 0 else 0)
        self.playlist.setCurrentRow(visible[(index + delta) % len(visible)])

    def begin_search(self):
        self._show_navigation()
        self.end_folder_browse()
        self.panel.show()
        self.search.show()
        self.search.setFocus()
        self.fit_list()

    def escape(self):
        if self.folder_prompt.isVisible():
            self.folder_prompt.hide()
            self.folder_label.setVisible(self.browser_folder is not None)
            self.setFocus()
        elif self.browser_folder is not None:
            self.end_folder_browse()
        elif self.search.hasFocus():
            self.search.clearFocus()
            if not self.search.text():
                self.search.hide()
            self.setFocus()
        elif self.isFullScreen():
            self.showNormal()

    def focusNextPrevChild(self, _next):
        # Keep Tab as a player command instead of focus navigation.
        return False

    def keyPressEvent(self, event):
        key = event.key()
        modifiers = event.modifiers()
        ctrl = bool(modifiers & Qt.KeyboardModifier.ControlModifier)
        reserved = bool(modifiers & (Qt.KeyboardModifier.ControlModifier
                                     | Qt.KeyboardModifier.AltModifier
                                     | Qt.KeyboardModifier.MetaModifier))
        shift = bool(modifiers & Qt.KeyboardModifier.ShiftModifier)
        focused_edit = isinstance(QApplication.focusWidget(), QLineEdit)

        def volume(delta):
            self.player.set_volume(round(self.player.volume + delta, 2))
            self.update_state()
            self.settings_label.show()
            self.volume_notice_timer.start(2000)

        # Text entry keeps its keys; only Escape bubbles up to the player.
        if focused_edit:
            if key == Qt.Key.Key_Escape:
                self.escape()
                event.accept()
                return
            super().keyPressEvent(event)
            return

        if ctrl and key == Qt.Key.Key_O:
            self.open_files()
            event.accept()
            return
        if ctrl and key == Qt.Key.Key_L:
            self.begin_folder_change()
            event.accept()
            return
        if reserved:
            super().keyPressEvent(event)
            return

        if key in (Qt.Key.Key_Plus, Qt.Key.Key_Equal) or event.text() in ("+", "="):
            volume(0.05)
            event.accept()
        elif key == Qt.Key.Key_Minus or event.text() == "-":
            volume(-0.05)
            event.accept()
        elif key == Qt.Key.Key_Space:
            self.toggle_play()
            event.accept()
        elif key == Qt.Key.Key_N:
            self.next_track()
            event.accept()
        elif key == Qt.Key.Key_P:
            self.previous_track()
            event.accept()
        elif key == Qt.Key.Key_V:
            self.cycle_view(-1 if shift else 1)
            event.accept()
        elif key == Qt.Key.Key_Q:
            self.close()
            event.accept()
        elif key == Qt.Key.Key_O:
            self.open_files()
            event.accept()
        elif key == Qt.Key.Key_C:
            self.begin_folder_browse()
            event.accept()
        elif key == Qt.Key.Key_L:
            self.play_browsed_folder()
            event.accept()
        elif key == Qt.Key.Key_Backspace:
            self.folder_parent()
            event.accept()
        elif key == Qt.Key.Key_F:
            self.showNormal() if self.isFullScreen() else self.showFullScreen()
            event.accept()
        elif key == Qt.Key.Key_Escape:
            self.escape()
            event.accept()
        elif key == Qt.Key.Key_Tab:
            self.panel.setVisible(not self.panel.isVisible())
            event.accept()
        elif key == Qt.Key.Key_Slash or event.text() == "/":
            self.begin_search()
            event.accept()
        elif key in (Qt.Key.Key_J, Qt.Key.Key_Down):
            self.select_relative(1)
            event.accept()
        elif key in (Qt.Key.Key_K, Qt.Key.Key_Up):
            self.select_relative(-1)
            event.accept()
        elif key in (Qt.Key.Key_Return, Qt.Key.Key_Enter):
            self.activate_folder_item()
            event.accept()
        elif key == Qt.Key.Key_Right:
            if self.browser_folder is not None:
                self.activate_folder_item()
            else:
                self.seek_relative(5)
            event.accept()
        elif key == Qt.Key.Key_Left:
            if self.browser_folder is not None:
                self.folder_parent()
            else:
                self.seek_relative(-5)
            event.accept()
        else:
            super().keyPressEvent(event)

    def seek_relative(self, delta):
        if self.player.sink:
            target = max(0, self.player.position + delta)
            if self.duration:
                target = min(target, max(0, self.duration - 0.1))
            self.player.seek(target)
            self.visualizer.reset()

    def resizeEvent(self, event):
        self.visualizer.setGeometry(self.rect())
        if hasattr(self, "playlist"):
            self.fit_list()
        super().resizeEvent(event)

    def closeEvent(self, event):
        application = QApplication.instance()
        if application is not None:
            application.removeEventFilter(self)
        self.player.shutdown()
        event.accept()


def parser():
    result = argparse.ArgumentParser(
        prog="omaMusi", description="Play local audio with a live visualization and text-only playlist.",
        epilog="Example: cd ~/Music && omaMusi -all",
    )
    result.add_argument("paths", nargs="*", help="Audio files or directories (default: current folder)")
    result.add_argument("-all", "--all", action="store_true", help="Play all audio files in the current folder")
    result.add_argument("-r", "--recursive", action="store_true", help="Include subfolders when scanning directories")
    view_choices = [mode.lower() for mode in Visualizer.modes] + [
        "sprites", "phi", "cathedral", "phi-cathedral",
    ]
    result.add_argument("--view", choices=view_choices, default="warp",
                        help="Initial visualization (change with V while playing)")
    result.add_argument("--version", action="version", version="omaMusi 0.3.6")
    return result


def main():
    cli = parser()
    args = cli.parse_args()
    for executable in ("ffmpeg", "ffprobe"):
        if not shutil.which(executable):
            cli.error(f"{executable} is required. Install FFmpeg first.")
    try:
        paths = ([str(Path.cwd())] if args.all else []) + args.paths
        tracks = discover(paths or [str(Path.cwd())], recursive=args.recursive)
    except ValueError as error:
        cli.error(str(error))
    if not tracks:
        print("omaMusi: no audio files found. Press o to open files or drop music onto the window.", file=sys.stderr)
    app = QApplication(sys.argv[:1])
    app.setApplicationName("omaMusi")
    app.setStyle("Fusion")
    aliases = {
        "sprites": "warp",
        "phi": "phi cathedral",
        "cathedral": "phi cathedral",
        "phi-cathedral": "phi cathedral",
    }
    view = aliases.get(args.view, args.view)
    window = PlayerWindow(tracks, [mode.lower() for mode in Visualizer.modes].index(view))
    visualizer = window.visualizer
    print(f"omaMusi: view={view} renderer={visualizer.renderer} ({visualizer.renderer_detail})",
          file=sys.stderr)
    if os.environ.get("OMA_PARTICLE_CANARY") == "1":
        print("omaMusi: PARTICLE CANARY on — swarm forced to giant red dots in Particle Dance",
              file=sys.stderr)
    window.show()
    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())
