//! Named playlists store file references, never the music itself.
use crate::library::{absolute, casefold};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
struct Document {
    version: u32,
    playlists: BTreeMap<String, Vec<String>>,
}

pub struct PlaylistStore {
    path: PathBuf,
    playlists: BTreeMap<String, Vec<String>>,
}

impl PlaylistStore {
    pub fn open(path: PathBuf) -> Result<Self> {
        let playlists = match fs::read(&path) {
            Ok(bytes) => {
                let data: Document = serde_json::from_slice(&bytes)
                    .context("Could not read saved playlists; the existing file was preserved")?;
                if data.version != 1
                    || data.playlists.iter().any(|(name, tracks)| {
                        name.trim().is_empty() || tracks.iter().any(|p| !Path::new(p).is_absolute())
                    })
                {
                    bail!("Invalid saved playlist format; the existing file was preserved");
                }
                data.playlists
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self { path, playlists })
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.playlists.keys().cloned().collect();
        names.sort_by_key(|s| casefold(s));
        names
    }
    pub fn tracks(&self, name: &str) -> Result<Vec<PathBuf>> {
        Ok(self
            .playlists
            .get(name)
            .context("Playlist does not exist")?
            .iter()
            .map(PathBuf::from)
            .collect())
    }
    fn name(&self, name: &str, previous: Option<&str>) -> Result<String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 120 || name.chars().any(|c| (c as u32) < 32) {
            bail!("Use a playlist name of 1–120 characters without line breaks");
        }
        if self
            .playlists
            .keys()
            .any(|other| Some(other.as_str()) != previous && casefold(other) == casefold(name))
        {
            bail!("A playlist with that name already exists");
        }
        Ok(name.to_owned())
    }
    fn track(path: &Path) -> Result<String> {
        absolute(path)?
            .to_str()
            .map(str::to_owned)
            .context("Saved playlists require UTF-8 file paths")
    }
    fn save(&mut self, playlists: BTreeMap<String, Vec<String>>) -> Result<()> {
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let mut temporary = tempfile::Builder::new()
            .prefix(".playlists-")
            .tempfile_in(parent)?;
        serde_json::to_writer_pretty(
            &mut temporary,
            &Document {
                version: 1,
                playlists: playlists.clone(),
            },
        )?;
        temporary.write_all(b"\n")?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        temporary.persist(&self.path).map_err(|e| e.error)?;
        self.playlists = playlists;
        Ok(())
    }
    pub fn create(&mut self, name: &str, track: Option<&Path>) -> Result<String> {
        let name = self.name(name, None)?;
        let tracks = track.map(Self::track).transpose()?.into_iter().collect();
        let mut playlists = self.playlists.clone();
        playlists.insert(name.clone(), tracks);
        self.save(playlists)?;
        Ok(name)
    }
    pub fn add(&mut self, name: &str, track: &Path) -> Result<bool> {
        let track = Self::track(track)?;
        let mut playlists = self.playlists.clone();
        let tracks = playlists.get_mut(name).context("Playlist does not exist")?;
        if tracks.contains(&track) {
            return Ok(false);
        }
        tracks.push(track);
        self.save(playlists)?;
        Ok(true)
    }
    pub fn rename(&mut self, name: &str, replacement: &str) -> Result<String> {
        let replacement = self.name(replacement, Some(name))?;
        let mut playlists = self.playlists.clone();
        let tracks = playlists.remove(name).context("Playlist does not exist")?;
        playlists.insert(replacement.clone(), tracks);
        self.save(playlists)?;
        Ok(replacement)
    }
    pub fn remove(&mut self, name: &str, track: &Path) -> Result<()> {
        let track = Self::track(track)?;
        let mut playlists = self.playlists.clone();
        playlists
            .get_mut(name)
            .context("Playlist does not exist")?
            .retain(|p| p != &track);
        self.save(playlists)
    }
    pub fn delete(&mut self, name: &str) -> Result<()> {
        let mut playlists = self.playlists.clone();
        playlists.remove(name).context("Playlist does not exist")?;
        self.save(playlists)
    }
}
