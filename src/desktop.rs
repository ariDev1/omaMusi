//! User-local desktop entries with installation-specific ownership.
use anyhow::{Context, Result, bail};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const APP_ID: &str = "io.github.aridev1.omamusi";
const ICON: &[u8] = include_bytes!("../omamusi/icon.svg");
pub struct Desktop {
    data: PathBuf,
    executable: PathBuf,
}
impl Desktop {
    pub fn new(data: PathBuf, executable: PathBuf) -> Self {
        Self { data, executable }
    }
    pub fn current() -> Result<Self> {
        Ok(Self::new(
            gtk::glib::user_data_dir(),
            std::env::current_exe()?,
        ))
    }
    pub fn entry(&self) -> PathBuf {
        self.data.join(format!("applications/{APP_ID}.desktop"))
    }
    pub fn icon(&self) -> PathBuf {
        self.data
            .join(format!("icons/hicolor/scalable/apps/{APP_ID}.svg"))
    }
    fn command(&self) -> Result<String> {
        let path = self
            .executable
            .to_str()
            .context("Desktop executable path is not UTF-8")?;
        if !self.executable.is_absolute()
            || path.chars().any(|c| matches!(c, '\n' | '\r' | '\t' | '='))
        {
            bail!("Invalid desktop executable path");
        }
        let mut quoted = String::new();
        for c in path.chars() {
            if matches!(c, '\\' | '"' | '`' | '$') {
                quoted.push('\\');
            }
            quoted.push(c);
        }
        Ok(format!(
            "\"{}\" --desktop launch",
            quoted.replace('\\', "\\\\").replace('%', "%%")
        ))
    }
    fn owned(&self) -> Result<bool> {
        let path = self.entry();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            return Ok(false);
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Ok(false);
        }
        let text = fs::read_to_string(path)?;
        let command = format!("Exec={}", self.command()?);
        Ok(text.lines().any(|line| line == "X-omaMusi-Managed=true")
            && text.lines().any(|line| line == command))
    }
    pub fn install(&self) -> Result<PathBuf> {
        let entry = self.entry();
        let icon = self.icon();
        if fs::symlink_metadata(&entry).is_ok() && !self.owned()? {
            bail!("Another launcher already exists at {}", entry.display());
        }
        if let Ok(metadata) = fs::symlink_metadata(&icon)
            && (!metadata.is_file()
                || metadata.file_type().is_symlink()
                || fs::read(&icon)? != ICON)
        {
            bail!("An unrelated icon already exists at {}", icon.display());
        }
        let command = self.command()?;
        atomic(&icon, ICON)?;
        atomic(&entry, format!("[Desktop Entry]\nType=Application\nName=omaMusi\nComment=Play your music with animated visuals\nExec={command}\nIcon={APP_ID}\nTerminal=false\nCategories=AudioVideo;Audio;Player;\nKeywords=Music;Songs;Playlist;Visualizer;\nStartupNotify=false\nStartupWMClass=omaMusi\nX-omaMusi-Managed=true\n").as_bytes())?;
        Ok(entry)
    }
    pub fn remove(&self) -> Result<()> {
        if !self.owned()? {
            return Ok(());
        }
        fs::remove_file(self.entry())?;
        let icon = self.icon();
        if fs::symlink_metadata(&icon).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
            && fs::read(&icon)? == ICON
        {
            fs::remove_file(icon)?;
        }
        Ok(())
    }
}
fn atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent().context("No destination directory")?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(contents)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
pub fn music_cli() -> crate::cli::Cli {
    let home = gtk::glib::home_dir();
    let music = gtk::glib::user_special_dir(gtk::glib::UserDirectory::Music)
        .filter(|p| p.is_absolute() && *p != home && p.is_dir());
    crate::cli::Cli::for_music(music)
}
