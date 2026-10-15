use omamusi::install::Installation;
use std::{fs, os::unix::fs::symlink};

#[test]
fn native_install_upgrade_remove_preserves_playlists_and_music() {
    let root = tempfile::Builder::new()
        .prefix("oma native spaces ")
        .tempdir()
        .unwrap();
    let install = Installation::new(root.path().to_owned(), root.path().join("user data"));
    let source = root.path().join("source");
    fs::write(&source, "first").unwrap();
    fs::create_dir_all(install.directory()).unwrap();
    let playlists = install.directory().join("playlists.json");
    fs::write(&playlists, "keep").unwrap();
    let music = root.path().join("song.flac");
    fs::write(&music, "music").unwrap();
    install.install(&source).unwrap();
    install.install(&source).unwrap();
    assert_eq!(fs::read_link(install.launcher()).unwrap(), install.binary());
    assert_eq!(fs::read_to_string(install.binary()).unwrap(), "first");
    fs::write(&source, "second").unwrap();
    install.install(&source).unwrap();
    assert_eq!(fs::read_to_string(install.binary()).unwrap(), "second");
    // procfs reports a regular file but fails the first copy read with EIO.
    assert!(
        install
            .install(std::path::Path::new("/proc/self/mem"))
            .is_err()
    );
    assert_eq!(fs::read_to_string(install.binary()).unwrap(), "second");
    assert!(
        install
            .install(&root.path().join("missing source"))
            .is_err()
    );
    assert_eq!(fs::read_to_string(install.binary()).unwrap(), "second");
    install.remove().unwrap();
    install.remove().unwrap();
    assert!(!install.launcher().exists());
    assert!(!install.binary().exists());
    assert_eq!(fs::read_to_string(playlists).unwrap(), "keep");
    assert_eq!(fs::read_to_string(music).unwrap(), "music");
}

#[test]
fn unrelated_files_and_symlinks_are_refused_before_mutation() {
    let root = tempfile::tempdir().unwrap();
    let install = Installation::new(root.path().to_owned(), root.path().join("data"));
    let source = root.path().join("source");
    fs::write(&source, "native").unwrap();
    fs::create_dir_all(install.launcher().parent().unwrap()).unwrap();
    fs::write(install.launcher(), "other executable").unwrap();
    assert!(install.install(&source).is_err());
    assert!(!install.directory().exists());
    assert_eq!(
        fs::read_to_string(install.launcher()).unwrap(),
        "other executable"
    );
    fs::remove_file(install.launcher()).unwrap();
    fs::create_dir_all(install.directory()).unwrap();
    fs::write(install.directory().join("unrelated"), "keep").unwrap();
    assert!(install.install(&source).is_err());
    assert!(install.remove().is_err());
    fs::remove_file(install.directory().join("unrelated")).unwrap();
    symlink(&source, install.directory().join(".omamusi-managed")).unwrap();
    assert!(install.install(&source).is_err());
    assert_eq!(fs::read_to_string(source).unwrap(), "native");
}

#[test]
fn recognized_python_installation_upgrades_without_running_python() {
    let root = tempfile::tempdir().unwrap();
    let install = Installation::new(root.path().to_owned(), root.path().join("data"));
    let source = root.path().join("native");
    fs::write(&source, "native").unwrap();
    let legacy = install.directory().join("venv/bin/python");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, "old").unwrap();
    fs::write(
        install.directory().join(".omamusi-managed"),
        "io.github.aridev1.omamusi\n",
    )
    .unwrap();
    fs::create_dir_all(install.launcher().parent().unwrap()).unwrap();
    symlink(
        install.directory().join("venv/bin/omaMusi"),
        install.launcher(),
    )
    .unwrap();
    let entry = root
        .path()
        .join("data/applications/io.github.aridev1.omamusi.desktop");
    fs::create_dir_all(entry.parent().unwrap()).unwrap();
    fs::write(
        &entry,
        format!(
            "[Desktop Entry]\nExec=\"{}\" -m omamusi.desktop launch\nX-omaMusi-Managed=true\n",
            legacy.display()
        ),
    )
    .unwrap();
    install.install(&source).unwrap();
    assert!(
        fs::read_to_string(entry)
            .unwrap()
            .contains("--desktop launch")
    );
    assert!(!install.directory().join("venv").exists());
}

