//! Standalone entry point for production GTK player workflow checks.
fn main() -> std::process::ExitCode {
    use gtk::prelude::*;
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.get(1).is_some_and(|arg| arg == "--activate") {
        let app = gtk::Application::builder()
            .application_id(&arguments[2])
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        assert!(app.is_remote(), "No existing player to activate");
        assert_eq!(i32::from(app.run_with_args::<&str>(&[])), 0);
        return std::process::ExitCode::SUCCESS;
    }
    if arguments.get(1).is_some_and(|arg| arg == "--hold") {
        let app = gtk::Application::builder()
            .application_id(&arguments[2])
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        assert!(
            !app.is_remote(),
            "Crashed instance retained its registration"
        );
        let _hold = app.hold();
        app.connect_activate(|_| {});
        std::fs::write(&arguments[3], "ready").unwrap();
        app.run_with_args::<&str>(&[]);
        return std::process::ExitCode::SUCCESS;
    }
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| std::ffi::OsString::from("cargo"));
    let listed = std::process::Command::new(&cargo)
        .args([
            "test",
            "--lib",
            "ui::tests::desktop_workflows",
            "--",
            "--list",
            "--exact",
        ])
        .output();
    if !listed.is_ok_and(|output| {
        output.status.success()
            && String::from_utf8_lossy(&output.stdout)
                .contains("ui::tests::desktop_workflows: test")
    }) {
        eprintln!("Player checks missing or could not compile; refusing empty verification");
        return std::process::ExitCode::FAILURE;
    }
    match std::process::Command::new(cargo)
        .args([
            "test",
            "--lib",
            "ui::tests::desktop_workflows",
            "--",
            "--ignored",
            "--exact",
            "--test-threads=1",
            "--nocapture",
        ])
        .status()
    {
        Ok(status) if status.success() => std::process::ExitCode::SUCCESS,
        result => {
            eprintln!("Player verification failed: {result:?}");
            std::process::ExitCode::FAILURE
        }
    }
}
