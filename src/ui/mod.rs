//! Keyboard-first GTK player; filesystem workers never contain GTK objects.
mod actions;
mod browser;
mod gpu_stats;
mod playlists;
use crate::visual::{VisualMode, VisualState};
use crate::{
    artwork::{Artwork, load_artwork},
    audio::{AudioEvent, AudioPlayer, probe},
    background::{BackgroundTasks, TaskResult},
    cli::Cli,
    gallery::{CoverGallery, Rect},
    library,
    player::{Action, Effect, FolderListing, PlayerState, TrackInfo, discover_requests},
    render::{Scene, canvas::RenderCanvas, cpu::CpuRenderer},
    theme::Theme,
    version,
};
use anyhow::{Result, bail};
use browser::BrowserEntry;
use gtk::{gdk, gio, glib, prelude::*};
use playlists::PlaylistDialog;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

const MAX_TRACK_ROWS: usize = 500;

enum Work {
    Metadata(TrackInfo),
    Tracks(Vec<PathBuf>),
    Folder(FolderListing),
    Browser(Vec<BrowserEntry>),
    Artwork(Option<Artwork>),
    Theme(Theme),
}
enum JobKind {
    Metadata(u64, PathBuf),
    Library(u64),
    Folder(u64),
    Browser(u64, PathBuf),
    Artwork(u64, PathBuf),
    Gallery(u64, PathBuf),
    Theme,
}
struct Job {
    token: u64,
    kind: JobKind,
}
struct AppState {
    model: PlayerState,
    audio: AudioPlayer,
    visual: VisualState,
    gallery: CoverGallery,
    cover: Option<Arc<Artwork>>,
    background: BackgroundTasks<Work>,
    jobs: HashMap<String, Job>,
    metadata: HashMap<PathBuf, TrackInfo>,
    browser_entries: Vec<BrowserEntry>,
    browser_selected: usize,
    browser_success: Option<PathBuf>,
    artwork_path: Option<PathBuf>,
    gallery_queue: VecDeque<PathBuf>,
    gallery_seen: HashSet<PathBuf>,
    gallery_generation: u64,
    gallery_bounds: Rect,
    folder_start: Option<PathBuf>,
    theme: Theme,
    closed: bool,
    pending_notice: Option<String>,
}
impl AppState {
    fn submit(
        &mut self,
        category: &str,
        kind: JobKind,
        operation: impl FnOnce() -> Result<Work> + Send + 'static,
    ) {
        if let Some(token) = self.background.submit(category, operation) {
            self.jobs.insert(category.into(), Job { token, kind });
        }
    }
}
pub struct PlayerWindow {
    pub window: gtk::ApplicationWindow,
    state: Rc<RefCell<AppState>>,
    overlay: gtk::Overlay,
    canvas: RenderCanvas,
    logo: gtk::Label,
    visual_notice: gtk::Label,
    gpu_readout: gtk::Label,
    gpu_frames: Rc<Cell<u64>>,
    gpu_sample: Cell<(Instant, u64)>,
    gpu_poll: Cell<Instant>,
    gpu_stats: RefCell<gpu_stats::Telemetry>,
    gpu_active: Cell<bool>,
    folder_label: gtk::Label,
    folder_prompt: gtk::Box,
    folder_input: gtk::Entry,
    panel: gtk::Box,
    panel_width: Cell<i32>,
    count: gtk::Label,
    search: gtk::Entry,
    listing: gtk::ListBox,
    folder_list: gtk::ListBox,
    scroller: gtk::ScrolledWindow,
    title: gtk::Label,
    subtitle: gtk::Label,
    status: gtk::Label,
    transport: gtk::Label,
    volume_notice: gtk::Label,
    hint: gtk::Label,
    footer: gtk::Box,
    version_label: gtk::Label,
    track_rows: RefCell<Vec<gtk::ListBoxRow>>,
    displayed_track_indices: RefCell<Vec<usize>>,
    shown_tracks_revision: Cell<u64>,
    shown_current: Cell<Option<usize>>,
    shown_query: RefCell<String>,
    visible_tracks: Cell<usize>,
    shown_browser: RefCell<Vec<String>>,
    playlist_dialog: RefCell<Option<Rc<PlaylistDialog>>>,
    timer: RefCell<Option<glib::SourceId>>,
    updating: Cell<bool>,
    panel_hidden: Cell<bool>,
    last_interaction: Cell<Instant>,
    last_tick: Cell<Instant>,
    last_theme: Cell<Instant>,
    notice_until: Cell<Instant>,
    volume_until: Cell<Instant>,
    css: gtk::CssProvider,
    data_root: PathBuf,
    home: PathBuf,
}
fn label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_use_markup(false);
    if !class.is_empty() {
        label.add_css_class(class);
    }
    label
}
fn entry(placeholder: &str) -> gtk::Entry {
    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some(placeholder));
    entry.add_css_class("omamusi-entry");
    entry
}
fn make_listing() -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(true);
    list.add_css_class("omamusi-list");
    list
}
/// Sidebar width for the track list: a responsive share of the stage that
/// leaves room for the visualization, instead of one fixed column.
fn list_width(stage_width: f64) -> i32 {
    ((stage_width - 60.0).max(0.0) * 0.70) as i32
}
fn clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            (seconds / 60) % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

