//! Desktop-independent player actions and effects.
use crate::library::{discover, matches};
use anyhow::Result;
use std::{collections::HashSet, path::PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct TrackInfo {
    pub duration: f64,
    pub sample_rate: u32,
    pub title: String,
    pub artist: String,
    pub album: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScanRequest {
    pub paths: Vec<PathBuf>,
    pub recursive: bool,
}
pub fn discover_requests(requests: &[ScanRequest]) -> Result<Vec<PathBuf>> {
    let mut tracks = Vec::new();
    let mut seen = HashSet::new();
    for request in requests {
        for path in discover(&request.paths, request.recursive)? {
            if seen.insert(path.clone()) {
                tracks.push(path);
            }
        }
    }
    Ok(tracks)
}
#[derive(Debug)]
pub enum Action {
    Play(usize),
    Select(usize),
    MoveSelection(i32),
    Search(String),
    TogglePause,
    ToggleShuffle,
    Next {
        automatic: bool,
    },
    Previous {
        position: f64,
        has_stream: bool,
    },
    Finished(u64),
    Failed {
        generation: u64,
        message: String,
    },
    MetadataReady {
        generation: u64,
        result: std::result::Result<TrackInfo, String>,
    },
    ReplaceQueue(Vec<PathBuf>, Option<PathBuf>),
    AddPaths(Vec<PathBuf>, bool),
    ScanFolder(PathBuf),
    LibraryReady {
        generation: u64,
        result: std::result::Result<Vec<PathBuf>, String>,
    },
    LoadPlaylist {
        name: String,
        tracks: Vec<PathBuf>,
    },
    Browse(PathBuf),
    CancelBrowser,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    AudioStop,
    AudioTogglePause,
    Seek(f64),
    ResetVisual,
    Probe {
        generation: u64,
        path: PathBuf,
    },
    AudioPlay {
        generation: u64,
        path: PathBuf,
        sample_rate: u32,
        paused: bool,
    },
    Scan {
        generation: u64,
        requests: Vec<ScanRequest>,
    },
    Browse {
        generation: u64,
        folder: PathBuf,
    },
    Invalidate(&'static str),
}
enum ScanKind {
    Add,
    Replace(PathBuf),
}

pub struct PlayerState {
    pub tracks: Vec<PathBuf>,
    pub current: Option<usize>,
    pub selected: Option<usize>,
    pub history: Vec<usize>,
    pub shuffle: bool,
    pub query: String,
    pub search_focus: bool,
    pub loading: bool,
    pub paused: bool,
    pub generation: u64,
    pub library_generation: u64,
    pub browser_generation: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub status: String,
    pub current_folder: PathBuf,
    pub library_root: PathBuf,
    pub browser_folder: Option<PathBuf>,
    add_requests: Vec<ScanRequest>,
    scan_kind: Option<ScanKind>,
    pause_when_ready: bool,
    random: u64,
}
impl Default for PlayerState {
    fn default() -> Self {
        let folder = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
            | 1;
        Self {
            tracks: Vec::new(),
            current: None,
            selected: None,
            history: Vec::new(),
            shuffle: false,
            query: String::new(),
            search_focus: false,
            loading: false,
            paused: false,
            generation: 0,
            library_generation: 0,
            browser_generation: 0,
            title: "omaMusi".into(),
            artist: String::new(),
            album: String::new(),
            duration: 0.0,
            status: String::new(),
            current_folder: folder.clone(),
            library_root: folder,
            browser_folder: None,
            add_requests: Vec::new(),
            scan_kind: None,
            pause_when_ready: false,
            random: seed,
        }
    }
}
impl PlayerState {
    pub fn visible_indices(&self) -> Vec<usize> {
        self.tracks
            .iter()
            .enumerate()
            .filter_map(|(i, path)| matches(path, &self.query).then_some(i))
            .collect()
    }
    fn play(&mut self, index: usize, remember: bool) -> Vec<Effect> {
        let Some(path) = self.tracks.get(index).cloned() else {
            return Vec::new();
        };
        if remember
            && let Some(old) = self.current
            && old != index
        {
            self.history.push(old);
        }
        self.current = Some(index);
        self.selected = Some(index);
        self.generation += 1;
        self.loading = true;
        self.pause_when_ready = false;
        self.paused = false;
        self.duration = 0.0;
        self.title = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        self.artist.clear();
        self.album.clear();
        self.status = "loading track…".into();
        vec![
            Effect::Invalidate("metadata"),
            Effect::Invalidate("artwork"),
            Effect::AudioStop,
            Effect::ResetVisual,
            Effect::Probe {
                generation: self.generation,
                path,
            },
        ]
    }
    fn replace(&mut self, tracks: Vec<PathBuf>, start: Option<PathBuf>) -> Vec<Effect> {
        self.library_generation += 1;
        self.browser_generation += 1;
        self.generation += 1;
        self.tracks = tracks;
        self.current = None;
        self.selected = None;
        self.history.clear();
        self.query.clear();
        self.search_focus = false;
        self.loading = false;
        self.paused = false;
        self.duration = 0.0;
        self.browser_folder = None;
        self.add_requests.clear();
        self.scan_kind = None;
        let mut effects = vec![
            Effect::Invalidate("library"),
            Effect::Invalidate("metadata"),
            Effect::Invalidate("artwork"),
            Effect::Invalidate("gallery"),
            Effect::Invalidate("browser"),
            Effect::AudioStop,
            Effect::ResetVisual,
        ];
        if !self.tracks.is_empty() {
            let index = start
                .and_then(|p| self.tracks.iter().position(|t| *t == p))
                .unwrap_or(0);
            effects.extend(self.play(index, true));
        } else {
            self.title = "No audio files in this folder.".into();
            self.status = "press c to browse folders · o to open files".into();
        }
        effects
    }
    fn next(&mut self, automatic: bool) -> Vec<Effect> {
        if self.tracks.is_empty() {
            return Vec::new();
        }
        let index = if self.shuffle {
            let candidates: Vec<_> = (0..self.tracks.len())
                .filter(|i| Some(*i) != self.current)
                .collect();
            self.random ^= self.random << 13;
            self.random ^= self.random >> 7;
            self.random ^= self.random << 17;
            if candidates.is_empty() {
                0
            } else {
                candidates[self.random as usize % candidates.len()]
            }
        } else {
            match self.current {
                Some(i) if i + 1 < self.tracks.len() => i + 1,
                None => 0,
                _ if !automatic => 0,
                _ => {
                    self.status = "playlist finished · space to replay".into();
                    return Vec::new();
                }
            }
        };
        self.play(index, true)
    }
    pub fn dispatch(&mut self, action: Action) -> Vec<Effect> {
        match action {
            Action::Play(index) => self.play(index, true),
            Action::Select(index) => {
                if index < self.tracks.len() {
                    self.selected = Some(index);
                }
                Vec::new()
            }
            Action::Search(query) => {
                self.query = query;
                self.search_focus = true;
                Vec::new()
            }
            Action::MoveSelection(delta) => {
                let visible = self.visible_indices();
                if !visible.is_empty() {
                    let index = self
                        .selected
                        .and_then(|i| visible.iter().position(|v| *v == i))
                        .map(|i| i as i64)
                        .unwrap_or(if delta > 0 { -1 } else { 0 });
                    self.selected = Some(
                        visible[(index + delta as i64).rem_euclid(visible.len() as i64) as usize],
                    );
                    self.search_focus = false;
                }
                Vec::new()
            }
            Action::ToggleShuffle => {
                self.shuffle = !self.shuffle;
                Vec::new()
            }
            Action::TogglePause => {
                if self.loading {
                    self.pause_when_ready = !self.pause_when_ready;
                    self.status = if self.pause_when_ready {
                        "loading track… · pause requested"
                    } else {
                        "loading track…"
                    }
                    .into();
                    Vec::new()
                } else if self.current.is_some() {
                    self.paused = !self.paused;
                    vec![Effect::AudioTogglePause]
                } else if !self.tracks.is_empty() {
                    self.play(0, true)
                } else {
                    Vec::new()
                }
            }
            Action::Next { automatic } => self.next(automatic),
            Action::Previous {
                position,
                has_stream,
            } => {
                if has_stream && position > 3.0 {
                    vec![Effect::Seek(0.0), Effect::ResetVisual]
                } else if let Some(index) = self.history.pop() {
                    self.play(index, false)
                } else if !self.tracks.is_empty() {
                    self.play(
                        self.current
                            .map(|i| (i + self.tracks.len() - 1) % self.tracks.len())
                            .unwrap_or(self.tracks.len() - 1),
                        false,
                    )
                } else {
                    Vec::new()
                }
            }
            Action::Finished(generation) => {
                if generation == self.generation {
                    self.next(true)
                } else {
                    Vec::new()
                }
            }
            Action::Failed {
                generation,
                message,
            } => {
                if generation == self.generation {
                    self.status = format!("error: {message}");
                    self.loading = false;
                }
                Vec::new()
            }
            Action::MetadataReady { generation, result } => {
                if generation != self.generation || !self.loading {
                    return Vec::new();
                }
                self.loading = false;
                match result {
                    Ok(info) => {
                        self.duration = info.duration;
                        self.title = info.title;
                        self.artist = info.artist;
                        self.album = info.album;
                        self.paused = self.pause_when_ready;
                        self.pause_when_ready = false;
                        self.status.clear();
                        let path = self.tracks[self.current.unwrap()].clone();
                        self.current_folder =
                            path.parent().unwrap_or(&self.current_folder).to_owned();
                        vec![
                            Effect::AudioPlay {
                                generation,
                                path,
                                sample_rate: info.sample_rate,
                                paused: self.paused,
                            },
                            Effect::ResetVisual,
                        ]
                    }
                    Err(message) => {
                        self.status = format!("error: {message}");
                        self.pause_when_ready = false;
                        Vec::new()
                    }
                }
            }
            Action::ReplaceQueue(tracks, start) => self.replace(tracks, start),
            Action::AddPaths(paths, recursive) => {
                self.library_generation += 1;
                self.scan_kind = Some(ScanKind::Add);
                self.add_requests.push(ScanRequest { paths, recursive });
                vec![Effect::Scan {
                    generation: self.library_generation,
                    requests: self.add_requests.clone(),
                }]
            }
            Action::ScanFolder(folder) => {
                self.library_generation += 1;
                self.add_requests.clear();
                self.scan_kind = Some(ScanKind::Replace(folder.clone()));
                vec![Effect::Scan {
                    generation: self.library_generation,
                    requests: vec![ScanRequest {
                        paths: vec![folder],
                        recursive: true,
                    }],
                }]
            }
            Action::LibraryReady { generation, result } => {
                if generation != self.library_generation {
                    return Vec::new();
                }
                self.add_requests.clear();
                let kind = self.scan_kind.take();
                match result {
                    Err(message) => {
                        self.status = format!("error: {message}");
                        Vec::new()
                    }
                    Ok(tracks) => match kind {
                        Some(ScanKind::Replace(folder)) => {
                            self.current_folder = folder.clone();
                            self.library_root = folder;
                            self.replace(tracks, None)
                        }
                        Some(ScanKind::Add) => {
                            let start = self.tracks.len();
                            let mut seen: HashSet<_> = self.tracks.iter().cloned().collect();
                            for path in tracks {
                                if seen.insert(path.clone()) {
                                    self.tracks.push(path);
                                }
                            }
                            if self.tracks.len() > start && self.current.is_none() && !self.loading
                            {
                                self.play(start, true)
                            } else {
                                Vec::new()
                            }
                        }
                        None => Vec::new(),
                    },
                }
            }
            Action::LoadPlaylist { name, tracks } => {
                let total = tracks.len();
                let available: Vec<_> = tracks.into_iter().filter(|p| p.is_file()).collect();
                let missing = total - available.len();
                if available.is_empty() {
                    self.status = "No available tracks in this playlist.".into();
                    Vec::new()
                } else {
                    self.library_root = available[0].parent().unwrap().to_owned();
                    let count = available.len();
                    let effects = self.replace(available, None);
                    self.status = format!(
                        "Loaded playlist: {name} · {count} tracks · {missing} missing files skipped"
                    );
                    effects
                }
            }
            Action::Browse(folder) => {
                self.browser_generation += 1;
                self.browser_folder = Some(folder.clone());
                vec![Effect::Browse {
                    generation: self.browser_generation,
                    folder,
                }]
            }
            Action::CancelBrowser => {
                self.browser_generation += 1;
                self.browser_folder = None;
                let mut effects = vec![Effect::Invalidate("browser")];
                if matches!(self.scan_kind, Some(ScanKind::Replace(_))) {
                    self.library_generation += 1;
                    self.scan_kind = None;
                    effects.push(Effect::Invalidate("library"));
                }
                effects
            }
        }
    }
}
