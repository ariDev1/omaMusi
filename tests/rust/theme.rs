use omamusi::theme::Theme;
use std::fs;

#[test]
fn absent_theme_uses_complete_builtin_palette_and_monospace() {
    let root = tempfile::tempdir().unwrap();
    let theme = Theme::load_from(root.path());
    assert_eq!(theme.font, "monospace");
    for name in [
        "background",
        "foreground",
        "accent",
        "dark_foreground",
        "light_foreground",
        "cyan",
        "green",
        "bright_foreground",
        "magenta",
    ] {
        assert!(theme.colors.contains_key(name));
    }
    assert_eq!(theme.colors["accent"], "#7aa2f7");
}
#[test]
fn valid_palette_and_font_are_read_without_changing_configuration() {
    let root = tempfile::tempdir().unwrap();
    let theme_path = root
        .path()
        .join(".local/state/omarchy/current/theme/colors.toml");
    fs::create_dir_all(theme_path.parent().unwrap()).unwrap();
    let source = "accent = '#ff0000'\ncyan = 'not a color'\n";
    fs::write(&theme_path, source).unwrap();
    let font_path = root.path().join(".config/alacritty/alacritty.toml");
    fs::create_dir_all(font_path.parent().unwrap()).unwrap();
    fs::write(&font_path, "[font.normal]\nfamily = 'Test Mono'\n").unwrap();
    let theme = Theme::load_from(root.path());
    assert_eq!(theme.colors["accent"], "#ff0000");
    assert_eq!(theme.colors["cyan"], "#7dcfff");
    assert_eq!(theme.font, "Test Mono");
    assert_eq!(fs::read_to_string(theme_path).unwrap(), source);
    assert_eq!(theme.palette()[1], [1.0, 0.0, 0.0]);
}
#[test]
fn invalid_state_palette_falls_back_to_config_palette() {
    let root = tempfile::tempdir().unwrap();
    for (directory, contents) in [
        (".local/state/omarchy/current/theme", "invalid ["),
        (".config/omarchy/current/theme", "accent = '#00ff00'"),
    ] {
        let path = root.path().join(directory);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("colors.toml"), contents).unwrap();
    }
    assert_eq!(Theme::load_from(root.path()).colors["accent"], "#00ff00");
}
