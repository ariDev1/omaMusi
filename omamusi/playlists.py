"""Named playlists store references to files, never the music itself."""
import json
import os
from pathlib import Path
import tempfile


class PlaylistStore:
    def __init__(self, path=None):
        data_home = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share")
        self.path = Path(path) if path is not None else data_home / "omamusi/playlists.json"
        self._playlists = {}
        try:
            data = json.loads(self.path.read_text(encoding="utf-8"))
        except FileNotFoundError:
            return
        except (ValueError, UnicodeError) as error:
            raise ValueError("Could not read saved playlists; the existing file was preserved.") from error
        if (not isinstance(data, dict) or data.get("version") != 1
                or not isinstance(data.get("playlists"), dict)):
            raise ValueError("Invalid saved playlist format; the existing file was preserved.")
        for name, tracks in data["playlists"].items():
            if (not isinstance(name, str) or not name.strip()
                    or not isinstance(tracks, list)
                    or any(not isinstance(track, str) or not Path(track).is_absolute() for track in tracks)):
                raise ValueError("Invalid saved playlist entries; the existing file was preserved.")
        self._playlists = data["playlists"]

    def names(self):
        return sorted(self._playlists, key=str.casefold)

    def tracks(self, name):
        return [Path(track) for track in self._playlists[name]]

    def _name(self, name, previous=None):
        name = name.strip()
        if not name or len(name) > 120 or any(ord(c) < 32 for c in name):
            raise ValueError("Use a playlist name of 1–120 characters without line breaks.")
        if any(other != previous and other.casefold() == name.casefold()
               for other in self._playlists):
            raise ValueError("A playlist with that name already exists.")
        return name

    def _save(self, playlists):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=self.path.parent,
                                             prefix=".playlists-", delete=False) as handle:
                temporary = Path(handle.name)
                json.dump({"version": 1, "playlists": playlists}, handle,
                          ensure_ascii=False, indent=2)
                handle.write("\n")
                handle.flush()
                os.fsync(handle.fileno())
            os.replace(temporary, self.path)
            self._playlists = playlists
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)

    def create(self, name, track=None):
        name = self._name(name)
        tracks = [] if track is None else [str(Path(track).expanduser().resolve())]
        self._save({**self._playlists, name: tracks})
        return name

    def add(self, name, track):
        track = str(Path(track).expanduser().resolve())
        tracks = self._playlists[name]
        if track in tracks:
            return False
        self._save({**self._playlists, name: [*tracks, track]})
        return True

    def rename(self, name, replacement):
        replacement = self._name(replacement, previous=name)
        playlists = dict(self._playlists)
        tracks = playlists.pop(name)
        playlists[replacement] = tracks
        self._save(playlists)
        return replacement

    def remove(self, name, track):
        track = str(Path(track).expanduser().resolve())
        self._save({**self._playlists,
                    name: [value for value in self._playlists[name] if value != track]})

    def delete(self, name):
        playlists = dict(self._playlists)
        del playlists[name]
        self._save(playlists)
