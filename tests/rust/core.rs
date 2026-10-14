use omamusi::{
    library::{discover, matches},
    playlists::PlaylistStore,
};
use std::{fs, path::PathBuf};
use tempfile::tempdir;

#[test]
fn discovery_natural_order_dedup_recursion_and_explicit_order() {
    let root = tempdir().unwrap();
    for name in ["10.mp3", "2.FLAC", "readme.txt"] {
        fs::write(root.path().join(name), b"").unwrap();
    }
    fs::create_dir(root.path().join("nested")).unwrap();
    fs::write(root.path().join("nested/a.opus"), b"").unwrap();
    let tracks = discover(&[root.path().into(), root.path().join("2.FLAC")], false).unwrap();
    assert_eq!(
        tracks
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect::<Vec<_>>(),
        ["2.FLAC", "10.mp3"]
    );
    assert_eq!(discover(&[root.path().into()], true).unwrap().len(), 3);
    let ordered = vec![root.path().join("10.mp3"), root.path().join("2.FLAC")];
    assert_eq!(discover(&ordered, false).unwrap(), ordered);
    assert!(discover(&[root.path().join("absent")], true).is_err());
    assert_eq!(
        discover(&[root.path().join("readme.txt")], false)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn discovery_preserves_unicode_metacharacters_and_non_utf8_paths() {
    use std::os::unix::ffi::OsStringExt;
    let root = tempdir().unwrap();
    let names = [
        std::ffi::OsString::from("Straße 音楽 $(touch nope) `echo x`.flac"),
        std::ffi::OsString::from_vec(b"invalid\xff.mp3".to_vec()),
    ];
    for name in &names {
        fs::write(root.path().join(name), b"").unwrap();
    }
    let tracks = discover(&[root.path().into()], true).unwrap();
    assert_eq!(tracks.len(), 2);
    for name in &names {
        assert!(tracks.contains(&root.path().join(name)));
    }
    assert!(matches(
        tracks
            .iter()
            .find(|p| p.to_string_lossy().contains("Straße"))
            .unwrap(),
        "STRASSE"
    ));
}

#[test]
fn discovery_does_not_follow_recursive_symlink_loops_and_reports_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempdir().unwrap();
    fs::create_dir(root.path().join("child")).unwrap();
    fs::write(root.path().join("child/song.mp3"), b"").unwrap();
    symlink(root.path(), root.path().join("child/loop")).unwrap();
    symlink(
        root.path().join("child/song.mp3"),
        root.path().join("alias.mp3"),
    )
    .unwrap();
    assert_eq!(discover(&[root.path().into()], true).unwrap().len(), 1);
    let blocked = root.path().join("blocked");
    fs::create_dir(&blocked).unwrap();
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o0)).unwrap();
    let result = discover(&[root.path().into()], true);
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
}

#[test]
fn search_matches_parent_folders_case_insensitively_and_empty_query() {
    let track = PathBuf::from("/music/Artist/Album/Disc 2/song.flac");
    for query in ["", "aRtIsT", "ALBUM", "disc 2", "SONG"] {
        assert!(matches(&track, query));
    }
    assert!(!matches(&track, "absent"));
}

#[test]
fn playlists_persist_order_deduplicate_and_leave_music_untouched() {
    let root = tempdir().unwrap();
    let data = root.path().join("playlists.json");
    let one = root.path().join("one.flac");
    let two = root.path().join("two.mp3");
    fs::write(&one, b"music").unwrap();
    fs::write(&two, b"music").unwrap();
    let mut store = PlaylistStore::open(data.clone()).unwrap();
    store.create(" Evening ", Some(&two)).unwrap();
    assert!(store.add("Evening", &one).unwrap());
    assert!(!store.add("Evening", &one).unwrap());
    let mut store = PlaylistStore::open(data.clone()).unwrap();
    assert_eq!(store.tracks("Evening").unwrap(), [two.clone(), one.clone()]);
    store.rename("Evening", "Morning").unwrap();
    store.remove("Morning", &two).unwrap();
    assert_eq!(store.tracks("Morning").unwrap(), std::slice::from_ref(&one));
    store.delete("Morning").unwrap();
    assert!(PlaylistStore::open(data).unwrap().names().is_empty());
    assert_eq!(fs::read(one).unwrap(), b"music");
    assert_eq!(fs::read(two).unwrap(), b"music");
}

#[test]
fn playlist_invalid_names_and_collisions_preserve_bytes() {
    let root = tempdir().unwrap();
    let data = root.path().join("playlists.json");
    let mut store = PlaylistStore::open(data.clone()).unwrap();
    store.create("Straße", None).unwrap();
    let original = fs::read(&data).unwrap();
    for name in [
        "",
        "  ",
        "line\nbreak",
        "bad\0name",
        "STRASSE",
        &"x".repeat(121),
    ] {
        assert!(store.create(name, None).is_err(), "{name:?}");
        assert_eq!(fs::read(&data).unwrap(), original);
    }
    store.create(&"音".repeat(120), None).unwrap();
}

#[test]
fn malformed_playlist_and_failed_save_preserve_data() {
    let root = tempdir().unwrap();
    let data = root.path().join("playlists.json");
    for contents in [
        "{",
        r#"{"version":2,"playlists":{}}"#,
        r#"{"version":1,"playlists":{"Test":["relative.mp3"]}}"#,
        r#"{"version":1,"playlists":{"Test":[3]}}"#,
    ] {
        fs::write(&data, contents).unwrap();
        assert!(PlaylistStore::open(data.clone()).is_err());
        assert_eq!(fs::read_to_string(&data).unwrap(), contents);
    }
    fs::write(
        &data,
        r#"{"version":1,"playlists":{"Keep":["/missing/song.flac"]}}"#,
    )
    .unwrap();
    let mut store = PlaylistStore::open(data.clone()).unwrap();
    fs::rename(&data, root.path().join("saved-original.json")).unwrap();
    fs::create_dir(&data).unwrap();
    assert!(store.rename("Keep", "Lost").is_err());
    assert_eq!(store.names(), ["Keep"]);
    assert_eq!(
        store.tracks("Keep").unwrap(),
        [PathBuf::from("/missing/song.flac")]
    );
    assert!(fs::read_dir(root.path()).unwrap().all(|p| {
        !p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".playlists-")
    }));
}

#[test]
fn baseline_version_one_json_is_read_without_migration() {
    let root = tempdir().unwrap();
    let data = root.path().join("playlists.json");
    fs::write(
        &data,
        r#"{"version":1,"playlists":{"夜":["/music/音.flac","/music/missing.mp3"]}}"#,
    )
    .unwrap();
    let store = PlaylistStore::open(data).unwrap();
    assert_eq!(store.names(), ["夜"]);
    assert_eq!(
        store.tracks("夜").unwrap(),
        [
            PathBuf::from("/music/音.flac"),
            PathBuf::from("/music/missing.mp3")
        ]
    );
}
