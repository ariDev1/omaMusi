"""Install a user-local application menu entry and launch the Music folder."""
import argparse
from importlib.resources import files
import os
from pathlib import Path
import sys
import tempfile


APP_ID = 'io.github.aridev1.omamusi'


def _paths():
    configured = Path(os.environ.get('XDG_DATA_HOME') or '')
    data = configured if configured.is_absolute() else Path.home() / '.local/share'
    return (data / 'applications' / f'{APP_ID}.desktop',
            data / 'icons/hicolor/scalable/apps' / f'{APP_ID}.svg')


def _command():
    # Exec has both desktop string escaping and command argument quoting.
    executable = str(Path(sys.executable).absolute())
    if '=' in executable or any(char in executable for char in '\n\r\t'):
        raise ValueError('This Python installation path cannot be used in a desktop launcher.')
    escaped = ''.join('\\' + char if char in '\\"`$' else char for char in executable)
    escaped = escaped.replace('\\', '\\\\').replace('%', '%%')
    return f'"{escaped}" -m omamusi.desktop launch'


def _icon():
    return files('omamusi').joinpath('icon.svg').read_bytes()


def _owned(entry):
    if entry.is_symlink() or not entry.is_file():
        return False
    lines = entry.read_text(encoding='utf-8').splitlines()
    return 'X-omaMusi-Managed=true' in lines and f'Exec={_command()}' in lines


def install():
    entry, icon = _paths()
    if (entry.exists() or entry.is_symlink()) and not _owned(entry):
        raise ValueError(f'Another launcher already exists at {entry}. Remove it using its original installation first.')
    artwork = _icon()
    if (icon.exists() or icon.is_symlink()) and (
            icon.is_symlink() or not icon.is_file() or icon.read_bytes() != artwork):
        raise ValueError(f'An unrelated icon already exists at {icon}.')
    command = _command()
    icon.parent.mkdir(parents=True, exist_ok=True)
    entry.parent.mkdir(parents=True, exist_ok=True)
    icon.write_bytes(artwork)
    entry.write_text(
        '[Desktop Entry]\n'
        'Type=Application\n'
        'Name=omaMusi\n'
        'Comment=Play your music with animated visuals\n'
        f'Exec={command}\n'
        f'Icon={APP_ID}\n'
        'Terminal=false\n'
        'Categories=AudioVideo;Audio;Player;\n'
        'Keywords=Music;Songs;Playlist;Visualizer;\n'
        'StartupNotify=false\n'
        'StartupWMClass=omaMusi\n'
        'X-omaMusi-Managed=true\n', encoding='utf-8')
    return entry


def remove():
    entry, icon = _paths()
    if not _owned(entry):
        return
    entry.unlink()
    if icon.is_file() and not icon.is_symlink() and icon.read_bytes() == _icon():
        icon.unlink()


def launch():
    from PySide6.QtCore import QStandardPaths
    from .app import main as player_main

    music = Path(QStandardPaths.writableLocation(QStandardPaths.StandardLocation.MusicLocation))
    args = ['--view', 'event horizon']
    if music.is_absolute() and music != Path.home() and music.is_dir():
        return player_main(args + [str(music), '--recursive'])
    # Keep the empty directory alive until the player's background scan finishes.
    with tempfile.TemporaryDirectory(prefix='omamusi-empty-') as empty:
        return player_main(args + [empty])


def main(argv=None):
    parser = argparse.ArgumentParser(description='Manage the omaMusi application-menu launcher.')
    parser.add_argument('action', choices=('install', 'remove', 'launch'))
    args = parser.parse_args(argv)
    try:
        if args.action == 'launch':
            return launch()
        if args.action == 'install':
            entry = install()
            print(f'Installed {entry}. Open omaMusi from your application menu.')
        else:
            remove()
            print('Removed the menu entry for this installation. Music and playlists are kept.')
        return 0
    except (OSError, ValueError) as error:
        print(f'omaMusi: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
