"""Playlist discovery, independent of the GUI."""

from pathlib import Path
import os
import re
import stat


AUDIO_EXTENSIONS = {
    ".mp3", ".flac", ".wav", ".ogg", ".oga", ".opus", ".m4a", ".aac",
    ".aiff", ".aif", ".wma", ".ape", ".alac", ".mka", ".mp2", ".ac3",
    ".wv", ".amr", ".au",
}


def natural_key(path: Path) -> list:
    return [int(part) if part.isdigit() else part.casefold()
            for part in re.split(r"(\d+)", str(path))]


def directory_entries(folder):
    """Classify with stat(), which preserves permission and device errors."""
    with os.scandir(folder) as entries:
        return [(Path(entry.path), entry.stat().st_mode, entry.is_symlink()) for entry in entries]


def _audio_files(folder, recursive):
    pending = [folder]
    while pending:
        for path, mode, symlink in directory_entries(pending.pop()):
            if stat.S_ISDIR(mode):
                if recursive and not symlink:
                    pending.append(path)
            elif stat.S_ISREG(mode) and path.suffix.lower() in AUDIO_EXTENSIONS:
                yield path


def discover(paths: list[str], recursive: bool = False) -> list[Path]:
    """Expand directories; retain explicit-file order and deduplicate paths."""
    result = []
    seen = set()
    for value in paths:
        path = Path(value).expanduser().resolve()
        try:
            mode = path.stat().st_mode
        except FileNotFoundError as error:
            raise ValueError(f"Path does not exist: {value}") from error
        if stat.S_ISDIR(mode):
            candidates = sorted(_audio_files(path, recursive), key=natural_key)
        elif stat.S_ISREG(mode):
            # Explicit files may use extensions FFmpeg supports beyond this list.
            candidates = [path]
        else:
            raise ValueError(f"Not a regular audio file or directory: {value}")
        for candidate in candidates:
            candidate = candidate.resolve()
            if candidate not in seen:
                seen.add(candidate)
                result.append(candidate)
    return result
