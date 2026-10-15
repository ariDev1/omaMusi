use omamusi::player::{Action, Effect, PlayerState, TrackInfo};
use std::path::PathBuf;

fn player() -> PlayerState {
    let mut state = PlayerState::default();
    state.dispatch(Action::ReplaceQueue(
        vec![
            "/music/Album/one.flac".into(),
            "/music/Album/two.flac".into(),
            "/music/Elsewhere/three.flac".into(),
        ],
        None,
    ));
    state
}

#[test]
fn folder_navigation_scans_only_the_chosen_folder() {
    let mut state = player();
    let effects = state.dispatch(Action::ScanFolder(PathBuf::from("/music/Album")));
    assert!(effects.iter().any(|effect| matches!(effect, Effect::ScanFolder {folder,..} if folder == &PathBuf::from("/music/Album"))));
}

#[test]
fn cancel_folder_prompt_keeps_browser_and_rejects_late_folder_results() {
    use omamusi::player::FolderListing;
    let mut state = player();
    let tracks = state.tracks.clone();
    state.dispatch(Action::Browse("/music".into()));
    state.dispatch(Action::ScanFolder("/old-folder".into()));
    let old = state.library_generation;
    let effects = state.dispatch(Action::CancelFolderScan);
    assert_eq!(state.browser_folder, Some(PathBuf::from("/music")));
    assert!(effects.contains(&Effect::Invalidate("library")));
    state.dispatch(Action::FolderReady {
        generation: old,
        result: Ok(FolderListing {
            folder: "/old-folder".into(),
            tracks: vec!["/old-folder/stale.flac".into()],
            start: None,
        }),
    });
    assert_eq!(state.tracks, tracks);
    state.dispatch(Action::ScanFolder("/alias".into()));
    state.dispatch(Action::FolderReady {
        generation: state.library_generation,
        result: Ok(FolderListing {
            folder: "/canonical".into(),
            tracks: vec!["/canonical/one.flac".into(), "/canonical/two.flac".into()],
            start: Some("/canonical/two.flac".into()),
        }),
    });
    assert_eq!(state.current_folder, PathBuf::from("/canonical"));
    assert_eq!(state.library_root, PathBuf::from("/canonical"));
    assert_eq!(state.current, Some(1));
}

#[test]
fn next_wraps_manually_and_stops_automatically_at_end() {
    let mut state = player();
    assert_eq!(state.current, Some(0));
    state.dispatch(Action::Next { automatic: false });
    assert_eq!(state.current, Some(1));
    state.dispatch(Action::Next { automatic: false });
    assert_eq!(state.current, Some(2));
    let effects = state.dispatch(Action::Finished(state.generation));
    assert_eq!(state.current, Some(2));
    assert!(effects.is_empty());
    assert!(state.status.contains("finished"));
    state.dispatch(Action::Next { automatic: false });
    assert_eq!(state.current, Some(0));
}

#[test]
fn previous_restarts_after_three_seconds_otherwise_follows_history() {
    let mut state = player();
    state.dispatch(Action::Play(2));
    let effects = state.dispatch(Action::Previous {
        position: 3.1,
        has_stream: true,
    });
    assert!(effects.contains(&Effect::Seek(0.0)));
    assert_eq!(state.current, Some(2));
    state.dispatch(Action::Previous {
        position: 2.0,
        has_stream: true,
    });
    assert_eq!(state.current, Some(0));
    state.dispatch(Action::Previous {
        position: 0.0,
        has_stream: false,
    });
    assert_eq!(state.current, Some(2));
}

#[test]
fn search_navigation_wraps_and_no_match_keeps_focus() {
    let mut state = player();
    state.dispatch(Action::Search("ALBUM".into()));
    assert_eq!(state.visible_indices(), [0, 1]);
    state.dispatch(Action::Select(2));
    state.dispatch(Action::MoveSelection(1));
    assert_eq!(state.selected, Some(0));
    assert!(!state.search_focus);
    state.dispatch(Action::Search("album".into()));
    state.dispatch(Action::MoveSelection(-1));
    assert_eq!(state.selected, Some(1));
    state.dispatch(Action::Search("absent".into()));
    state.dispatch(Action::MoveSelection(1));
    assert!(state.search_focus);
    state.dispatch(Action::Search(String::new()));
    assert_eq!(state.visible_indices(), [0, 1, 2]);
}

#[test]
fn shuffle_uses_hidden_tracks_and_avoids_current() {
    let mut state = player();
    state.dispatch(Action::Search("one".into()));
    state.dispatch(Action::ToggleShuffle);
    for _ in 0..20 {
        let old = state.current;
        state.dispatch(Action::Next { automatic: false });
        assert_ne!(old, state.current);
    }
    assert!(state.history.iter().any(|&i| i != 0));
}

