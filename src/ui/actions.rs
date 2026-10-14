use super::*;
impl PlayerWindow {
    pub fn handle_key(
        self: &Rc<Self>,
        key: gdk::Key,
        modifiers: gdk::ModifierType,
    ) -> glib::Propagation {
        self.interact();
        let focus = gtk::prelude::GtkWindowExt::focus(&self.window);
        let in_search = gtk::prelude::WidgetExt::is_visible(&self.search)
            && focus
                .as_ref()
                .is_some_and(|w| w == &self.search || w.is_ancestor(&self.search));
        let in_folder = self.folder_prompt.is_visible()
            && focus
                .as_ref()
                .is_some_and(|w| w == &self.folder_input || w.is_ancestor(&self.folder_input));
        if key == gdk::Key::Escape {
            if self.folder_prompt.is_visible() {
                self.folder_prompt.set_visible(false);
                self.dispatch(Action::CancelBrowser);
            } else if self.state.borrow().model.browser_folder.is_some() {
                self.dispatch(Action::CancelBrowser);
            } else if in_search {
                if self.search.text().is_empty() {
                    self.search.set_visible(false);
                }
                self.focus_player();
            } else if self.window.is_fullscreen() {
                self.window.unfullscreen();
            }
            self.focus_player();
            return glib::Propagation::Stop;
        }
        if in_search && matches!(key, gdk::Key::Up | gdk::Key::Down) {
            self.dispatch(Action::MoveSelection(if key == gdk::Key::Down {
                1
            } else {
                -1
            }));
            if !self.state.borrow().model.search_focus {
                self.focus_player();
                self.scroll_selection();
            }
            return glib::Propagation::Stop;
        }
        if in_search || in_folder {
            return glib::Propagation::Proceed;
        }
        let control = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
        let reserved = modifiers.intersects(
            gdk::ModifierType::CONTROL_MASK
                | gdk::ModifierType::ALT_MASK
                | gdk::ModifierType::SUPER_MASK
                | gdk::ModifierType::META_MASK,
        );
        let letter = key.to_unicode().map(|c| c.to_ascii_lowercase());
        if control && letter == Some('o') {
            self.open_files();
            return glib::Propagation::Stop;
        }
        if control && letter == Some('l') {
            self.prompt_folder();
            return glib::Propagation::Stop;
        }
        if reserved {
            return glib::Propagation::Proceed;
        }
        match key {
            gdk::Key::space => self.dispatch(Action::TogglePause),
            gdk::Key::Up | gdk::Key::Down => {
                self.move_selection(if key == gdk::Key::Down { 1 } else { -1 })
            }
            gdk::Key::Return | gdk::Key::KP_Enter => self.activate_folder_item(),
            gdk::Key::Right => {
                if self.state.borrow().model.browser_folder.is_some() {
                    self.activate_folder_item();
                } else {
                    self.seek_relative(5.0);
                }
            }
            gdk::Key::Left | gdk::Key::BackSpace => {
                if self.state.borrow().model.browser_folder.is_some() {
                    self.folder_parent();
                } else if key == gdk::Key::Left {
                    self.seek_relative(-5.0);
                }
            }
            gdk::Key::Tab => {
                self.panel_hidden.set(!self.panel_hidden.get());
                self.refresh();
            }
            _ => match letter {
                Some('+') | Some('=') => self.volume(0.05),
                Some('-') => self.volume(-0.05),
                Some('n') => self.dispatch(Action::Next { automatic: false }),
                Some('p') => {
                    let snapshot = self.state.borrow().audio.snapshot();
                    self.dispatch(Action::Previous {
                        position: snapshot.position,
                        has_stream: snapshot.has_stream,
                    });
                }
                Some('r') => self.dispatch(Action::ToggleShuffle),
                Some('j') => self.move_selection(1),
                Some('k') => self.move_selection(-1),
                Some('/') => {
                    self.dispatch(Action::CancelBrowser);
                    self.panel_hidden.set(false);
                    self.search.set_visible(true);
                    self.search.grab_focus();
                    self.state.borrow_mut().model.search_focus = true;
                    self.refresh();
                }
                Some('c') => {
                    let folder = self.state.borrow().model.current_folder.clone();
                    self.dispatch(Action::Browse(folder));
                }
                Some('l') => {
                    let folder = self.state.borrow().model.browser_folder.clone();
                    if let Some(folder) = folder {
                        self.dispatch(Action::ScanFolder(folder));
                    }
                }
                Some('o') => self.open_files(),
                Some('a') => self.open_playlists(true),
                Some('b') => self.open_playlists(false),
                Some('v') => {
                    let mode = self.state.borrow_mut().visual.cycle(
                        if modifiers.contains(gdk::ModifierType::SHIFT_MASK) {
                            -1
                        } else {
                            1
                        },
                    );
                    if mode != VisualMode::CoverGallery {
                        self.state.borrow_mut().gallery.hovered = None;
                    }
                    self.visual_notice.set_text(mode.name());
                    self.notice_until
                        .set(Instant::now() + Duration::from_secs(2));
                    self.ensure_artwork();
                    self.ensure_gallery();
                    self.canvas.queue_draw();
                    self.refresh();
                }
                Some('f') => {
                    if self.window.is_fullscreen() {
                        self.window.unfullscreen();
                    } else {
                        self.window.fullscreen();
                    }
                }
                Some('q') => self.window.close(),
                _ => return glib::Propagation::Proceed,
            },
        }
        glib::Propagation::Stop
    }
    fn move_selection(&self, delta: i32) {
        {
            let mut state = self.state.borrow_mut();
            if state.model.browser_folder.is_some() {
                let count = state.browser_entries.len();
                if count > 0 {
                    state.browser_selected = (state.browser_selected as i64 + delta as i64)
                        .rem_euclid(count as i64)
                        as usize;
                }
                drop(state);
                self.refresh();
                return;
            }
        }
        self.dispatch(Action::MoveSelection(delta));
        self.scroll_selection();
    }
    fn scroll_selection(&self) {
        let selected = self.state.borrow().model.selected;
        if let Some(index) = selected
            && let Some(row) = self.track_rows.borrow().get(index)
            && let Some(bounds) = row.compute_bounds(&self.listing)
        {
            let adjustment = self.scroller.vadjustment();
            let top = bounds.y() as f64;
            let bottom = top + bounds.height() as f64;
            if top < adjustment.value() {
                adjustment.set_value(top);
            } else if bottom > adjustment.value() + adjustment.page_size() {
                adjustment.set_value(bottom - adjustment.page_size());
            }
        }
    }
    fn seek_relative(&self, delta: f64) {
        let mut state = self.state.borrow_mut();
        let snapshot = state.audio.snapshot();
        if !snapshot.has_stream {
            return;
        }
        let mut target = (snapshot.position + delta).max(0.0);
        if state.model.duration > 0.0 {
            target = target.min((state.model.duration - 0.1).max(0.0));
        }
        if let Err(error) = state.audio.seek(target) {
            state.model.status = format!("error: {error:#}");
        }
        state.visual.reset();
        drop(state);
        self.refresh();
    }
    fn volume(&self, delta: f32) {
        let mut state = self.state.borrow_mut();
        let value = ((state.audio.snapshot().volume + delta) * 100.0).round() / 100.0;
        state.audio.set_volume(value);
        drop(state);
        self.volume_until
            .set(Instant::now() + Duration::from_secs(2));
        self.refresh();
    }
    fn prompt_folder(&self) {
        self.folder_input
            .set_text(&self.state.borrow().model.current_folder.to_string_lossy());
        self.folder_prompt.set_visible(true);
        self.folder_label.set_visible(false);
        self.folder_input.grab_focus();
        self.folder_input.select_region(0, -1);
    }
    fn folder_parent(&self) {
        let folder = self
            .state
            .borrow()
            .model
            .browser_folder
            .as_ref()
            .and_then(|p| p.parent())
            .map(Path::to_owned);
        if let Some(folder) = folder {
            self.dispatch(Action::Browse(folder));
        }
    }
    pub(super) fn activate_folder_item(&self) {
        let entry = {
            let state = self.state.borrow();
            if state.model.browser_folder.is_some() {
                state.browser_entries.get(state.browser_selected).cloned()
            } else {
                None
            }
        };
        match entry {
            Some(BrowserEntry::Directory(path)) => self.dispatch(Action::Browse(path)),
            Some(BrowserEntry::Play(path)) => {
                self.state.borrow_mut().folder_start = None;
                self.dispatch(Action::ScanFolder(path));
            }
            Some(BrowserEntry::Song(path)) => {
                let folder = path.parent().unwrap_or(Path::new("/")).to_owned();
                self.state.borrow_mut().folder_start = Some(path);
                self.dispatch(Action::ScanFolder(folder));
            }
            None => {
                let index = self.state.borrow().model.selected;
                if let Some(index) = index {
                    self.dispatch(Action::Play(index));
                }
            }
        }
    }
    fn open_files(self: &Rc<Self>) {
        let dialog = gtk::FileChooserNative::new(
            Some("Add music"),
            Some(&self.window),
            gtk::FileChooserAction::Open,
            Some("Add"),
            Some("Cancel"),
        );
        dialog.set_select_multiple(true);
        let audio = gtk::FileFilter::new();
        audio.set_name(Some("Audio files"));
        for extension in [
            "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "aiff", "wma", "ape", "alac", "mka",
            "mp2", "ac3", "wv", "amr", "au",
        ] {
            audio.add_pattern(&format!("*.{extension}"));
        }
        dialog.add_filter(&audio);
        let all = gtk::FileFilter::new();
        all.set_name(Some("All files"));
        all.add_pattern("*");
        dialog.add_filter(&all);
        let _ = dialog.set_current_folder(Some(&gio::File::for_path(
            &self.state.borrow().model.current_folder,
        )));
        let weak = Rc::downgrade(self);
        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept
                && let Some(player) = weak.upgrade()
            {
                let files = dialog.files();
                let paths = (0..files.n_items())
                    .filter_map(|i| {
                        files
                            .item(i)
                            .and_then(|item| item.downcast::<gio::File>().ok())
                            .and_then(|file| file.path())
                    })
                    .collect();
                player.drop_paths(paths);
            }
            dialog.destroy();
        });
        dialog.show();
    }
    fn open_playlists(self: &Rc<Self>, adding: bool) {
        if let Some(dialog) = self.playlist_dialog.borrow().as_ref() {
            dialog.window.present();
            return;
        }
        let target = {
            let state = self.state.borrow();
            if adding {
                if state.model.browser_folder.is_some() {
                    match state.browser_entries.get(state.browser_selected) {
                        Some(BrowserEntry::Song(path)) => Some(path.clone()),
                        _ => state
                            .model
                            .current
                            .and_then(|i| state.model.tracks.get(i))
                            .cloned(),
                    }
                } else {
                    state
                        .model
                        .selected
                        .filter(|i| {
                            state
                                .model
                                .tracks
                                .get(*i)
                                .is_some_and(|p| library::matches(p, &state.model.query))
                        })
                        .or(state.model.current)
                        .and_then(|i| state.model.tracks.get(i))
                        .cloned()
                }
            } else {
                None
            }
        };
        if adding && target.is_none() {
            self.notice("Select a track first, or start playing one.");
            return;
        }
        let dialog =
            PlaylistDialog::new(self, self.data_root.join("omamusi/playlists.json"), target);
        self.playlist_dialog.replace(Some(dialog.clone()));
        dialog.window.present();
    }
    pub(super) fn notice(&self, message: &str) {
        {
            let mut state = self.state.borrow_mut();
            state.model.status = message.into();
            if state.model.loading {
                state.pending_notice = Some(message.into());
            }
        }
        self.interact();
        self.refresh();
    }
    pub(super) fn load_validated_playlist(
        &self,
        name: String,
        tracks: Vec<PathBuf>,
        missing: usize,
    ) {
        if tracks.is_empty() {
            return;
        }
        let count = tracks.len();
        let folder = tracks[0].parent().unwrap_or(Path::new("/")).to_owned();
        {
            let mut state = self.state.borrow_mut();
            state.model.current_folder = folder.clone();
            state.model.library_root = folder;
        }
        self.dispatch(Action::ReplaceQueue(tracks, None));
        self.notice(&format!(
            "Loaded playlist: {name} · {count} tracks · {missing} missing files skipped"
        ));
    }
}
