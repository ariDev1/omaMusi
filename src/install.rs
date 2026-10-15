//! Atomic user-local installation and recognized Python-install migration.
use crate::desktop::{APP_ID, Desktop, atomic};
use anyhow::{Context, Result, bail};
use std::{
    fs, io,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

pub struct Installation {
    home: PathBuf,
    data: PathBuf,
}
impl Installation {
    pub fn new(home: PathBuf, data: PathBuf) -> Self {
        Self { home, data }
    }
    pub fn current() -> Self {
        Self::new(gtk::glib::home_dir(), crate::desktop::user_data_root())
    }
    pub fn directory(&self) -> PathBuf {
        self.data.join("omamusi")
    }
    pub fn binary(&self) -> PathBuf {
        self.directory().join("bin/omaMusi")
    }
    pub fn launcher(&self) -> PathBuf {
        self.home.join(".local/bin/omaMusi")
    }
    fn marker(&self) -> PathBuf {
        self.directory().join(".omamusi-managed")
    }
    fn managed(&self) -> bool {
        fs::symlink_metadata(self.marker())
            .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
            && fs::read_to_string(self.marker()).is_ok_and(|text| text.trim_end() == APP_ID)
    }
    fn legacy(&self) -> PathBuf {
        self.directory().join("venv/bin/python")
    }
    fn check_directory(&self, adoption: bool) -> Result<()> {
        if let Ok(metadata) = fs::symlink_metadata(self.directory()) {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                bail!(
                    "Refusing unmanaged installation at {}",
                    self.directory().display()
                );
            }
            if !self.managed() {
                if !adoption {
                    bail!(
                        "Refusing unmanaged installation at {}",
                        self.directory().display()
                    );
                }
                for entry in fs::read_dir(self.directory())? {
                    let entry = entry?;
                    if entry.file_name() != "playlists.json" || !entry.file_type()?.is_file() {
                        bail!(
                            "Refusing unmanaged installation at {}",
                            self.directory().display()
                        );
                    }
                }
            }
        }
        for path in [self.directory().join("bin"), self.directory().join("venv")] {
            if let Ok(metadata) = fs::symlink_metadata(&path)
                && (!metadata.is_dir() || metadata.file_type().is_symlink())
            {
                bail!("Refusing unsafe installation directory {}", path.display());
            }
        }
        if let Ok(metadata) = fs::symlink_metadata(self.binary())
            && (!metadata.is_file() || metadata.file_type().is_symlink())
        {
            bail!("Refusing unsafe installed executable");
        }
        Ok(())
    }
    fn check_launcher(&self) -> Result<()> {
        if fs::symlink_metadata(self.launcher()).is_ok() {
            let target = fs::read_link(self.launcher())
                .context("Another executable already owns the launcher")?;
            if !self.managed()
                || (target != self.binary() && target != self.directory().join("venv/bin/omaMusi"))
            {
                bail!(
                    "Another executable already owns {}",
                    self.launcher().display()
                );
            }
        }
        Ok(())
    }
    pub fn install(&self, source: &Path) -> Result<()> {
        self.check_directory(true)?;
        self.check_launcher()?;
        let desktop = Desktop::new(self.data.clone(), self.binary());
        let legacy = self.legacy();
        let legacy = self.managed().then_some(legacy.as_path());
        desktop.check_install(legacy)?;
        // Open the source before creating anything, and replace only after a full synced copy.
        let mut source = fs::File::open(source)?;
        if !source.metadata()?.is_file() {
            bail!("The source executable is not a regular file");
        }
        // Establish ownership before creating installation artifacts. A failed or
        // interrupted first copy/menu write then remains safe to retry. The
        // checks above permit adoption only of an empty or playlists-only directory.
        atomic(&self.marker(), format!("{APP_ID}\n").as_bytes())?;
        let bin = self.directory().join("bin");
        fs::create_dir_all(&bin)?;
        let mut staged = tempfile::NamedTempFile::new_in(&bin)?;
        io::copy(&mut source, &mut staged)?;
        staged
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o755))?;
        staged.as_file().sync_all()?;
        staged.persist(self.binary())?;
        desktop.install_with_legacy(legacy)?;
        let launcher = self.launcher();
        let parent = launcher.parent().context("Launcher has no directory")?;
        fs::create_dir_all(parent)?;
        let staged_link = tempfile::tempdir_in(parent)?;
        symlink(self.binary(), staged_link.path().join("omaMusi"))?;
        fs::rename(staged_link.path().join("omaMusi"), launcher)?;
        if self.directory().join("venv").exists() {
            fs::remove_dir_all(self.directory().join("venv"))?;
        }
        Ok(())
    }
    pub fn remove(&self) -> Result<()> {
        if !self.directory().exists() {
            return Ok(());
        }
        self.check_directory(false)?;
        Desktop::new(self.data.clone(), self.binary()).remove_with_legacy(Some(&self.legacy()))?;
        if fs::read_link(self.launcher()).is_ok_and(|target| {
            target == self.binary() || target == self.directory().join("venv/bin/omaMusi")
        }) {
            fs::remove_file(self.launcher())?;
        }
        if self.binary().exists() {
            fs::remove_file(self.binary())?;
        }
        if self.directory().join("venv").exists() {
            fs::remove_dir_all(self.directory().join("venv"))?;
        }
        Ok(())
    }
}

pub fn check_dependencies() -> Result<()> {
    for program in ["ffmpeg", "ffprobe"] {
        if !std::process::Command::new(program)
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
        {
            bail!("Missing or unusable {program}. Install FFmpeg first.");
        }
    }
    Ok(())
}
