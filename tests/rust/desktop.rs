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
