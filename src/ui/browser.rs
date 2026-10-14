use crate::library::{absolute, directory_entries, is_audio, natural_cmp};
use anyhow::Result;
use std::path::{Path, PathBuf};
#[derive(Clone, Debug)]
pub enum BrowserEntry {
    Play(PathBuf),
    Directory(PathBuf),
    Song(PathBuf),
}
impl BrowserEntry {
    pub fn label(&self, folder: &Path) -> String {
        match self {
            Self::Play(_) => "  ▶ play this folder".into(),
            Self::Directory(path) if Some(path.as_path()) == folder.parent() => "  ../".into(),
            Self::Directory(path) => format!(
                "  {}/",
                path.file_name().unwrap_or_default().to_string_lossy()
            ),
            Self::Song(path) => format!(
                "  {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ),
        }
    }
}
pub fn entries(folder: &Path) -> Result<Vec<BrowserEntry>> {
    let folder = absolute(folder)?;
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for (path, dir, _) in directory_entries(&folder)? {
        if dir {
            dirs.push(path);
        } else if is_audio(&path) {
            files.push(path);
        }
    }
    dirs.sort_by(|a, b| natural_cmp(a, b));
    files.sort_by(|a, b| natural_cmp(a, b));
    let mut result = vec![BrowserEntry::Play(folder.clone())];
    if let Some(parent) = folder.parent() {
        result.push(BrowserEntry::Directory(parent.to_owned()));
    }
    result.extend(dirs.into_iter().map(BrowserEntry::Directory));
    result.extend(files.into_iter().map(BrowserEntry::Song));
    Ok(result)
}
