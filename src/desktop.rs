//! User-local desktop entries with installation-specific ownership.
use anyhow::{Context, Result, bail};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const APP_ID: &str = "io.github.aridev1.omamusi";
pub fn user_data_root() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| gtk::glib::home_dir().join(".local/share"))
}
const ICON: &[u8] = include_bytes!("../assets/icon.svg");
pub struct Desktop {
    data: PathBuf,
    executable: PathBuf,
}
impl Desktop {
    pub fn new(data: PathBuf, executable: PathBuf) -> Self {
        Self { data, executable }
    }
    pub fn current() -> Result<Self> {
        Ok(Self::new(user_data_root(), std::env::current_exe()?))
    }
    pub fn entry(&self) -> PathBuf {
        self.data.join(format!("applications/{APP_ID}.desktop"))
    }
    pub fn icon(&self) -> PathBuf {
        self.data
            .join(format!("icons/hicolor/scalable/apps/{APP_ID}.svg"))
    }
    fn command(&self) -> Result<String> {
        Ok(format!(
            "{} --desktop launch",
            quote_executable(&self.executable)?
        ))
    }
    fn owned(&self) -> Result<bool> {
        self.owned_command(&self.command()?)
    }
    fn owned_command(&self, command: &str) -> Result<bool> {
        let path = self.entry();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            return Ok(false);
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Ok(false);
        }
        let text = fs::read_to_string(path)?;
        let command = format!("Exec={command}");
        Ok(text.lines().any(|line| line == "X-omaMusi-Managed=true")
            && text.lines().any(|line| line == command))
    }
    fn legacy_owned(&self, legacy: Option<&Path>) -> Result<bool> {
        match legacy {
            Some(path) => self.owned_command(&format!(
                "{} -m omamusi.desktop launch",
                quote_executable(path)?
            )),
            None => Ok(false),
        }
    }
    pub(crate) fn check_install(&self, legacy: Option<&Path>) -> Result<()> {
        let entry = self.entry();
        let icon = self.icon();
        if fs::symlink_metadata(&entry).is_ok() && !self.owned()? && !self.legacy_owned(legacy)? {
            bail!("Another launcher already exists at {}", entry.display());
        }
        if let Ok(metadata) = fs::symlink_metadata(&icon)
            && (!metadata.is_file()
                || metadata.file_type().is_symlink()
                || fs::read(&icon)? != ICON)
        {
            bail!("An unrelated icon already exists at {}", icon.display());
        }
        self.command()?;
        Ok(())
    }
    pub fn install(&self) -> Result<PathBuf> {
        self.install_with_legacy(None)
    }
    pub(crate) fn install_with_legacy(&self, legacy: Option<&Path>) -> Result<PathBuf> {
        self.check_install(legacy)?;
        let entry = self.entry();
        let icon = self.icon();
        let command = self.command()?;
        atomic(&icon, ICON)?;
        atomic(&entry, format!("[Desktop Entry]\nType=Application\nName=omaMusi\nComment=Play your music with animated visuals\nExec={command}\nIcon={APP_ID}\nTerminal=false\nCategories=AudioVideo;Audio;Player;\nKeywords=Music;Songs;Playlist;Visualizer;\nStartupNotify=false\nStartupWMClass=omaMusi\nX-omaMusi-Managed=true\n").as_bytes())?;
        Ok(entry)
    }
    pub fn remove(&self) -> Result<()> {
        self.remove_with_legacy(None)
    }
    pub(crate) fn remove_with_legacy(&self, legacy: Option<&Path>) -> Result<()> {
        if !self.owned()? && !self.legacy_owned(legacy)? {
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
fn quote_executable(executable: &Path) -> Result<String> {
    let path = executable
        .to_str()
        .context("Desktop executable path is not UTF-8")?;
    if !executable.is_absolute() || path.chars().any(|c| matches!(c, '\n' | '\r' | '\t' | '=')) {
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
        "\"{}\"",
        quoted.replace('\\', "\\\\").replace('%', "%%")
    ))
}
pub(crate) fn atomic(path: &Path, contents: &[u8]) -> Result<()> {
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

/// Run off the GTK thread; only a numeric PID and an owned window title enter IPC.
pub(crate) fn focus_hyprland(program: &std::ffi::OsStr, pid: u32, title: &str) -> Result<()> {
    use std::{process::Command, time::Duration};
    let run = |arguments: &[&str], limit| {
        let mut command = Command::new(program);
        command.args(arguments);
        crate::process::capture(command, Duration::from_secs(2), limit)
    };
    let mut target = format!("pid:{pid}");
    if let Ok(output) = run(&["-j", "clients"], 4 * 1024 * 1024)
        && let Ok(clients) = serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout)
        && let Some(address) = clients
            .iter()
            .find(|client| {
                client["pid"].as_u64() == Some(pid as u64)
                    && client["title"].as_str() == Some(title)
            })
            .and_then(|client| client["address"].as_str())
        && address.strip_prefix("0x").is_some_and(|suffix| {
            !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_hexdigit())
        })
    {
        target = format!("address:{address}");
    }
    if run(&["dispatch", "focuswindow", &target], 4096).is_ok() {
        return Ok(());
    }
    // https://wiki.hypr.land/Configuring/Basics/Dispatchers/ (Hyprland 0.55 Lua).
    let expression = format!(
        "hl.dsp.focus({{window={}}})",
        serde_json::to_string(&target)?
    );
    run(&["dispatch", &expression], 4096)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn focus_assistance_selects_modal_window_and_handles_both_hyprland_syntaxes() {
        for legacy in [true, false] {
            let root = tempfile::tempdir().unwrap();
            let program = root.path().join("hyprctl");
            let log = root.path().join("commands");
            fs::write(&program, format!(r#"#!/bin/sh
if [ "$1" = "-j" ]; then
  printf '%s' '[{{"pid":7,"title":"omaMusi","address":"0x111"}},{{"pid":7,"title":"Saved playlists","address":"0x222"}}]'
  exit 0
fi
printf '%s\n' "$2" "$3" >> '{}'
if [ "$2" = focuswindow ]; then exit {}; fi
[ "$2" = 'hl.dsp.focus({{window="address:0x222"}})' ]
"#, log.display(), if legacy {0} else {1})).unwrap();
            fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
            focus_hyprland(program.as_os_str(), 7, "Saved playlists").unwrap();
            let commands = fs::read_to_string(log).unwrap();
            assert!(commands.contains("focuswindow\naddress:0x222"));
            assert_eq!(commands.contains("hl.dsp.focus"), !legacy);
        }
    }
}