impl PlayerWindow {
    pub fn new(
        app: &gtk::Application,
        cli: &Cli,
        data_root: PathBuf,
        home: PathBuf,
    ) -> Result<Rc<Self>> {
        let theme = Theme::default();
        let mut visual = VisualState::new();
        visual.mode = cli.view;
        visual.set_palette(theme.palette());
        let state = Rc::new(RefCell::new(AppState {
            model: PlayerState::default(),
            audio: AudioPlayer::new()?,
            visual,
            gallery: CoverGallery::new(),
            cover: None,
            background: BackgroundTasks::new(),
            jobs: HashMap::new(),
            metadata: HashMap::new(),
            browser_entries: Vec::new(),
            browser_selected: 0,
            browser_success: None,
            artwork_path: None,
            gallery_queue: VecDeque::new(),
            gallery_seen: HashSet::new(),
            gallery_generation: 0,
            gallery_bounds: Rect {
                x: 30.0,
                y: 64.0,
                width: 600.0,
                height: 400.0,
            },
            folder_start: None,
            theme: theme.clone(),
            closed: false,
            pending_notice: None,
        }));
        let cpu = state.clone();
        let gpu = state.clone();
        let gpu_frames = Rc::new(Cell::new(0u64));
        let frames = gpu_frames.clone();
        let canvas = RenderCanvas::new(
            move |ctx, w, h| {
                let mut state = cpu.borrow_mut();
                let state = &mut *state;
                if state.closed {
                    return Ok(());
                }
                let mut scene = Scene {
                    frame: state.visual.frame(),
                    wave: &state.visual.waveform,
                    cover: state.cover.as_deref(),
                    gallery: &mut state.gallery,
                    gallery_bounds: state.gallery_bounds,
                };
                CpuRenderer::draw(ctx, &mut scene, w as f64, h as f64)
            },
            move |backend, w, h, ratio, dt| {
                let mut state = gpu.borrow_mut();
                let state = &mut *state;
                if state.closed {
                    return Ok(());
                }
                let mut scene = Scene {
                    frame: state.visual.frame(),
                    wave: &state.visual.waveform,
                    cover: state.cover.as_deref(),
                    gallery: &mut state.gallery,
                    gallery_bounds: state.gallery_bounds,
                };
                backend.draw(&mut scene, w, h, ratio, dt)?;
                frames.set(frames.get().wrapping_add(1));
                Ok(())
            },
        );
        canvas.widget.set_focusable(true);
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("omaMusi")
            .default_width(1060)
            .default_height(680)
            .build();
        window.set_size_request(640, 420);
        window.add_css_class("omamusi");
        window.set_icon_name(Some("io.github.aridev1.omamusi"));
        let gnome = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase()
            .contains("gnome");
        window.set_decorated(gnome);
        if gnome {
            let header = gtk::HeaderBar::new();
            header.add_css_class("omamusi-header");
            window.set_titlebar(Some(&header));
        }
        let overlay = gtk::Overlay::new();
        overlay.add_css_class("omamusi-stage");
        overlay.set_child(Some(&canvas.widget));
        window.set_child(Some(&overlay));
        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.set_margin_start(30);
        root.set_margin_end(30);
        root.set_margin_top(26);
        root.set_margin_bottom(24);
        root.set_hexpand(true);
        root.set_vexpand(true);
        overlay.add_overlay(&root);
        // GtkOverlay's default unconstrained preferred-size query can inflate
        // wrapped footer height. Allocate the UI at the actual stage dimensions.
        overlay.connect_get_child_position(|overlay, _| {
            Some(gdk::Rectangle::new(0, 0, overlay.width(), overlay.height()))
        });
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let logo = label("omaMusi", "omamusi-accent");
        header.append(&logo);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);
        let visual_notice = label("", "omamusi-hint");
        header.append(&visual_notice);
        root.append(&header);
        let folder_label = label("", "omamusi-muted");
        folder_label.set_wrap(true);
        folder_label.set_visible(false);
        root.append(&folder_label);
        let folder_prompt = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        folder_prompt.append(&label("cd", "omamusi-accent"));
        let folder_input = entry("folder path · enter to change · esc to cancel");
        folder_input.set_hexpand(true);
        folder_prompt.append(&folder_input);
        folder_prompt.set_visible(false);
        root.append(&folder_prompt);
        let middle = gtk::Box::new(gtk::Orientation::Vertical, 0);
        middle.set_vexpand(true);
        middle.set_margin_top(10);
        let gpu_readout = label("", "omamusi-gpu-readout");
        gpu_readout.set_halign(gtk::Align::End);
        gpu_readout.set_valign(gtk::Align::Start);
        gpu_readout.set_justify(gtk::Justification::Right);
        gpu_readout.set_max_width_chars(76);
        gpu_readout.set_wrap(true);
        gpu_readout.set_visible(false);
        middle.append(&gpu_readout);
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
        panel.set_halign(gtk::Align::Start);
        panel.set_valign(gtk::Align::Start);
        panel.set_size_request(320, -1);
        let count = label("", "omamusi-muted");
        count.set_visible(false);
        panel.append(&count);
        let search = entry("/ search files and folders");
        search.set_visible(false);
        panel.append(&search);
        let listing = make_listing();
        let folder_list = make_listing();
        folder_list.set_visible(false);
        let lists = gtk::Box::new(gtk::Orientation::Vertical, 0);
        lists.append(&listing);
        lists.append(&folder_list);
        let scroller = gtk::ScrolledWindow::new();
        scroller.add_css_class("omamusi-list");
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::External);
        scroller.set_child(Some(&lists));
        scroller.set_size_request(-1, 100);
        panel.append(&scroller);
        middle.append(&panel);
        root.append(&middle);
        let title = label("Drop some music here.", "omamusi-title");
        title.set_wrap(true);
        let subtitle = label("", "omamusi-muted");
        subtitle.set_wrap(true);
        let status = label("", "omamusi-muted");
        status.set_wrap(true);
        root.append(&title);
        root.append(&subtitle);
        root.append(&status);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let transport = label("", "omamusi-accent");
        transport.set_hexpand(true);
        let volume_notice = label("", "omamusi-muted");
        row.append(&transport);
        row.append(&volume_notice);
        root.append(&row);
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 20);
        let hint = label("", "omamusi-hint");
        hint.set_wrap(true);
        hint.set_hexpand(true);
        let version_label = label(
            &format!("v{} · {}", version::VERSION, version::REVISION),
            "omamusi-hint",
        );
        version_label.set_valign(gtk::Align::End);
        footer.append(&hint);
        footer.append(&version_label);
        root.append(&footer);
        let css = gtk::CssProvider::new();
        css.load_from_data(&theme.css());
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        let now = Instant::now();
        let player = Rc::new(Self {
            window,
            state,
            overlay,
            canvas,
            logo,
            visual_notice,
            gpu_readout,
            gpu_frames,
            gpu_sample: Cell::new((now, 0)),
            gpu_poll: Cell::new(now),
            gpu_stats: RefCell::new(gpu_stats::Telemetry::new()),
            gpu_active: Cell::new(false),
            folder_label,
            folder_prompt,
            folder_input,
            panel,
            panel_width: Cell::new(0),
            count,
            search,
            listing,
            folder_list,
            scroller,
            title,
            subtitle,
            status,
            transport,
            volume_notice,
            hint,
            footer,
            version_label,
            track_rows: RefCell::new(Vec::new()),
            displayed_track_indices: RefCell::new(Vec::new()),
            shown_tracks_revision: Cell::new(u64::MAX),
            shown_current: Cell::new(None),
            shown_query: RefCell::new(String::new()),
            visible_tracks: Cell::new(0),
            shown_browser: RefCell::new(Vec::new()),
            playlist_dialog: RefCell::new(None),
            timer: RefCell::new(None),
            updating: Cell::new(false),
            panel_hidden: Cell::new(false),
            last_interaction: Cell::new(now),
            last_tick: Cell::new(now),
            last_theme: Cell::new(now),
            notice_until: Cell::new(now),
            volume_until: Cell::new(now),
            css,
            data_root,
            home,
        });
        player.connect();
        player.refresh();
        let weak = Rc::downgrade(&player);
        let timer = glib::timeout_add_local(Duration::from_millis(16), move || {
            if let Some(player) = weak.upgrade() {
                if player.state.borrow().closed {
                    return glib::ControlFlow::Break;
                }
                player.tick();
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
        player.timer.replace(Some(timer));
        let home = player.home.clone();
        player
            .state
            .borrow_mut()
            .submit("theme", JobKind::Theme, move || {
                Ok(Work::Theme(Theme::load_from(&home)))
            });
        let paths = if cli.empty_start {
            Vec::new()
        } else {
            cli.startup_paths(&std::env::current_dir()?)
        };
        if !paths.is_empty() {
            player.dispatch(Action::AddPaths(paths, cli.scan_recursive()));
        }
        Ok(player)
    }
    fn connect(self: &Rc<Self>) {
        let key = gtk::EventControllerKey::new();
        key.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        key.connect_key_pressed(move |_, key, _, modifiers| {
            weak.upgrade()
                .map(|p| p.handle_key(key, modifiers))
                .unwrap_or(glib::Propagation::Proceed)
        });
        self.window.add_controller(key);
        let weak = Rc::downgrade(self);
        self.window.connect_close_request(move |_| {
            if let Some(player) = weak.upgrade() {
                player.close();
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        self.search.connect_changed(move |entry| {
            if let Some(player) = weak.upgrade()
                && !player.updating.get()
            {
                player.dispatch(Action::Search(entry.text().to_string()));
            }
        });
        let weak = Rc::downgrade(self);
        self.search.connect_activate(move |_| {
            if let Some(player) = weak.upgrade() {
                let selected = player.selected_visible();
                if let Some(index) = selected {
                    player.dispatch(Action::Play(index));
                    player.focus_player();
                }
            }
        });
        let weak = Rc::downgrade(self);
        self.folder_input.connect_activate(move |entry| {
            if let Some(player) = weak.upgrade() {
                let text = entry.text();
                if text.trim().is_empty() {
                    player.notice("folder: enter a path · esc to cancel");
                    return;
                }
                let path = PathBuf::from(text.trim());
                let path = if path.is_absolute() || path.starts_with("~") {
                    path
                } else {
                    let state = player.state.borrow();
                    state
                        .model
                        .browser_folder
                        .as_ref()
                        .unwrap_or(&state.model.current_folder)
                        .join(path)
                };
                player.state.borrow_mut().folder_start = None;
                player.dispatch(Action::ScanFolder(path));
            }
        });
        let weak = Rc::downgrade(self);
        self.listing.connect_row_selected(move |_, row| {
            if let Some(player) = weak.upgrade()
                && !player.updating.get()
                && let Some(row) = row
                && let Some(index) = player
                    .displayed_track_indices
                    .borrow()
                    .get(row.index() as usize)
                    .copied()
            {
                player.dispatch(Action::Select(index));
            }
        });
        let weak = Rc::downgrade(self);
        self.listing.connect_row_activated(move |_, row| {
            if let Some(player) = weak.upgrade()
                && let Some(index) = player
                    .displayed_track_indices
                    .borrow()
                    .get(row.index() as usize)
                    .copied()
            {
                player.dispatch(Action::Play(index));
            }
        });
        let weak = Rc::downgrade(self);
        self.folder_list.connect_row_selected(move |_, row| {
            if let Some(player) = weak.upgrade()
                && !player.updating.get()
                && let Some(row) = row
            {
                player.state.borrow_mut().browser_selected = row.index() as usize;
            }
        });
        let weak = Rc::downgrade(self);
        self.folder_list.connect_row_activated(move |_, _| {
            if let Some(player) = weak.upgrade() {
                player.activate_folder_item();
            }
        });
        let motion = gtk::EventControllerMotion::new();
        let weak = Rc::downgrade(self);
        let last_pointer = Cell::new(None);
        motion.connect_motion(move |_, x, y| {
            if let Some(player) = weak.upgrade() {
                // Reallocation can replay motion at the stationary pointer.
                // Only a changed position counts as activity; otherwise hiding
                // the footer would immediately bring it back.
                if last_pointer.replace(Some((x, y))) != Some((x, y)) {
                    player.interact();
                }
                player.gallery_hover(Some((x, y)));
            }
        });
        let weak = Rc::downgrade(self);
        motion.connect_leave(move |_| {
            if let Some(player) = weak.upgrade() {
                player.gallery_hover(None);
            }
        });
        self.overlay.add_controller(motion);
        let click = gtk::GestureClick::new();
        click.set_button(1);
        let weak = Rc::downgrade(self);
        click.connect_pressed(move |_, _, x, y| {
            if let Some(player) = weak.upgrade() {
                player.interact();
                let path = {
                    let mut state = player.state.borrow_mut();
                    let state = &mut *state;
                    if state.visual.mode != VisualMode::CoverGallery {
                        None
                    } else {
                        let current = state
                            .model
                            .current
                            .and_then(|i| state.model.tracks.get(i))
                            .cloned();
                        state
                            .gallery
                            .hit((x, y), state.gallery_bounds)
                            .and_then(|cover| cover.pick(current.as_deref()))
                    }
                };
                if let Some(path) = path {
                    let index = player
                        .state
                        .borrow()
                        .model
                        .tracks
                        .iter()
                        .position(|p| *p == path);
                    if let Some(index) = index {
                        player.dispatch(Action::Play(index));
                    }
                }
            }
        });
        self.overlay.add_controller(click);
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        let weak = Rc::downgrade(self);
        scroll.connect_scroll(move |_, _, _| {
            if let Some(player) = weak.upgrade() {
                player.interact();
            }
            glib::Propagation::Proceed
        });
        self.overlay.add_controller(scroll);
        let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        let weak = Rc::downgrade(self);
        drop.connect_drop(move |_, value, _, _| {
            if let Some(player) = weak.upgrade()
                && let Ok(files) = value.get::<gdk::FileList>()
            {
                let paths = files
                    .files()
                    .into_iter()
                    .filter_map(|file| file.path())
                    .collect();
                player.drop_paths(paths);
                true
            } else {
                false
            }
        });
        self.overlay.add_controller(drop);
    }
    pub fn drop_paths(&self, paths: Vec<PathBuf>) {
        self.dispatch(Action::AddPaths(paths, false));
    }
    fn interact(&self) {
        self.last_interaction.set(Instant::now());
        self.panel.set_can_target(true);
    }
    fn focus_player(&self) {
        gtk::prelude::GtkWindowExt::set_focus(&self.window, Some(&self.canvas.widget));
        self.state.borrow_mut().model.search_focus = false;
    }
    pub fn activate(&self) {
        self.window.unminimize();
        self.window.present();
        if let Some(dialog) = self.playlist_dialog.borrow().as_ref() {
            dialog.window.present();
        }
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
            let title = self
                .playlist_dialog
                .borrow()
                .as_ref()
                .and_then(|dialog| dialog.window.title())
                .or_else(|| self.window.title())
                .unwrap_or_default()
                .to_string();
            let pid = std::process::id();
            std::thread::spawn(move || {
                let _ =
                    crate::desktop::focus_hyprland(std::ffi::OsStr::new("hyprctl"), pid, &title);
            });
        }
        self.interact();
    }
    pub fn dispatch(&self, action: Action) {
        self.interact();
        let effects = self.state.borrow_mut().model.dispatch(action);
        self.effects(effects);
        self.ensure_artwork();
        self.ensure_gallery();
        self.refresh();
    }
    fn effects(&self, effects: Vec<Effect>) {
        let mut state = self.state.borrow_mut();
        for effect in effects {
            match effect {
                Effect::AudioStop => {
                    state.audio.stop();
                    state.pending_notice = None;
                }
                Effect::AudioTogglePause => state.audio.toggle_pause(),
                Effect::Seek(seconds) => {
                    if let Err(error) = state.audio.seek(seconds) {
                        state.audio.stop();
                        let generation = state.model.generation;
                        state.model.dispatch(Action::Failed {
                            generation,
                            message: format!("{error:#}"),
                        });
                    }
                }
                Effect::ResetVisual => state.visual.reset(),
                Effect::Invalidate(category) => {
                    state.background.invalidate(category);
                    state.jobs.remove(category);
                    if category == "artwork" {
                        state.cover = None;
                        state.artwork_path = None;
                    }
                    if category == "gallery" {
                        state.gallery.clear();
                        state.gallery_seen.clear();
                        state.gallery_queue.clear();
                        state.gallery_generation += 1;
                    }
                    if category == "browser" {
                        state.browser_entries.clear();
                        state.browser_success = None;
                    }
                }
                Effect::Probe { generation, path } => {
                    let cached = state.metadata.get(&path).cloned();
                    let source = path.clone();
                    state.submit("metadata", JobKind::Metadata(generation, path), move || {
                        if !std::fs::metadata(&source)?.is_file() {
                            bail!("Audio file is unavailable: {}", source.display());
                        }
                        Ok(Work::Metadata(if let Some(cached) = cached {
                            cached
                        } else {
                            probe(&source)?
                        }))
                    });
                }
                Effect::AudioPlay {
                    generation,
                    path,
                    sample_rate,
                    paused,
                } => {
                    match state
                        .audio
                        .play(&path, 0.0, sample_rate, generation, paused)
                    {
                        Ok(()) => {
                            let duration = state.model.duration;
                            state.audio.set_duration(duration);
                            if let Some(notice) = state.pending_notice.take() {
                                state.model.status = notice;
                            }
                        }
                        Err(error) => {
                            state.model.dispatch(Action::Failed {
                                generation,
                                message: format!("{error:#}"),
                            });
                        }
                    }
                }
                Effect::Scan {
                    generation,
                    requests,
                } => state.submit("library", JobKind::Library(generation), move || {
                    Ok(Work::Tracks(discover_requests(&requests)?))
                }),
                Effect::ScanFolder { generation, folder } => {
                    let start = state.folder_start.take();
                    state.submit("library", JobKind::Folder(generation), move || {
                        let folder = library::absolute(&folder)?;
                        if !std::fs::metadata(&folder)?.is_dir() {
                            bail!("Not a folder: {}", folder.display());
                        }
                        let tracks = library::discover(std::slice::from_ref(&folder), false)?;
                        let start = start.map(|path| library::absolute(&path)).transpose()?;
                        Ok(Work::Folder(FolderListing {
                            folder,
                            tracks,
                            start,
                        }))
                    });
                }
                Effect::Browse { generation, folder } => {
                    let source = folder.clone();
                    state.submit("browser", JobKind::Browser(generation, folder), move || {
                        Ok(Work::Browser(browser::entries(&source)?))
                    });
                }
            }
        }
    }
    fn poll(&self) {
        let results = self.state.borrow().background.poll();
        for result in results {
            self.apply_work_result(result);
        }
        self.ensure_artwork();
        self.ensure_gallery();
    }
    fn apply_work_result(&self, result: TaskResult<Work>) {
        let mut folder_completed = false;
        let action = {
            let mut state = self.state.borrow_mut();
            if state
                .jobs
                .get(&result.category)
                .is_none_or(|job| job.token != result.generation)
            {
                return;
            }
            let job = state.jobs.remove(&result.category).unwrap();
            match job.kind {
                JobKind::Metadata(generation, path) => {
                    let result = match result.result {
                        Ok(Work::Metadata(info)) => {
                            state.metadata.insert(path, info.clone());
                            Ok(info)
                        }
                        Ok(_) => Err("Invalid metadata result".into()),
                        Err(e) => Err(format!("{e:#}")),
                    };
                    Some(Action::MetadataReady { generation, result })
                }
                JobKind::Library(generation) => {
                    let result = match result.result {
                        Ok(Work::Tracks(tracks)) => Ok(tracks),
                        Ok(_) => Err("Invalid library result".into()),
                        Err(e) => Err(format!("{e:#}")),
                    };
                    Some(Action::LibraryReady { generation, result })
                }
                JobKind::Folder(generation) => {
                    let result = match result.result {
                        Ok(Work::Folder(listing)) => Ok(listing),
                        Ok(_) => Err("Invalid folder result".into()),
                        Err(e) => Err(format!("{e:#}")),
                    };
                    folder_completed =
                        generation == state.model.library_generation && result.is_ok();
                    Some(Action::FolderReady { generation, result })
                }
                JobKind::Browser(generation, folder) => {
                    if generation == state.model.browser_generation {
                        match result.result {
                            Ok(Work::Browser(entries)) => {
                                state.browser_entries = entries;
                                state.browser_selected = 0;
                                state.browser_success = Some(folder.clone());
                                state.model.browser_folder = Some(folder);
                            }
                            Err(error) => {
                                state.model.browser_folder = state.browser_success.clone();
                                state.model.status = format!("error: {error:#}");
                            }
                            _ => {}
                        }
                    }
                    None
                }
                JobKind::Artwork(generation, path) => {
                    if generation == state.model.generation
                        && state.artwork_path.as_ref() == Some(&path)
                        && let Ok(Work::Artwork(art)) = result.result
                    {
                        state.cover = art.map(Arc::new);
                    }
                    None
                }
                JobKind::Gallery(generation, path) => {
                    if generation == state.gallery_generation
                        && let Ok(Work::Artwork(Some(art))) = result.result
                    {
                        state.gallery.add(path, art);
                    }
                    None
                }
                JobKind::Theme => {
                    if let Ok(Work::Theme(theme)) = result.result
                        && theme != state.theme
                    {
                        state.visual.set_palette(theme.palette());
                        self.css.load_from_data(&theme.css());
                        state.theme = theme;
                    }
                    None
                }
            }
        };
        if let Some(action) = action {
            if folder_completed {
                self.focus_player();
                self.folder_prompt.set_visible(false);
            }
            self.dispatch(action);
        }
    }
    fn ensure_artwork(&self) {
        let mut state = self.state.borrow_mut();
        if state.visual.mode != VisualMode::CoverArt || state.closed {
            return;
        }
        let Some(path) = state
            .model
            .current
            .and_then(|i| state.model.tracks.get(i))
            .cloned()
        else {
            return;
        };
        if state.artwork_path.as_ref() == Some(&path) {
            return;
        }
        state.artwork_path = Some(path.clone());
        let source = path.clone();
        let generation = state.model.generation;
        state.submit("artwork", JobKind::Artwork(generation, path), move || {
            Ok(Work::Artwork(load_artwork(&source)?))
        });
    }
    fn ensure_gallery(&self) {
        let mut state = self.state.borrow_mut();
        if state.visual.mode != VisualMode::CoverGallery || state.closed {
            return;
        }
        let mut new: Vec<_> = state
            .model
            .tracks
            .iter()
            .filter(|p| !state.gallery_seen.contains(*p))
            .cloned()
            .collect();
        fastrand::shuffle(&mut new);
        for path in new {
            state.gallery_seen.insert(path.clone());
            state.gallery_queue.push_back(path);
        }
        state.gallery.scanning =
            state.jobs.contains_key("gallery") || !state.gallery_queue.is_empty();
        if state.jobs.contains_key("gallery") {
            return;
        }
        if let Some(path) = state.gallery_queue.pop_front() {
            let source = path.clone();
            let generation = state.gallery_generation;
            state.submit("gallery", JobKind::Gallery(generation, path), move || {
                Ok(Work::Artwork(load_artwork(&source)?))
            });
        }
    }
    fn gallery_hover(&self, point: Option<(f64, f64)>) {
        let (hovered, tooltip) = {
            let mut state = self.state.borrow_mut();
            if state.visual.mode != VisualMode::CoverGallery {
                return;
            }
            let bounds = state.gallery_bounds;
            state.gallery.hover(point, bounds);
            let hovered = state.gallery.hovered;
            let tooltip = hovered
                .and_then(|index| state.gallery.covers.get(&state.gallery.tiles[index].key))
                .map(|cover| {
                    format!(
                        "{} tracks\n{}",
                        cover.tracks.len(),
                        cover
                            .tracks
                            .iter()
                            .take(3)
                            .map(|path| path.file_stem().unwrap_or_default().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
            (hovered, tooltip)
        };
        self.overlay
            .set_cursor_from_name(hovered.map(|_| "pointer"));
        self.overlay.set_tooltip_text(tooltip.as_deref());
        self.canvas.queue_draw();
    }
    fn tick(&self) {
        self.poll();
        let now = Instant::now();
        let elapsed = now
            .duration_since(self.last_tick.replace(now))
            .as_secs_f64();
        let events = self.state.borrow_mut().audio.poll_events();
        for event in events {
            self.state.borrow_mut().audio.stop();
            match event {
                AudioEvent::Finished(generation) => self.dispatch(Action::Finished(generation)),
                AudioEvent::Failed(generation, message) => self.dispatch(Action::Failed {
                    generation,
                    message,
                }),
            }
        }
        {
            let mut state = self.state.borrow_mut();
            let pcm = state.audio.samples();
            let snapshot = state.audio.snapshot();
            state.visual.feed(&pcm);
            let active = snapshot.has_stream && !snapshot.paused && state.model.has_stream;
            state.visual.tick(elapsed, active);
            if state.visual.mode == VisualMode::CoverGallery {
                state.gallery.tick(elapsed);
            }
        }
        let gpu_lab = self.state.borrow().visual.mode == VisualMode::GpuLab;
        self.gpu_readout.set_visible(gpu_lab);
        if gpu_lab && !self.gpu_active.replace(true) {
            self.gpu_sample.set((now, self.gpu_frames.get()));
            self.gpu_poll.set(now);
            self.gpu_stats.borrow_mut().reset();
            self.gpu_readout.set_text("GPU Lab · measuring…");
        } else if !gpu_lab {
            self.gpu_active.set(false);
        }
        if gpu_lab && now.duration_since(self.gpu_poll.get()) >= Duration::from_millis(200) {
            self.gpu_poll.set(now);
            self.gpu_stats.borrow_mut().sample();
        }
        if gpu_lab && now.duration_since(self.gpu_sample.get().0) >= Duration::from_secs(1) {
            let (previous, count) = self.gpu_sample.replace((now, self.gpu_frames.get()));
            let backend = self.canvas.description();
            let renderer = backend
                .strip_prefix("gpu (")
                .and_then(|name| name.split(" (").next())
                .and_then(|name| name.split(" / ").next())
                .unwrap_or(if self.canvas.is_cpu() {
                    "Software renderer"
                } else {
                    "Initializing GPU"
                });
            let rate = if self.canvas.is_cpu() {
                "software fallback".into()
            } else {
                format!(
                    "{:.0} render calls/s",
                    (self.gpu_frames.get() - count) as f64
                        / now.duration_since(previous).as_secs_f64()
                )
            };
            self.gpu_readout.set_text(&format!(
                "GPU LAB · {renderer} · {rate}\n{}",
                self.gpu_stats.borrow_mut().report()
            ));
        }
        if now.duration_since(self.last_theme.get()) >= Duration::from_secs(2) {
            self.last_theme.set(now);
            let home = self.home.clone();
            self.state
                .borrow_mut()
                .submit("theme", JobKind::Theme, move || {
                    Ok(Work::Theme(Theme::load_from(&home)))
                });
        }
        self.refresh();
        self.canvas.queue_draw();
    }
    fn refresh(&self) {
        self.updating.set(true);
        let stage_width = self.overlay.width() as f64;
        {
            let state = self.state.borrow();
            let model = &state.model;
            let snapshot = state.audio.snapshot();
            let tracks_changed = self.shown_tracks_revision.get() != model.tracks_revision;
            let query_changed = *self.shown_query.borrow() != model.query;
            if tracks_changed || query_changed {
                while let Some(child) = self.listing.first_child() {
                    self.listing.remove(&child);
                }
                let mut rows = Vec::new();
                let mut indices = Vec::new();
                for (index, path) in model
                    .tracks
                    .iter()
                    .enumerate()
                    .filter(|(_, path)| library::matches(path, &model.query))
                    .take(MAX_TRACK_ROWS)
                {
                    let relative = path
                        .strip_prefix(&model.library_root)
                        .unwrap_or(path)
                        .with_extension("");
                    let row = gtk::ListBoxRow::new();
                    let text = label(
                        &format!(
                            "{}{}",
                            if model.current == Some(index) {
                                "› "
                            } else {
                                "  "
                            },
                            relative.display()
                        ),
                        "",
                    );
                    text.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    row.set_child(Some(&text));
                    row.set_tooltip_text(Some(&path.to_string_lossy()));
                    self.listing.append(&row);
                    rows.push(row);
                    indices.push(index);
                }
                self.track_rows.replace(rows);
                self.displayed_track_indices.replace(indices);
                self.shown_tracks_revision.set(model.tracks_revision);
                self.shown_query.replace(model.query.clone());
                self.visible_tracks.set(
                    model
                        .tracks
                        .iter()
                        .filter(|path| library::matches(path, &model.query))
                        .count(),
                );
            }
            if tracks_changed || query_changed || self.shown_current.get() != model.current {
                if !tracks_changed && !query_changed {
                    let rows = self.track_rows.borrow();
                    for index in [self.shown_current.get(), model.current]
                        .into_iter()
                        .flatten()
                    {
                        let Some(position) = self
                            .displayed_track_indices
                            .borrow()
                            .iter()
                            .position(|displayed| *displayed == index)
                        else {
                            continue;
                        };
                        let (Some(path), Some(row)) = (model.tracks.get(index), rows.get(position))
                        else {
                            continue;
                        };
                        let relative = path
                            .strip_prefix(&model.library_root)
                            .unwrap_or(path)
                            .with_extension("");
                        if let Some(text) = row
                            .child()
                            .and_then(|child| child.downcast::<gtk::Label>().ok())
                        {
                            text.set_text(&format!(
                                "{}{}",
                                if model.current == Some(index) {
                                    "› "
                                } else {
                                    "  "
                                },
                                relative.display()
                            ));
                        }
                    }
                }
                self.shown_current.set(model.current);
            }
            if let Some(index) = model.selected
                && let Some(row) = self.track_rows.borrow().get(index)
            {
                self.listing.select_row(Some(row));
            }
            let browsing = model.browser_folder.is_some();
            self.listing.set_visible(!browsing);
            self.folder_list.set_visible(browsing);
            self.count.set_visible(browsing);
            self.folder_label
                .set_visible(browsing && !self.folder_prompt.is_visible());
            if let Some(folder) = &model.browser_folder {
                let labels: Vec<_> = state
                    .browser_entries
                    .iter()
                    .map(|entry| entry.label(folder))
                    .collect();
                if *self.shown_browser.borrow() != labels {
                    while let Some(child) = self.folder_list.first_child() {
                        self.folder_list.remove(&child);
                    }
                    for text in &labels {
                        let row = gtk::ListBoxRow::new();
                        row.set_child(Some(&label(text, "")));
                        self.folder_list.append(&row);
                    }
                    self.shown_browser.replace(labels);
                }
                if let Some(row) = self.folder_list.row_at_index(state.browser_selected as i32) {
                    self.folder_list.select_row(Some(&row));
                }
                self.folder_label.set_text(&folder.to_string_lossy());
                self.count
                    .set_text(&format!("browse · {} entries", state.browser_entries.len()));
            }
            if !browsing {
                self.count
                    .set_visible(self.visible_tracks.get() > MAX_TRACK_ROWS);
                if self.visible_tracks.get() > MAX_TRACK_ROWS {
                    self.count.set_text(&format!(
                        "showing first {MAX_TRACK_ROWS} of {} matches · use / to search",
                        self.visible_tracks.get()
                    ));
                }
            }
            if self.search.text().as_str() != model.query {
                self.search.set_text(&model.query);
            }
            if !model.query.is_empty() {
                self.search.set_visible(true);
            }
            let visible = if browsing {
                state.browser_entries.len()
            } else {
                self.visible_tracks.get()
            };
            let desired_list_height = (visible as i32 * 26 + 2).max(28);
            // The list fades out on idle, so let it claim a generous share of
            // the stage while leaving room for the visualization. This tracks
            // the real window width instead of pinning a fixed column.
            let target = list_width(stage_width);
            if self.panel_width.get() != target {
                self.panel_width.set(target);
                self.panel.set_size_request(target, -1);
            }
            self.panel.set_visible(
                !self.panel_hidden.get()
                    && (browsing
                        || model.tracks.len() > 1
                        || gtk::prelude::WidgetExt::is_visible(&self.search)),
            );
            self.title.set_text(if model.current.is_none() {
                "Drop some music here."
            } else {
                &model.title
            });
            let subtitle = [model.artist.as_str(), model.album.as_str()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            self.subtitle.set_text(&subtitle);
            self.subtitle.set_visible(!subtitle.is_empty());
            self.status
                .set_text(&model.status.chars().take(220).collect::<String>());
            self.status.set_tooltip_text(Some(&model.status));
            self.status.set_visible(!model.status.is_empty());
            self.window.set_title(Some(&if model.current.is_none() {
                "omaMusi".into()
            } else {
                format!("{} — omaMusi", model.title)
            }));
            self.transport.set_text(&format!(
                "{}{} / {}",
                if snapshot.paused { "paused · " } else { "" },
                clock(snapshot.position),
                clock(model.duration)
            ));
            self.transport.set_visible(model.current.is_some());
            self.volume_notice
                .set_text(&format!("vol {:.0}%", snapshot.volume * 100.0));
            self.volume_notice
                .set_visible(Instant::now() < self.volume_until.get());
            self.visual_notice
                .set_visible(Instant::now() < self.notice_until.get());
            self.hint.set_text(&format!("playback: space play/pause · n next · p previous · r random {} · +/= louder · − quieter\n{}\nmusic: / search files & folders · c folders · o/ctrl+o add files · ctrl+l folder path\nplaylists: a add track · b saved playlists\nview: v/shift+v next/previous visual · f fullscreen · tab playlist · esc back · q quit",if model.shuffle{"on"}else{"off"},if browsing{"browse: ↑↓ select (k/j) · →/enter open/play · ←/backspace parent · l play folder"}else{"select: ↑↓ select (k/j) · enter play · ←/→ seek 5s"}));
            let age = self.last_interaction.get().elapsed().as_secs_f64();
            let opacity = if model.current.is_none() || age < 4.0 {
                1.0
            } else {
                let t = ((age - 4.0) / 0.9).clamp(0.0, 1.0);
                1.0 - (if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                })
            };
            for widget in [
                self.logo.clone().upcast::<gtk::Widget>(),
                self.folder_label.clone().upcast(),
                self.folder_prompt.clone().upcast(),
                self.panel.clone().upcast(),
                self.status.clone().upcast(),
                self.hint.clone().upcast(),
                self.version_label.clone().upcast(),
            ] {
                widget.set_opacity(opacity);
            }
            self.panel.set_can_target(opacity > 0.05);
            // Faded hints must release their height so the remaining track
            // information sits at the bottom inset. Restore it on interaction.
            self.footer.set_visible(opacity > 0.0);
            self.status
                .set_visible(opacity > 0.0 && !model.status.is_empty());
            let root = self.logo.parent().unwrap().parent().unwrap();
            let minimum = root
                .measure(gtk::Orientation::Vertical, self.overlay.width().max(640))
                .0;
            let reserved = minimum - self.scroller.height_request().max(28);
            let available = (self.overlay.height() - reserved).max(28);
            self.scroller
                .set_size_request(-1, desired_list_height.min(available));
        }
        let left = if self.panel.is_visible() {
            30.0 + self.panel.width() as f64 + 24.0
        } else {
            30.0
        };
        self.state.borrow_mut().gallery_bounds = Rect {
            x: left,
            y: 64.0,
            width: (stage_width - left - 30.0).max(0.0),
            height: (self.overlay.height() as f64 - 250.0).max(0.0),
        };
        self.updating.set(false);
    }
    pub fn close(&self) {
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.remove();
        }
        let dialog = self.playlist_dialog.borrow_mut().take();
        if let Some(dialog) = dialog {
            dialog.close();
        }
        let mut state = self.state.borrow_mut();
        state.closed = true;
        state.background.close();
        state.audio.stop();
    }
}

pub fn run(cli: Cli) -> Result<()> {
    crate::install::check_dependencies()?;
    glib::set_prgname(Some("omaMusi"));
    glib::set_application_name("omaMusi");
    let app = gtk::Application::builder()
        .application_id("io.github.aridev1.omamusi")
        .build();
    run_app(&app, cli)
}
pub fn run_app(app: &gtk::Application, cli: Cli) -> Result<()> {
    let player: Rc<RefCell<Option<Rc<PlayerWindow>>>> = Rc::new(RefCell::new(None));
    let startup_error = Rc::new(RefCell::new(None));
    let failure = startup_error.clone();
    let stored = player.clone();
    let home = glib::home_dir();
    let data_root = crate::desktop::user_data_root();
    app.connect_activate(move |app| {
        if let Some(player) = stored.borrow().as_ref() {
            player.activate();
            return;
        }
        match PlayerWindow::new(app, &cli, data_root.clone(), home.clone()) {
            Ok(window) => {
                window.window.present();
                stored.replace(Some(window));
            }
            Err(error) => {
                failure.replace(Some(format!("{error:#}")));
                app.quit();
            }
        }
    });
    let stored = player.clone();
    app.connect_shutdown(move |_| {
        if let Some(player) = stored.borrow_mut().take() {
            player.close();
        }
    });
    let result = app.run_with_args::<&str>(&[]);
    if let Some(message) = startup_error.borrow_mut().take() {
        bail!("{message}");
    }
    if i32::from(result) != 0 {
        bail!("Player exited with {}", i32::from(result));
    }
    Ok(())
}

#[cfg(test)]
mod compositor_tests;
#[cfg(test)]
mod tests;
