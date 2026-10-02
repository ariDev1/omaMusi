"""Read Omarchy's current palette and terminal font without changing config."""

from pathlib import Path
import tomllib

from PySide6.QtGui import QColor


DEFAULTS = {
    "background": "#1a1b26", "foreground": "#c0caf5", "accent": "#7aa2f7",
    "dark_foreground": "#565f89", "light_foreground": "#a9b1d6",
    "cyan": "#7dcfff", "green": "#9ece6a", "bright_foreground": "#ffffff",
}


def load_theme():
    colors = dict(DEFAULTS)
    home = Path.home()
    for path in (home / ".local/state/omarchy/current/theme/colors.toml",
                 home / ".config/omarchy/current/theme/colors.toml"):
        try:
            data = tomllib.loads(path.read_text())
            colors.update({k: v for k, v in data.items()
                           if isinstance(v, str) and QColor(v).isValid()})
            break
        except (OSError, ValueError):
            pass
    family = "monospace"
    try:
        data = tomllib.loads((home / ".config/alacritty/alacritty.toml").read_text())
        family = data.get("font", {}).get("normal", {}).get("family", family)
    except (OSError, ValueError):
        pass
    return colors, family


def stylesheet(colors, family):
    family = family.replace("'", "").replace('"', "")
    return f"""
        QWidget {{ color: {colors['foreground']}; font-family: '{family}'; font-size: 14px; background: transparent; }}
        QLabel#muted {{ color: {colors['light_foreground']}; }}
        QLabel#accent {{ color: {colors['accent']}; }}
        QLabel#title {{ color: {colors['foreground']}; font-size: 23px; }}
        QLabel#hint {{ color: {colors['dark_foreground']}; font-size: 11px; }}
        QLineEdit {{ border: none; padding: 0; color: {colors['foreground']}; selection-background-color: {colors['accent']}; selection-color: {colors['background']}; }}
        QListWidget {{ border: none; outline: none; background: transparent; }}
        QListWidget::item {{ padding: 5px 0; color: {colors['light_foreground']}; background: transparent; }}
        QListWidget::item:selected {{ color: {colors['accent']}; background: transparent; }}
        QToolTip {{ color: {colors['foreground']}; background: {colors['background']}; border: none; }}
    """