#[test]
fn missing_dependencies_do_not_create_an_installation() {
    let root = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_omaMusi"))
        .arg("--install")
        .env("HOME", root.path())
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("ffmpeg"));
    assert!(!root.path().join("data").exists());
}

#[test]
fn installed_native_executable_and_remove_script_work_without_python() {
    let root = tempfile::Builder::new()
        .prefix("oma actual 50% ")
        .tempdir()
        .unwrap();
    let data = root.path().join("data");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_omaMusi"))
        .arg("--install")
        .env("HOME", root.path())
        .env("XDG_DATA_HOME", &data)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let installed = root.path().join(".local/bin/omaMusi");
    assert!(
        std::process::Command::new(&installed)
            .arg("--version")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        std::process::Command::new("desktop-file-validate")
            .arg(data.join("applications/io.github.aridev1.omamusi.desktop"))
            .status()
            .unwrap()
            .success()
    );
    let playlists = data.join("omamusi/playlists.json");
    fs::write(&playlists, "keep").unwrap();
    let output = std::process::Command::new("bash")
        .arg("scripts/remove-player.sh")
        .env("HOME", root.path())
        .env("XDG_DATA_HOME", &data)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!installed.exists());
    assert_eq!(fs::read_to_string(playlists).unwrap(), "keep");
}

#[test]
fn removal_script_never_executes_an_unmanaged_binary() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("data");
    let binary = data.join("omamusi/bin/omaMusi");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    let sentinel = root.path().join("executed");
    fs::write(
        &binary,
        format!("#!/bin/sh\nprintf ran > '{}'\n", sentinel.display()),
    )
    .unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    let output = std::process::Command::new("bash")
        .arg("scripts/remove-player.sh")
        .env("HOME", root.path())
        .env("XDG_DATA_HOME", &data)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!sentinel.exists());
    assert!(binary.exists());
}

#[test]
fn relative_xdg_data_home_uses_standard_absolute_user_directory() {
    let root = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_omaMusi"))
        .arg("--install")
        .current_dir(root.path())
        .env("HOME", root.path())
        .env("XDG_DATA_HOME", "relative-data")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        root.path()
            .join(".local/share/omamusi/bin/omaMusi")
            .is_file()
    );
    assert!(!root.path().join("relative-data").exists());
}

#[test]
fn failed_first_install_and_interrupted_copy_remain_retryable() {
    for fail_copy in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        let install = Installation::new(root.path().to_owned(), data.clone());
        let source = root.path().join("source");
        fs::write(&source, "native").unwrap();
        fs::create_dir_all(install.directory()).unwrap();
        let playlists = install.directory().join("playlists.json");
        fs::write(&playlists, "preserve playlists").unwrap();
        let obstacle = data.join("icons");
        if !fail_copy {
            fs::write(&obstacle, "unrelated").unwrap();
        }
        let first_source = if fail_copy {
            std::path::Path::new("/proc/self/mem")
        } else {
            source.as_path()
        };
        assert!(install.install(first_source).is_err());
        if !fail_copy {
            assert_eq!(fs::read_to_string(&obstacle).unwrap(), "unrelated");
            fs::remove_file(&obstacle).unwrap();
        }
        install
            .install(&source)
            .expect("failed initial installation must allow a safe retry");
        assert_eq!(fs::read_to_string(install.binary()).unwrap(), "native");
        assert_eq!(
            fs::read_to_string(&playlists).unwrap(),
            "preserve playlists"
        );
        install.remove().unwrap();
        assert_eq!(fs::read_to_string(playlists).unwrap(), "preserve playlists");
    }
}
