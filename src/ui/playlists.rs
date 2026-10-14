//! Modal keyboard-first playlists with all file operations on a bounded worker.
use super::*;
use crate::playlists::PlaylistStore;
use std::{collections::BTreeMap, rc::Weak};
#[derive(Clone)]
struct Track {
    path: PathBuf,
    available: bool,
}
struct Snapshot {
    playlists: BTreeMap<String, Vec<Track>>,
}
enum Operation {
    Read,
    Create(String, Option<PathBuf>),
    Add(String, PathBuf),
    Rename(String, String),
    Remove(String, PathBuf),
    Delete(String),
    Play(String),
}
struct Reply {
    snapshot: Snapshot,
    notice: Option<String>,
    selected: Option<String>,
    play: Option<(String, Vec<PathBuf>, usize)>,
    close: bool,
}
fn operate(path: PathBuf, operation: Operation) -> Result<Reply> {
    let mut store = PlaylistStore::open(path)?;
    let mut notice = None;
    let mut selected = None;
    let mut play = None;
    let mut close = false;
    match operation {
        Operation::Read => {}
        Operation::Create(name, track) => {
            let name = store.create(&name, track.as_deref())?;
            if track.is_some() {
                notice = Some(format!("Added to playlist: {name}"));
                close = true;
            }
            selected = Some(name);
        }
        Operation::Add(name, track) => {
            let added = store.add(&name, &track)?;
            notice = Some(format!(
                "{}: {name}",
                if added {
                    "Added to playlist"
                } else {
                    "Already in playlist"
                }
            ));
            close = true;
        }
        Operation::Rename(name, replacement) => {
            selected = Some(store.rename(&name, &replacement)?);
        }
        Operation::Remove(name, track) => store.remove(&name, &track)?,
        Operation::Delete(name) => store.delete(&name)?,
        Operation::Play(name) => {
            let all = store.tracks(&name)?;
            let available: Vec<_> = all.iter().filter(|p| p.is_file()).cloned().collect();
            if available.is_empty() {
                bail!("This playlist has no available tracks. → inspects its entries.");
            }
            play = Some((name, available.clone(), all.len() - available.len()));
            close = true;
        }
    }
    let mut playlists = BTreeMap::new();
    for name in store.names() {
        playlists.insert(
            name.clone(),
            store
                .tracks(&name)?
                .into_iter()
                .map(|path| {
                    let available = path.is_file();
                    Track { path, available }
                })
                .collect(),
        );
    }
    Ok(Reply {
        snapshot: Snapshot { playlists },
        notice,
        selected,
        play,
        close,
    })
}
#[derive(Clone)]
enum Prompt {
    Create,
    Rename(String),
    Delete(String),
}
pub(super) struct PlaylistDialog {
    pub(super) window: gtk::Window,
    pub(super) name_input: gtk::Entry,
    pub(super) busy: Cell<bool>,
    listing: gtk::ListBox,
    heading: gtk::Label,
    message: gtk::Label,
    help: gtk::Label,
    parent: Weak<PlayerWindow>,
    path: PathBuf,
    target: Option<PathBuf>,
    snapshot: RefCell<Snapshot>,
    name: RefCell<Option<String>>,
    prompt: RefCell<Option<Prompt>>,
    rows: RefCell<Vec<Option<String>>>,
    background: BackgroundTasks<Reply>,
    timer: RefCell<Option<glib::SourceId>>,
    closed: Cell<bool>,
    selected_after: RefCell<Option<String>>,
}
impl PlaylistDialog {
    pub(super) fn new(
        parent: &Rc<PlayerWindow>,
        path: PathBuf,
        target: Option<PathBuf>,
    ) -> Rc<Self> {
        let window = gtk::Window::builder()
            .title(if target.is_some() {
                "Add to playlist"
            } else {
                "Saved playlists"
            })
            .transient_for(&parent.window)
            .modal(true)
            .default_width(560)
            .default_height(400)
            .build();
        window.add_css_class("omamusi-playlists");
        window.set_size_request(360, 260);
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        for side in [gtk::PositionType::Left, gtk::PositionType::Right] {
            if side == gtk::PositionType::Left {
                root.set_margin_start(20);
            } else {
                root.set_margin_end(20);
            }
        }
        root.set_margin_top(20);
        root.set_margin_bottom(20);
        window.set_child(Some(&root));
        let heading = label("Loading playlists…", "omamusi-accent");
        root.append(&heading);
        let listing = super::make_listing();
        let scroller = gtk::ScrolledWindow::new();
        scroller.set_vexpand(true);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_child(Some(&listing));
        root.append(&scroller);
        let name_input = entry("Playlist name");
        name_input.set_max_length(120);
        name_input.set_visible(false);
        root.append(&name_input);
        let message = label("", "omamusi-muted");
        message.set_wrap(true);
        root.append(&message);
        let help = label("", "omamusi-hint");
        help.set_wrap(true);
        root.append(&help);
        let dialog = Rc::new(Self {
            window,
            name_input,
            busy: Cell::new(false),
            listing,
            heading,
            message,
            help,
            parent: Rc::downgrade(parent),
            path,
            target,
            snapshot: RefCell::new(Snapshot {
                playlists: BTreeMap::new(),
            }),
            name: RefCell::new(None),
            prompt: RefCell::new(None),
            rows: RefCell::new(Vec::new()),
            background: BackgroundTasks::new(),
            timer: RefCell::new(None),
            closed: Cell::new(false),
            selected_after: RefCell::new(None),
        });
        let weak = Rc::downgrade(&dialog);
        dialog.window.connect_close_request(move |_| {
            if let Some(dialog) = weak.upgrade() {
                dialog.close();
            }
            glib::Propagation::Proceed
        });
        let key = gtk::EventControllerKey::new();
        key.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&dialog);
        key.connect_key_pressed(move |_, key, _, modifiers| {
            weak.upgrade()
                .map(|dialog| dialog.handle_key(key, modifiers))
                .unwrap_or(glib::Propagation::Proceed)
        });
        dialog.window.add_controller(key);
        let weak = Rc::downgrade(&dialog);
        dialog.name_input.connect_activate(move |_| {
            if let Some(dialog) = weak.upgrade() {
                dialog.confirm_prompt();
            }
        });
        let weak = Rc::downgrade(&dialog);
        dialog.listing.connect_row_activated(move |_, _| {
            if let Some(dialog) = weak.upgrade() {
                dialog.activate_selection();
            }
        });
        let weak = Rc::downgrade(&dialog);
        let timer = glib::timeout_add_local(Duration::from_millis(15), move || {
            if let Some(dialog) = weak.upgrade() {
                if dialog.closed.get() {
                    return glib::ControlFlow::Break;
                }
                dialog.poll();
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
        dialog.timer.replace(Some(timer));
        dialog.submit(Operation::Read);
        dialog
    }
    fn submit(&self, operation: Operation) {
        if self.busy.replace(true) {
            return;
        }
        self.listing.set_sensitive(false);
        self.name_input.set_sensitive(false);
        let path = self.path.clone();
        self.background
            .submit("playlists", move || operate(path, operation));
    }
    fn poll(&self) {
        for reply in self.background.poll() {
            self.busy.set(false);
            self.listing.set_sensitive(true);
            self.name_input.set_sensitive(true);
            match reply.result {
                Ok(reply) => {
                    self.snapshot.replace(reply.snapshot);
                    if let Some(parent) = self.parent.upgrade() {
                        if let Some(notice) = reply.notice {
                            parent.notice(&notice);
                        }
                        if let Some((name, tracks, missing)) = reply.play {
                            parent.load_validated_playlist(name, tracks, missing);
                        }
                    }
                    self.cancel_prompt();
                    self.selected_after.replace(reply.selected);
                    self.refresh();
                    if reply.close {
                        self.window.close();
                    }
                }
                Err(error) => {
                    self.message.set_text(&format!("{error:#}"));
                }
            }
        }
    }
    fn refresh(&self) {
        while let Some(child) = self.listing.first_child() {
            self.listing.remove(&child);
        }
        let mut rows = Vec::new();
        let snapshot = self.snapshot.borrow();
        if let Some(name) = self.name.borrow().as_ref() {
            self.heading.set_text(name);
            if let Some(tracks) = snapshot.playlists.get(name) {
                for track in tracks {
                    let row = gtk::ListBoxRow::new();
                    row.set_child(Some(&label(
                        &format!(
                            "{}{}",
                            track.path.file_stem().unwrap_or_default().to_string_lossy(),
                            if track.available { "" } else { " [missing]" }
                        ),
                        "",
                    )));
                    row.set_tooltip_text(Some(&track.path.to_string_lossy()));
                    self.listing.append(&row);
                    rows.push(None);
                }
            }
            self.help
                .set_text("↑↓ select · delete remove track · ← back · esc close");
        } else {
            self.heading.set_text(
                &self
                    .target
                    .as_ref()
                    .map(|path| {
                        format!(
                            "Add {} to…",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        )
                    })
                    .unwrap_or_else(|| "Saved playlists".into()),
            );
            let mut names: Vec<_> = snapshot.playlists.keys().collect();
            names.sort_by_key(|name| library::casefold(name));
            for name in names {
                let row = gtk::ListBoxRow::new();
                row.set_child(Some(&label(
                    &format!("{name} · {} tracks", snapshot.playlists[name].len()),
                    "",
                )));
                self.listing.append(&row);
                rows.push(Some(name.clone()));
            }
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&label("Create new playlist…", "")));
            self.listing.append(&row);
            rows.push(None);
            self.help.set_text(if self.target.is_some() {
                "↑↓ select · enter add · esc cancel"
            } else {
                "↑↓ select · enter play · → tracks · F2 rename · delete remove · esc close"
            });
        }
        let selected = self.selected_after.borrow_mut().take();
        let index = selected
            .and_then(|name| rows.iter().position(|row| row.as_ref() == Some(&name)))
            .unwrap_or(0);
        self.rows.replace(rows);
        if let Some(row) = self.listing.row_at_index(index as i32) {
            self.listing.select_row(Some(&row));
        }
    }
    fn selected(&self) -> Option<String> {
        self.listing.selected_row().and_then(|row| {
            self.rows
                .borrow()
                .get(row.index() as usize)
                .cloned()
                .flatten()
        })
    }
    fn begin_prompt(&self, prompt: Prompt) {
        let text = match &prompt {
            Prompt::Create => "New playlist name:".into(),
            Prompt::Rename(_) => "Rename playlist:".into(),
            Prompt::Delete(name) => format!(
                "Delete playlist “{name}”? Enter confirms · Escape cancels. Music files are kept."
            ),
        };
        self.message.set_text(&text);
        if !matches!(prompt, Prompt::Delete(_)) {
            self.name_input.set_text(match &prompt {
                Prompt::Rename(name) => name,
                _ => "",
            });
            self.name_input.set_visible(true);
            self.name_input.grab_focus();
            self.name_input.select_region(0, -1);
        } else {
            gtk::prelude::GtkWindowExt::set_focus(&self.window, Some(&self.listing));
        }
        self.prompt.replace(Some(prompt));
    }
    fn cancel_prompt(&self) {
        self.prompt.replace(None);
        self.name_input.set_visible(false);
        self.message.set_text("");
        gtk::prelude::GtkWindowExt::set_focus(&self.window, Some(&self.listing));
    }
    fn confirm_prompt(&self) {
        let prompt = self.prompt.borrow().clone();
        if let Some(prompt) = prompt {
            match prompt {
                Prompt::Create => self.submit(Operation::Create(
                    self.name_input.text().to_string(),
                    self.target.clone(),
                )),
                Prompt::Rename(name) => {
                    self.submit(Operation::Rename(name, self.name_input.text().to_string()))
                }
                Prompt::Delete(name) => self.submit(Operation::Delete(name)),
            }
        }
    }
    pub(super) fn activate_selection(&self) {
        if self.busy.get() || self.name.borrow().is_some() {
            return;
        }
        if let Some(name) = self.selected() {
            if let Some(target) = &self.target {
                self.submit(Operation::Add(name, target.clone()));
            } else {
                self.submit(Operation::Play(name));
            }
        } else {
            self.begin_prompt(Prompt::Create);
        }
    }
    fn remove_selection(&self) {
        if self.target.is_some() {
            return;
        }
        if let Some(name) = self.name.borrow().clone() {
            let path = self.listing.selected_row().and_then(|row| {
                self.snapshot
                    .borrow()
                    .playlists
                    .get(&name)
                    .and_then(|tracks| tracks.get(row.index() as usize))
                    .map(|track| track.path.clone())
            });
            if let Some(path) = path {
                self.submit(Operation::Remove(name, path));
            }
        } else if let Some(name) = self.selected() {
            self.begin_prompt(Prompt::Delete(name));
        }
    }
    pub(super) fn handle_key(
        &self,
        key: gdk::Key,
        modifiers: gdk::ModifierType,
    ) -> glib::Propagation {
        if key == gdk::Key::Escape {
            if self.prompt.borrow().is_some() {
                self.cancel_prompt();
            } else {
                self.window.close();
            }
            return glib::Propagation::Stop;
        }
        if self.busy.get() {
            return glib::Propagation::Stop;
        }
        if gtk::prelude::WidgetExt::is_visible(&self.name_input) {
            return glib::Propagation::Proceed;
        }
        if modifiers.intersects(
            gdk::ModifierType::CONTROL_MASK
                | gdk::ModifierType::ALT_MASK
                | gdk::ModifierType::SUPER_MASK
                | gdk::ModifierType::META_MASK,
        ) {
            return glib::Propagation::Proceed;
        }
        if self.prompt.borrow().is_some() {
            if matches!(key, gdk::Key::Return | gdk::Key::KP_Enter) {
                self.confirm_prompt();
            }
            return glib::Propagation::Stop;
        }
        match key {
            gdk::Key::Up | gdk::Key::Down => {
                let count = self.rows.borrow().len();
                if count > 0 {
                    let selected = self
                        .listing
                        .selected_row()
                        .map(|row| row.index())
                        .unwrap_or(0);
                    let index = (selected + if key == gdk::Key::Down { 1 } else { -1 })
                        .rem_euclid(count as i32);
                    if let Some(row) = self.listing.row_at_index(index) {
                        self.listing.select_row(Some(&row));
                    }
                }
            }
            gdk::Key::Return | gdk::Key::KP_Enter => self.activate_selection(),
            gdk::Key::Right if self.target.is_none() && self.name.borrow().is_none() => {
                if let Some(name) = self.selected() {
                    self.name.replace(Some(name));
                    self.refresh();
                }
            }
            gdk::Key::Left => {
                let old = self.name.borrow_mut().take();
                self.selected_after.replace(old);
                self.refresh();
            }
            gdk::Key::F2 if self.target.is_none() && self.name.borrow().is_none() => {
                if let Some(name) = self.selected() {
                    self.begin_prompt(Prompt::Rename(name));
                }
            }
            gdk::Key::Delete => self.remove_selection(),
            _ => return glib::Propagation::Proceed,
        };
        glib::Propagation::Stop
    }
    pub(super) fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        self.background.close();
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.remove();
        }
        if let Some(parent) = self.parent.upgrade() {
            parent.playlist_dialog.replace(None);
            parent.focus_player();
        }
    }
}
