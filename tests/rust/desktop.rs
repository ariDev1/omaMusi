use omamusi::desktop::Desktop;
use std::fs;

#[test]
fn desktop_ownership_escaping_and_preservation() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("50% player/omaMusi");
    let desktop = Desktop::new(root.path().join("data"), executable);
    desktop.install().unwrap();
    desktop.install().unwrap();
    let text = fs::read_to_string(desktop.entry()).unwrap();
    assert!(text.contains("50%% player/omaMusi\" --desktop launch"));
    assert!(text.contains("X-omaMusi-Managed=true\n"));
    let other = Desktop::new(root.path().join("data"), root.path().join("other"));
    assert!(other.install().is_err());
    other.remove().unwrap();
    assert!(desktop.entry().exists());
    let playlists = root.path().join("data/omamusi/playlists.json");
    fs::create_dir_all(playlists.parent().unwrap()).unwrap();
    fs::write(&playlists, "keep").unwrap();
    desktop.remove().unwrap();
    desktop.remove().unwrap();
    assert!(!desktop.entry().exists());
    assert_eq!(fs::read_to_string(playlists).unwrap(), "keep");
}

#[test]
fn unrelated_entries_and_icons_are_preserved() {
    let root = tempfile::tempdir().unwrap();
    let desktop = Desktop::new(root.path().to_owned(), root.path().join("omaMusi"));
    fs::create_dir_all(desktop.entry().parent().unwrap()).unwrap();
    fs::write(desktop.entry(), "Other application").unwrap();
    assert!(desktop.install().is_err());
    desktop.remove().unwrap();
    assert_eq!(
        fs::read_to_string(desktop.entry()).unwrap(),
        "Other application"
    );
    fs::remove_file(desktop.entry()).unwrap();
    fs::create_dir_all(desktop.icon().parent().unwrap()).unwrap();
    fs::write(desktop.icon(), "Other icon").unwrap();
    assert!(desktop.install().is_err());
    assert!(!desktop.entry().exists());
    assert_eq!(fs::read_to_string(desktop.icon()).unwrap(), "Other icon");
}

#[test]
fn configured_localized_music_is_recursive_and_missing_or_home_opens_empty() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config");
    fs::create_dir(&config).unwrap();
    let music = root.path().join("Meine Musik");
    fs::create_dir(&music).unwrap();
    let directory_file = config.join("user-dirs.dirs");
    for (setting, expected) in [
        ("$HOME/Meine Musik", Some(music.clone())),
        ("$HOME", None),
        ("$HOME/missing", None),
    ] {
        fs::write(&directory_file, format!("XDG_MUSIC_DIR=\"{setting}\"\n")).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_player-smoke"))
            .arg("--music-config")
            .env("HOME", root.path())
            .env("XDG_CONFIG_HOME", &config)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let (paths, empty, recursive, view): (Vec<std::path::PathBuf>, bool, bool, String) =
            serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(paths, expected.clone().into_iter().collect::<Vec<_>>());
        assert_eq!(empty, expected.is_none());
        assert!(recursive);
        assert_eq!(view, "Event Horizon");
    }
}
