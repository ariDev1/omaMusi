//! Exercise the production controller and widgets on one GTK thread.
use super::*;
use gtk::{gdk, gio};
use std::{
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

#[track_caller]
fn wait(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        while glib::MainContext::default().pending() {
            glib::MainContext::default().iteration(false);
        }
        if predicate() {
            return;
        }
        assert!(Instant::now() < deadline, "GUI condition timed out");
        thread::sleep(Duration::from_millis(5));
    }
}
fn key(window: &Rc<PlayerWindow>, key: gdk::Key) {
    window.handle_key(key, gdk::ModifierType::empty());
}
fn make_tone(path: &Path) {
    assert!(
        Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=30",
                "-metadata",
                "title=GUI test tone"
            ])
            .arg(path)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
#[ignore = "requires a desktop, audio service and GPU; cargo run --bin player-smoke"]
fn desktop_workflows() {
    gtk::init().unwrap();
    let root = tempfile::tempdir().unwrap();
    let album = root.path().join("Music/Artist/Album");
    std::fs::create_dir_all(&album).unwrap();
    let one = album.join("01 one.wav");
    let two = album.join("02 two.wav");
    make_tone(&one);
    std::fs::copy(&one, &two).unwrap();
    let app = gtk::Application::builder()
        .application_id(format!(
            "io.github.aridev1.omamusi.Test{}",
            std::process::id()
        ))
        .build();
    app.register(None::<&gio::Cancellable>).unwrap();
    let cli = Cli::for_music(None);
    let player = PlayerWindow::new(
        &app,
        &cli,
        root.path().join("data"),
        root.path().join("home"),
    )
    .unwrap();
    player.state.borrow_mut().audio.set_volume(0.0);
    assert!(player.state.borrow().model.tracks.is_empty());
    let activations = Rc::new(Cell::new(0));
    let count = activations.clone();
    let existing = Rc::downgrade(&player);
    app.connect_activate(move |_| {
        count.set(count.get() + 1);
        existing.upgrade().unwrap().activate();
    });
    player.window.present();
    player.dispatch(Action::AddPaths(vec![root.path().join("Music")], true));
    wait(|| {
        player.state.borrow().model.tracks.len() == 2
            && player.state.borrow().audio.snapshot().position > 0.05
    });
    assert_eq!(player.title.text(), "GUI test tone");
    assert!(player.state.borrow().audio.snapshot().has_stream);
    let driver = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("player-smoke");
    let generation = player.state.borrow().audio.snapshot().generation;
    player.window.minimize();
    let mut remote = Command::new(&driver)
        .args(["--activate", app.application_id().unwrap().as_str()])
        .spawn()
        .unwrap();
    wait(|| remote.try_wait().unwrap().is_some());
    assert!(remote.wait().unwrap().success());
    wait(|| activations.get() == 1);
    assert_eq!(
        player.state.borrow().audio.snapshot().generation,
        generation
    );
    assert!(player.state.borrow().audio.snapshot().has_stream);
    let crash_id = format!("io.github.aridev1.omamusi.Crash{}", std::process::id());
    let ready = root.path().join("ready");
    for _ in 0..2 {
        let mut child = Command::new(&driver)
            .args(["--hold", &crash_id, ready.to_str().unwrap()])
            .spawn()
            .unwrap();
        wait(|| ready.exists());
        child.kill().unwrap();
        child.wait().unwrap();
        std::fs::remove_file(&ready).unwrap();
    }
    // Toggle pause while metadata is queued, before the GTK loop can apply it.
    player.dispatch(Action::Play(0));
    assert!(player.state.borrow().model.loading);
    key(&player, gdk::Key::space);
    wait(|| !player.state.borrow().model.loading);
    assert!(player.state.borrow().audio.snapshot().paused);
    assert!(player.state.borrow().audio.snapshot().paused);
    let paused = player.state.borrow().audio.snapshot().position;
    let initial_generation = player.state.borrow().audio.snapshot().generation;
    for _ in 0..9 {
        key(&player, gdk::Key::v);
    }
    assert_eq!(player.state.borrow().visual.mode, VisualMode::EventHorizon);
    assert_eq!(
        player.state.borrow().audio.snapshot().generation,
        initial_generation
    );
    assert!(player.state.borrow().audio.snapshot().paused);
    assert!((player.state.borrow().audio.snapshot().position - paused).abs() < 0.05);
    player.handle_key(gdk::Key::V, gdk::ModifierType::SHIFT_MASK);
    assert_eq!(player.state.borrow().visual.mode, VisualMode::PhiCathedral);
    key(&player, gdk::Key::slash);
    player.search.set_text("ALBUM");
    assert_eq!(player.state.borrow().model.visible_indices(), [0, 1]);
    key(&player, gdk::Key::Down);
    assert!(!player.state.borrow().model.search_focus);
    key(&player, gdk::Key::slash);
    player.search.set_text("no matches");
    key(&player, gdk::Key::Down);
    assert!(player.state.borrow().model.search_focus);
    let mode = player.state.borrow().visual.mode;
    assert_eq!(
        player.handle_key(gdk::Key::v, gdk::ModifierType::empty()),
        glib::Propagation::Proceed
    );
    assert_eq!(player.state.borrow().visual.mode, mode);
    key(&player, gdk::Key::Escape);
    player.search.set_text("");
    player.focus_player();
    player.dispatch(Action::Select(0));
    key(&player, gdk::Key::j);
    assert_eq!(player.state.borrow().model.selected, Some(1));
    key(&player, gdk::Key::k);
    assert_eq!(player.state.borrow().model.selected, Some(0));
    key(&player, gdk::Key::Return);
    wait(|| !player.state.borrow().model.loading);
    key(&player, gdk::Key::Right);
    wait(|| player.state.borrow().audio.snapshot().position >= 4.9);
    key(&player, gdk::Key::Left);
    wait(|| player.state.borrow().audio.snapshot().position < 1.0);
    let volume = player.state.borrow().audio.snapshot().volume;
    key(&player, gdk::Key::plus);
    assert!(player.state.borrow().audio.snapshot().volume > volume);
    key(&player, gdk::Key::minus);
    player.state.borrow_mut().audio.set_volume(0.0);
    key(&player, gdk::Key::r);
    assert!(player.state.borrow().model.shuffle);
    key(&player, gdk::Key::n);
    wait(|| !player.state.borrow().model.loading);
    assert_eq!(player.state.borrow().model.current, Some(1));
    key(&player, gdk::Key::p);
    wait(|| !player.state.borrow().model.loading);
    assert_eq!(player.state.borrow().model.current, Some(0));
    key(&player, gdk::Key::c);
    wait(|| !player.state.borrow().browser_entries.is_empty());
    assert!(player.state.borrow().model.browser_folder.is_some());
    key(&player, gdk::Key::Left);
    wait(|| player.state.borrow().model.browser_folder.as_deref() == album.parent());
    key(&player, gdk::Key::Escape);
    assert!(player.state.borrow().model.browser_folder.is_none());
    player.handle_key(gdk::Key::l, gdk::ModifierType::CONTROL_MASK);
    assert!(player.folder_prompt.is_visible());
    player.folder_input.set_text("");
    player.folder_input.emit_activate();
    assert!(player.state.borrow().model.status.contains("enter a path"));
    player.folder_input.set_text("..");
    player.folder_input.emit_activate();
    wait(|| !player.folder_prompt.is_visible());
    assert!(player.state.borrow().model.tracks.is_empty());
    player.focus_player();
    player.handle_key(gdk::Key::l, gdk::ModifierType::CONTROL_MASK);
    player.folder_input.set_text(album.to_str().unwrap());
    player.folder_input.emit_activate();
    wait(|| !player.folder_prompt.is_visible());
    let was_visible = player.panel.is_visible();
    key(&player, gdk::Key::Tab);
    assert_ne!(player.panel.is_visible(), was_visible);
    key(&player, gdk::Key::Tab);
    key(&player, gdk::Key::f);
    wait(|| player.window.is_fullscreen());
    key(&player, gdk::Key::Escape);
    wait(|| !player.window.is_fullscreen());
    let extra = root.path().join("extra.wav");
    std::fs::copy(&one, &extra).unwrap();
    player.drop_paths(vec![extra.clone()]);
    wait(|| player.state.borrow().model.tracks.contains(&extra));
    let generation = player.state.borrow().audio.snapshot().generation;
    key(&player, gdk::Key::a);
    wait(|| {
        player
            .playlist_dialog
            .borrow()
            .as_ref()
            .is_some_and(|dialog| !dialog.busy.get())
    });
    {
        let dialog = player.playlist_dialog.borrow().as_ref().unwrap().clone();
        dialog.activate_selection();
        assert!(gtk::prelude::WidgetExt::is_visible(&dialog.name_input));
        dialog.name_input.set_text("Evening");
        dialog.name_input.emit_activate();
    }
    wait(|| player.playlist_dialog.borrow().is_none());
    assert_eq!(
        player.state.borrow().audio.snapshot().generation,
        generation
    );
    let path = root.path().join("data/omamusi/playlists.json");
    assert_eq!(
        crate::playlists::PlaylistStore::open(path.clone())
            .unwrap()
            .names(),
        ["Evening"]
    );
    key(&player, gdk::Key::b);
    wait(|| {
        player
            .playlist_dialog
            .borrow()
            .as_ref()
            .is_some_and(|dialog| !dialog.busy.get())
    });
    let dialog = player.playlist_dialog.borrow().as_ref().unwrap().clone();
    dialog.handle_key(gdk::Key::F2, gdk::ModifierType::empty());
    dialog.name_input.set_text("Morning");
    dialog.name_input.emit_activate();
    wait(|| !dialog.busy.get());
    assert_eq!(
        crate::playlists::PlaylistStore::open(path.clone())
            .unwrap()
            .names(),
        ["Morning"]
    );
    dialog.handle_key(gdk::Key::Delete, gdk::ModifierType::empty());
    dialog.handle_key(gdk::Key::Escape, gdk::ModifierType::empty());
    assert_eq!(
        crate::playlists::PlaylistStore::open(path.clone())
            .unwrap()
            .names(),
        ["Morning"]
    );
    dialog.handle_key(gdk::Key::Delete, gdk::ModifierType::empty());
    dialog.handle_key(gdk::Key::Return, gdk::ModifierType::empty());
    wait(|| !dialog.busy.get());
    assert!(
        crate::playlists::PlaylistStore::open(path.clone())
            .unwrap()
            .names()
            .is_empty()
    );
    assert!(one.is_file() && two.is_file());
    dialog.window.close();
    wait(|| player.playlist_dialog.borrow().is_none());
    // Inspect and remove a missing saved track, then load the surviving song.
    let mut store = crate::playlists::PlaylistStore::open(path.clone()).unwrap();
    store.create("Mixed", Some(&one)).unwrap();
    let missing = root.path().join("missing.wav");
    store.add("Mixed", &missing).unwrap();
    key(&player, gdk::Key::b);
    wait(|| {
        player
            .playlist_dialog
            .borrow()
            .as_ref()
            .is_some_and(|d| !d.busy.get())
    });
    let dialog = player.playlist_dialog.borrow().as_ref().unwrap().clone();
    dialog.handle_key(gdk::Key::Right, gdk::ModifierType::empty());
    dialog.handle_key(gdk::Key::Down, gdk::ModifierType::empty());
    dialog.handle_key(gdk::Key::Delete, gdk::ModifierType::empty());
    wait(|| !dialog.busy.get());
    assert_eq!(
        crate::playlists::PlaylistStore::open(path.clone())
            .unwrap()
            .tracks("Mixed")
            .unwrap(),
        std::slice::from_ref(&one)
    );
    dialog.handle_key(gdk::Key::Left, gdk::ModifierType::empty());
    dialog.activate_selection();
    wait(|| player.playlist_dialog.borrow().is_none());
    wait(|| !player.state.borrow().model.loading);
    assert_eq!(
        player.state.borrow().model.tracks,
        std::slice::from_ref(&one)
    );
    store = crate::playlists::PlaylistStore::open(path.clone()).unwrap();
    store.add("Mixed", &missing).unwrap();
    key(&player, gdk::Key::b);
    wait(|| {
        player
            .playlist_dialog
            .borrow()
            .as_ref()
            .is_some_and(|d| !d.busy.get())
    });
    let dialog = player.playlist_dialog.borrow().as_ref().unwrap().clone();
    dialog.activate_selection();
    wait(|| player.playlist_dialog.borrow().is_none());
    assert!(
        player
            .state
            .borrow()
            .model
            .status
            .contains("1 missing files skipped")
    );
    // Theme refresh reads the test home and does not interrupt playback.
    let theme_path = root
        .path()
        .join("home/.local/state/omarchy/current/theme/colors.toml");
    std::fs::create_dir_all(theme_path.parent().unwrap()).unwrap();
    std::fs::write(&theme_path, "accent = '#ff0000'\n").unwrap();
    let generation = player.state.borrow().audio.snapshot().generation;
    player
        .last_theme
        .set(Instant::now() - Duration::from_secs(3));
    wait(|| player.state.borrow().theme.colors["accent"] == "#ff0000");
    assert_eq!(
        player.state.borrow().audio.snapshot().generation,
        generation
    );
    // Default size applies before mapping; the Wayland compositor owns mapped sizing.
    let small = PlayerWindow::new(
        &app,
        &Cli::for_music(None),
        root.path().join("small-data"),
        root.path().join("home"),
    )
    .unwrap();
    small.window.set_default_size(640, 420);
    small.window.present();
    wait(|| small.window.width() > 0);
    assert!(small.window.width() >= 640 && small.window.height() >= 420);
    small.close();
    small.window.close();
    assert!(player.version_label.is_visible());
    assert!(player.title.is_visible());
    player.close();
    player.window.close();
    assert!(!player.state.borrow().audio.snapshot().has_stream);
    assert!(player.state.borrow().closed);
    eprintln!(
        "GTK player: playback, view switches, search focus, keyboard transport, history/shuffle, folder browsing, drop paths, playlists, fullscreen and close passed"
    );
}
