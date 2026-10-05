"""Keyboard-first chooser and browser for saved playlists."""
from PySide6.QtCore import Qt, Signal
from PySide6.QtWidgets import QDialog, QFrame, QLineEdit, QListWidget, QListWidgetItem, QVBoxLayout


class PlaylistDialog(QDialog):
    notice = Signal(str)

    def __init__(self, parent, store, track=None):
        super().__init__(parent)
        self.store = store
        self.track = track
        self.playlist_name = None
        self.prompt = None
        self.prompt_target = None
        self.setWindowTitle("Add to playlist" if track is not None else "Saved playlists")
        self.setWindowModality(Qt.WindowModality.ApplicationModal)
        self.setAttribute(Qt.WidgetAttribute.WA_DeleteOnClose)
        self.resize(560, 400)
        self.setMinimumSize(360, 260)
        colors = parent.last_theme[0]
        self.setStyleSheet(parent.styleSheet() + f"QDialog {{ background: {colors['background']}; }}")
        layout = QVBoxLayout(self)
        self.heading = parent.label("", "accent", True)
        layout.addWidget(self.heading)
        self.listing = QListWidget()
        self.listing.setFrameShape(QFrame.Shape.NoFrame)
        self.listing.setFocusPolicy(Qt.FocusPolicy.NoFocus)
        self.listing.itemDoubleClicked.connect(lambda item: self.perform(self.activate_selection))
        layout.addWidget(self.listing, 1)
        self.name_input = QLineEdit()
        self.name_input.setMaxLength(120)
        self.name_input.returnPressed.connect(lambda: self.perform(self.confirm_prompt))
        self.name_input.hide()
        layout.addWidget(self.name_input)
        self.message = parent.label("", "muted", True)
        layout.addWidget(self.message)
        self.help = parent.label("", "hint", True)
        layout.addWidget(self.help)
        self.refresh()

    def refresh(self, selected=None):
        self.listing.clear()
        if self.playlist_name is not None:
            self.heading.setText(self.playlist_name)
            for path in self.store.tracks(self.playlist_name):
                missing = " [missing]" if not path.is_file() else ""
                item = QListWidgetItem(f"{path.stem}{missing}")
                item.setToolTip(str(path))
                item.setData(Qt.ItemDataRole.UserRole, path)
                self.listing.addItem(item)
            self.help.setText("↑↓ select · delete remove track · ← back · esc close")
        else:
            self.heading.setText(f"Add {self.track.name} to…" if self.track is not None else "Saved playlists")
            for name in self.store.names():
                item = QListWidgetItem(f"{name} · {len(self.store.tracks(name))} tracks")
                item.setData(Qt.ItemDataRole.UserRole, name)
                self.listing.addItem(item)
            self.listing.addItem(QListWidgetItem("Create new playlist…"))
            self.help.setText("↑↓ select · enter add · esc cancel" if self.track is not None else
                              "↑↓ select · enter play · → tracks · F2 rename · delete remove · esc close")
        row = 0
        if selected is not None:
            for index in range(self.listing.count()):
                if self.listing.item(index).data(Qt.ItemDataRole.UserRole) == selected:
                    row = index
                    break
        self.listing.setCurrentRow(row)

    def selected(self):
        item = self.listing.currentItem()
        return item.data(Qt.ItemDataRole.UserRole) if item is not None else None

    def perform(self, action):
        try:
            action()
        except (OSError, ValueError) as error:
            self.message.setText(str(error))

    def begin_prompt(self, action, target=None):
        self.prompt, self.prompt_target = action, target
        if action == "delete":
            self.message.setText(f'Delete playlist “{target}”? Enter confirms · Escape cancels. Music files are kept.')
            self.setFocus()
            return
        self.message.setText("Rename playlist:" if action == "rename" else "New playlist name:")
        self.name_input.setText(target or "")
        self.name_input.show()
        self.name_input.setFocus()
        self.name_input.selectAll()

    def cancel_prompt(self):
        self.prompt = self.prompt_target = None
        self.name_input.hide()
        self.message.clear()
        self.setFocus()

    def confirm_prompt(self):
        action, target = self.prompt, self.prompt_target
        if action == "delete":
            self.store.delete(target)
            self.cancel_prompt()
            self.refresh()
        elif action == "rename":
            name = self.store.rename(target, self.name_input.text())
            self.cancel_prompt()
            self.refresh(name)
        elif action == "create":
            name = self.store.create(self.name_input.text(), self.track)
            if self.track is not None:
                self.notice.emit(f"Added to playlist: {name}")
                self.accept()
            else:
                self.cancel_prompt()
                self.refresh(name)

    def activate_selection(self):
        if self.playlist_name is not None:
            return
        name = self.selected()
        if name is None:
            self.begin_prompt("create")
        elif self.track is not None:
            added = self.store.add(name, self.track)
            self.notice.emit(f"Added to playlist: {name}" if added else f"Already in playlist: {name}")
            self.accept()
        else:
            tracks = self.store.tracks(name)
            available = [path for path in tracks if path.is_file()]
            if not available:
                self.message.setText("This playlist has no available tracks. → inspects its entries.")
                return
            self.parent().load_saved_playlist(name, available, len(tracks) - len(available))
            self.accept()

    def remove_selection(self):
        target = self.selected()
        if target is None or self.track is not None:
            return
        if self.playlist_name is not None:
            row = self.listing.currentRow()
            self.store.remove(self.playlist_name, target)
            self.refresh()
            self.listing.setCurrentRow(min(row, self.listing.count() - 1))
        else:
            self.begin_prompt("delete", target)

    def keyPressEvent(self, event):
        key = event.key()
        if key == Qt.Key.Key_Escape:
            if self.prompt is not None:
                self.cancel_prompt()
            else:
                self.reject()
            return
        if self.name_input.hasFocus():
            super().keyPressEvent(event)
            return
        if event.modifiers() & (Qt.KeyboardModifier.ControlModifier | Qt.KeyboardModifier.AltModifier
                               | Qt.KeyboardModifier.MetaModifier):
            super().keyPressEvent(event)
            return
        if self.prompt is not None:
            if key in (Qt.Key.Key_Return, Qt.Key.Key_Enter):
                self.perform(self.confirm_prompt)
            return
        if key in (Qt.Key.Key_Up, Qt.Key.Key_Down):
            count = self.listing.count()
            if count:
                delta = 1 if key == Qt.Key.Key_Down else -1
                self.listing.setCurrentRow((self.listing.currentRow() + delta) % count)
        elif key in (Qt.Key.Key_Return, Qt.Key.Key_Enter):
            self.perform(self.activate_selection)
        elif key == Qt.Key.Key_Right and self.track is None and self.playlist_name is None:
            self.playlist_name = self.selected()
            if self.playlist_name is not None:
                self.refresh()
        elif key == Qt.Key.Key_Left and self.playlist_name is not None:
            name = self.playlist_name
            self.playlist_name = None
            self.refresh(name)
        elif key == Qt.Key.Key_F2 and self.track is None and self.playlist_name is None:
            if self.selected() is not None:
                self.begin_prompt("rename", self.selected())
        elif key == Qt.Key.Key_Delete:
            self.perform(self.remove_selection)
        else:
            super().keyPressEvent(event)
