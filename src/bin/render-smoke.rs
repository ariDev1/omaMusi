//! Standalone entry point for production-renderer GPU checks.
fn main() -> std::process::ExitCode {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| std::ffi::OsString::from("cargo"));
    let listed = std::process::Command::new(&cargo)
        .args([
            "test",
            "--lib",
            "render::gl::tests::hardware_rendering",
            "--",
            "--list",
            "--exact",
        ])
        .output();
    if !listed.is_ok_and(|output| {
        output.status.success()
            && String::from_utf8_lossy(&output.stdout)
                .contains("render::gl::tests::hardware_rendering: test")
    }) {
        eprintln!("GPU check is missing or could not compile; refusing an empty verification run");
        return std::process::ExitCode::FAILURE;
    }
    match std::process::Command::new(cargo)
        .args([
            "test",
            "--lib",
            "render::gl::tests::hardware_rendering",
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
            eprintln!("GPU verification failed: {result:?}");
            std::process::ExitCode::FAILURE
        }
    }
}
