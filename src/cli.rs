//! Compatible player arguments and native desktop operations.
use crate::visual::VisualMode;
use clap::{Parser, ValueEnum};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum DesktopAction {
    Install,
    Remove,
    Launch,
}
fn parse_view(value: &str) -> Result<VisualMode, String> {
    VisualMode::parse(value).ok_or_else(|| format!("Unknown visual: {value}"))
}
#[derive(Clone, Debug, Parser)]
#[command(
    name = "omaMusi",
    version,
    about = "Play local audio with a live visualization and text-only playlist."
)]
pub struct Cli {
    /// Audio files or directories (default: current folder).
    pub paths: Vec<PathBuf>,
    /// Include subfolders when scanning directories.
    #[arg(short = 'r', long)]
    pub recursive: bool,
    /// Include the current folder alongside explicit paths (legacy -all also works).
    #[arg(long)]
    pub all: bool,
    /// Initial visualization; change it with V while playing.
    #[arg(long,default_value="warp",value_parser=parse_view)]
    pub view: VisualMode,
    /// Manage the native application-menu launcher.
    #[arg(long, value_enum)]
    pub desktop: Option<DesktopAction>,
    #[arg(skip)]
    pub empty_start: bool,
}
impl Cli {
    pub fn parse_from<I, T>(arguments: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut after_separator = false;
        let arguments = arguments.into_iter().map(|arg| {
            let arg = arg.into();
            if arg == "--" {
                after_separator = true;
            }
            if !after_separator && arg == "-all" {
                OsString::from("--all")
            } else {
                arg
            }
        });
        <Self as Parser>::try_parse_from(arguments)
    }
    pub fn startup_paths(&self, cwd: &Path) -> Vec<PathBuf> {
        if self.empty_start {
            return Vec::new();
        }
        let mut paths = if self.all || self.paths.is_empty() {
            vec![cwd.to_owned()]
        } else {
            Vec::new()
        };
        paths.extend(self.paths.iter().cloned());
        paths
    }
    pub fn scan_recursive(&self) -> bool {
        self.recursive || self.paths.is_empty()
    }
    pub fn for_music(music: Option<PathBuf>) -> Self {
        let empty_start = music.is_none();
        Self {
            paths: music.into_iter().collect(),
            recursive: true,
            all: false,
            view: VisualMode::EventHorizon,
            desktop: None,
            empty_start,
        }
    }
}
