#![allow(
    non_snake_case,
    reason = "Preserve the established omaMusi executable name"
)]
use anyhow::Result;
use omamusi::{
    cli::{Cli, DesktopAction},
    desktop::Desktop,
};
fn run() -> Result<()> {
    let cli = Cli::parse_from(std::env::args_os()).unwrap_or_else(|error| error.exit());
    match cli.desktop {
        Some(DesktopAction::Install) => {
            println!("Installed {}", Desktop::current()?.install()?.display());
            Ok(())
        }
        Some(DesktopAction::Remove) => {
            Desktop::current()?.remove()?;
            println!("Removed this installation's menu entry. Music and playlists are kept.");
            Ok(())
        }
        Some(DesktopAction::Launch) => omamusi::ui::run(omamusi::desktop::music_cli()),
        None => omamusi::ui::run(cli),
    }
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("omaMusi: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
