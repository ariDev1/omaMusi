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
    if cli.install {
        omamusi::install::check_dependencies()?;
        omamusi::install::Installation::current().install(&std::env::current_exe()?)?;
        println!("Installed omaMusi for this user.");
        return Ok(());
    }
    if cli.uninstall {
        omamusi::install::Installation::current().remove()?;
        println!("Removed managed player installation. Music and playlists are kept.");
        return Ok(());
    }
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
