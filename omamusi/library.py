"""Playlist discovery, independent of the GUI."""

from pathlib import Path
import re


AUDIO_EXTENSIONS = {
    ".mp3", ".flac", ".wav", ".ogg", ".oga", ".opus", ".m4a", ".aac",
    ".aiff", ".aif", ".wma", ".ape", ".alac", ".mka", ".mp2", ".ac3",
    ".wv", ".amr", ".au",
}


def natural_key(path: Path) -> list:
    return [int(part) if part.isdigit() else part.casefold()
            for part in re.split(r"(\d+)", str(path))]


def discover(paths: list[str], recursive: bool = False) -> list[Path]:
    """Expand directories; retain explicit-file order and deduplicate paths."""
    result = []
    seen = set()
    for value in paths:
        path = Path(value).expanduser().resolve()
        if not path.exists():
            raise ValueError(f"Path does not exist: {value}")
        if path.is_dir():
            candidates = sorted(
                (p for p in (path.rglob("*") if recursive else path.iterdir())
                 if p.is_file() and p.suffix.lower() in AUDIO_EXTENSIONS),
                key=natural_key,
            )
        else:
            # Explicit files may use extensions FFmpeg supports beyond this list.
            candidates = [path]
        for candidate in candidates:
            candidate = candidate.resolve()
            if candidate not in seen:
                seen.add(candidate)
                result.append(candidate)
    return result
