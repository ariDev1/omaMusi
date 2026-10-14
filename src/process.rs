//! Bounded subprocess capture for metadata and artwork workers.
use anyhow::{Context, Result, bail};
use std::{
    io::Read,
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn capture(mut command: Command, timeout: Duration, limit: usize) -> Result<Output> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    fn reader(mut pipe: impl Read, limit: usize) -> std::io::Result<(Vec<u8>, bool)> {
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 4096];
        let mut oversized = false;
        loop {
            let count = pipe.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            let remaining = limit.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..count.min(remaining)]);
            oversized |= count > remaining;
        }
        Ok((bytes, oversized))
    }
    let out = thread::spawn(move || reader(stdout, limit));
    let err = thread::spawn(move || reader(stderr, 8192));
    let deadline = Instant::now() + timeout;
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(anyhow::anyhow!("Process timed out"));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(e.into());
            }
        }
    };
    let stdout = out
        .join()
        .map_err(|_| anyhow::anyhow!("Output reader panicked"))??;
    let stderr = err
        .join()
        .map_err(|_| anyhow::anyhow!("Error reader panicked"))??;
    let status = result?;
    if stdout.1 {
        bail!("Process output exceeds {limit} bytes");
    }
    if !status.success() {
        bail!("{}", String::from_utf8_lossy(&stderr.0));
    }
    Ok(Output {
        status,
        stdout: stdout.0,
        stderr: stderr.0,
    })
}

pub fn ffprobe(path: &std::path::Path) -> Result<serde_json::Value> {
    let mut command = Command::new("ffprobe");
    command
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(path);
    serde_json::from_slice(&capture(command, Duration::from_secs(10), 1024 * 1024)?.stdout)
        .context("Invalid metadata response")
}
