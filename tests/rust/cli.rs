use omamusi::{
    cli::{Cli, DesktopAction},
    visual::VisualMode,
};
use std::path::{Path, PathBuf};

#[test]
fn default_startup_is_recursive_cwd_with_warp() {
    let cli = Cli::parse_from(["omaMusi"]).unwrap();
    assert_eq!(cli.view, VisualMode::Warp);
    assert_eq!(
        cli.startup_paths(Path::new("/music")),
        [PathBuf::from("/music")]
    );
    assert!(cli.scan_recursive());
}
#[test]
fn explicit_paths_keep_order_and_recursion_flags() {
    let cli = Cli::parse_from(["omaMusi", "/music/two.flac", "/music/one.mp3"]).unwrap();
    assert_eq!(
        cli.startup_paths(Path::new("/cwd")),
        [
            PathBuf::from("/music/two.flac"),
            PathBuf::from("/music/one.mp3")
        ]
    );
    assert!(!cli.scan_recursive());
    let cli = Cli::parse_from(["omaMusi", "-r", "/music"]).unwrap();
    assert!(cli.scan_recursive());
    let cli = Cli::parse_from(["omaMusi", "-all", "/extra.flac"]).unwrap();
    assert!(cli.all);
    assert_eq!(
        cli.startup_paths(Path::new("/cwd")),
        [PathBuf::from("/cwd"), PathBuf::from("/extra.flac")]
    );
}
#[test]
fn all_view_names_and_legacy_aliases_parse() {
    for mode in VisualMode::ALL {
        assert_eq!(
            Cli::parse_from(["omaMusi", "--view", &mode.name().to_lowercase()])
                .unwrap()
                .view,
            mode
        );
    }
    for (alias, want) in [
        ("sprites", VisualMode::Warp),
        ("phi", VisualMode::PhiCathedral),
        ("cathedral", VisualMode::PhiCathedral),
        ("phi-cathedral", VisualMode::PhiCathedral),
    ] {
        assert_eq!(
            Cli::parse_from(["omaMusi", "--view", alias]).unwrap().view,
            want
        );
    }
    assert!(Cli::parse_from(["omaMusi", "--view", "missing"]).is_err());
}
#[test]
fn version_help_and_desktop_actions_do_not_require_a_display() {
    let version = Cli::parse_from(["omaMusi", "--version"]).unwrap_err();
    assert_eq!(version.kind(), clap::error::ErrorKind::DisplayVersion);
    assert!(version.to_string().contains("omaMusi 0.11.0"));
    assert_eq!(
        Cli::parse_from(["omaMusi", "--help"]).unwrap_err().kind(),
        clap::error::ErrorKind::DisplayHelp
    );
    assert_eq!(
        Cli::parse_from(["omaMusi", "--desktop", "install"])
            .unwrap()
            .desktop,
        Some(DesktopAction::Install)
    );
    assert!(Cli::parse_from(["omaMusi", "--desktop", "invalid"]).is_err());
}
#[test]
fn empty_menu_launch_never_scans_cwd() {
    let cli = Cli::for_music(None);
    assert_eq!(cli.view, VisualMode::EventHorizon);
    assert!(cli.startup_paths(Path::new("/unrelated")).is_empty());
    let cli = Cli::for_music(Some(PathBuf::from("/configured/Musik")));
    assert_eq!(
        cli.startup_paths(Path::new("/unrelated")),
        [PathBuf::from("/configured/Musik")]
    );
    assert!(cli.scan_recursive());
}