#[test]
fn metadata_generation_rejects_stale_tracks_and_preserves_pause_intent() {
    let mut state = player();
    let old = state.generation;
    state.dispatch(Action::Play(1));
    state.dispatch(Action::TogglePause);
    let metadata = TrackInfo {
        duration: 12.0,
        sample_rate: 96000,
        title: "Song".into(),
        artist: "Artist".into(),
        album: String::new(),
    };
    assert!(
        state
            .dispatch(Action::MetadataReady {
                generation: old,
                result: Ok(metadata.clone())
            })
            .is_empty()
    );
    assert!(state.loading);
    let effects = state.dispatch(Action::MetadataReady {
        generation: state.generation,
        result: Ok(metadata),
    });
    assert!(effects.iter().any(|e| matches!(
        e,
        Effect::AudioPlay {
            paused: true,
            sample_rate: 96000,
            ..
        }
    )));
    assert!(!state.loading);
    assert_eq!(state.title, "Song");
    assert!(state.dispatch(Action::Finished(old)).is_empty());
    assert_eq!(state.current, Some(1));
}

#[test]
fn failed_scan_preserves_queue_and_cancelled_folder_does_not_replace_it() {
    let mut state = player();
    let original = state.tracks.clone();
    let effects = state.dispatch(Action::ScanFolder("/blocked".into()));
    let token = effects
        .iter()
        .find_map(|e| {
            if let Effect::ScanFolder { generation, .. } = e {
                Some(*generation)
            } else {
                None
            }
        })
        .unwrap();
    state.dispatch(Action::FolderReady {
        generation: token,
        result: Err("permission denied".into()),
    });
    assert_eq!(state.tracks, original);
    assert!(state.status.contains("permission denied"));
    state.dispatch(Action::ScanFolder("/other".into()));
    let token = state.library_generation;
    state.dispatch(Action::CancelBrowser);
    state.dispatch(Action::FolderReady {
        generation: token,
        result: Ok(omamusi::player::FolderListing {
            folder: "/other".into(),
            tracks: vec!["/other/song.mp3".into()],
            start: None,
        }),
    });
    assert_eq!(state.tracks, original);
    assert!(state.browser_folder.is_none());
}

#[test]
fn repeated_additions_keep_request_order_and_recursion_flags() {
    let mut state = player();
    state.dispatch(Action::AddPaths(vec!["/one".into()], false));
    let old = state.library_generation;
    let effects = state.dispatch(Action::AddPaths(vec!["/two".into()], true));
    let requests = effects
        .iter()
        .find_map(|e| {
            if let Effect::Scan { requests, .. } = e {
                Some(requests)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(requests.len(), 2);
    assert!(!requests[0].recursive);
    assert!(requests[1].recursive);
    state.dispatch(Action::LibraryReady {
        generation: old,
        result: Ok(vec!["/one/a.mp3".into()]),
    });
    assert_eq!(state.tracks.len(), 3);
    state.dispatch(Action::LibraryReady {
        generation: state.library_generation,
        result: Ok(vec!["/one/a.mp3".into(), "/two/b.mp3".into()]),
    });
    assert_eq!(state.tracks.len(), 5);
    assert_eq!(state.current, Some(0));
}

#[test]
fn missing_playlist_songs_are_skipped_and_empty_playlist_keeps_playing() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("song.mp3");
    std::fs::write(&path, b"").unwrap();
    let mut state = player();
    let old = state.tracks.clone();
    state.dispatch(Action::LoadPlaylist {
        name: "Empty".into(),
        tracks: vec![PathBuf::from("/missing")],
    });
    assert_eq!(state.tracks, old);
    assert_eq!(state.current, Some(0));
    state.dispatch(Action::LoadPlaylist {
        name: "New".into(),
        tracks: vec![root.path().join("missing"), path.clone()],
    });
    assert_eq!(state.tracks, [path]);
    assert!(state.status.contains("1 missing"));
}

#[test]
fn replacing_queue_clears_search_history_and_empty_queue_stops() {
    let mut state = player();
    state.dispatch(Action::Play(2));
    state.dispatch(Action::Search("one".into()));
    let effects = state.dispatch(Action::ReplaceQueue(Vec::new(), None));
    assert!(effects.contains(&Effect::AudioStop));
    assert!(state.tracks.is_empty());
    assert!(state.current.is_none());
    assert!(state.history.is_empty());
    assert!(state.query.is_empty());
}

#[test]
fn space_replays_current_track_after_end_or_metadata_failure() {
    let mut state = player();
    state.dispatch(Action::Play(2));
    state.dispatch(Action::MetadataReady {
        generation: state.generation,
        result: Ok(TrackInfo {
            duration: 0.2,
            sample_rate: 48000,
            title: "last".into(),
            artist: String::new(),
            album: String::new(),
        }),
    });
    state.dispatch(Action::Finished(state.generation));
    let effects = state.dispatch(Action::TogglePause);
    assert!(
        effects.iter().any(
            |e| matches!(e,Effect::Probe {path,..} if path.to_string_lossy().contains("three"))
        )
    );
    state.dispatch(Action::MetadataReady {
        generation: state.generation,
        result: Err("missing media".into()),
    });
    assert!(
        state
            .dispatch(Action::TogglePause)
            .iter()
            .any(|e| matches!(e, Effect::Probe { .. }))
    );
}
