use omamusi::audio::{AudioEvent, AudioPlayer, decoder::Decoder, probe};
use std::{
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn tone(root: &Path, duration: f64, rate: u32) -> PathBuf {
    let path = root.join(format!("tone {rate} $(echo untouched).wav"));
    assert!(
        Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                &format!("sine=frequency=440:sample_rate={rate}:duration={duration}"),
                "-metadata",
                "title=Test tone",
                "-c:a",
                "pcm_s24le"
            ])
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    path
}
#[track_caller]
fn wait(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !predicate() {
        assert!(Instant::now() < deadline, "timed out");
        thread::sleep(Duration::from_millis(5));
    }
}
fn decode(path: &Path, offset: f64, rate: u32) -> Vec<f32> {
    let mut decoder = Decoder::spawn(path, offset, rate, 2).unwrap();
    let mut pcm = decoder.take_consumer().unwrap();
    let mut samples = Vec::new();
    wait(|| {
        while let Ok(sample) = pcm.pop() {
            samples.push(sample);
        }
        decoder.done() && pcm.is_empty()
    });
    assert!(decoder.error().is_none());
    decoder.close();
    samples
}

#[test]
fn decoder_preserves_duration_stereo_seek_and_metadata() {
    let root = tempfile::tempdir().unwrap();
    let path = tone(root.path(), 1.2, 48000);
    let samples = decode(&path, 0.0, 48000);
    assert!((samples.len() as f64 / 96000.0 - 1.2).abs() < 0.002);
    assert!(samples.iter().any(|s| s.abs() > 0.05));
    assert!(samples.as_chunks::<2>().0.iter().all(|p| p[0] == p[1]));
    assert!((decode(&path, 0.7, 48000).len() as f64 / 96000.0 - 0.5).abs() < 0.002);
    assert_eq!(probe(&path).unwrap().title, "Test tone");
}
#[test]
fn floating_decoder_retains_96khz_source() {
    let root = tempfile::tempdir().unwrap();
    let path = tone(root.path(), 0.3, 96000);
    assert_eq!(probe(&path).unwrap().sample_rate, 96000);
    assert!((decode(&path, 0.0, 96000).len() as f64 / 192000.0 - 0.3).abs() < 0.002);
}
#[test]
fn malformed_media_reports_error_and_full_queue_cancellation_reaps_child() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("broken.mp3");
    std::fs::write(&path, b"not music").unwrap();
    let mut broken = Decoder::spawn(&path, 0.0, 48000, 2).unwrap();
    wait(|| broken.done());
    assert!(broken.error().is_some());
    broken.close();
    let path = tone(root.path(), 10.0, 48000);
    let mut decoder = Decoder::spawn(&path, 0.0, 48000, 2).unwrap();
    let pid = decoder.process_id();
    thread::sleep(Duration::from_millis(100));
    let start = Instant::now();
    decoder.close();
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(!PathBuf::from(format!("/proc/{pid}")).exists());
}
#[test]
fn probe_rejects_missing_files_and_decoder_rejects_invalid_parameters() {
    assert!(probe(Path::new("/missing/track.mp3")).is_err());
    assert!(Decoder::spawn(Path::new("/missing"), f64::NAN, 48000, 2).is_err());
    assert!(Decoder::spawn(Path::new("/missing"), 0.0, 0, 2).is_err());
}

#[test]
#[ignore = "requires desktop or virtual audio output; run with --include-ignored"]
fn native_audio_pause_seek_volume_and_analysis() {
    let root = tempfile::tempdir().unwrap();
    let path = tone(root.path(), 3.0, 96000);
    let mut player = AudioPlayer::new().unwrap();
    player.set_volume(0.0);
    player.play(&path, 0.0, 96000, 41, false).unwrap();
    wait(|| player.snapshot().position > 0.08);
    assert!(player.samples().samples.iter().any(|s| s.abs() > 0.02));
    assert_eq!(player.samples().sample_rate, player.snapshot().sample_rate);
    player.pause(true);
    let paused = player.snapshot().position;
    thread::sleep(Duration::from_millis(120));
    assert!((player.snapshot().position - paused).abs() < 0.04);
    player.pause(false);
    wait(|| player.snapshot().position > paused + 0.06);
    player.seek(1.5).unwrap();
    wait(|| player.snapshot().position > 1.5);
    assert_eq!(player.snapshot().generation, 41);
    player.set_volume(0.2);
    assert!((player.snapshot().volume - 0.2).abs() < 0.001);
    player.set_volume(0.0);
    let start = Instant::now();
    player.stop();
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(player.poll_events().is_empty());
    assert_eq!(player.snapshot().position, 0.0);
}
#[test]
#[ignore = "requires desktop or virtual audio output; run with --include-ignored"]
fn native_audio_finished_once_and_old_events_discarded_on_replay() {
    let root = tempfile::tempdir().unwrap();
    let path = tone(root.path(), 0.25, 48000);
    let mut player = AudioPlayer::new().unwrap();
    player.set_volume(0.0);
    player.play(&path, 0.0, 48000, 7, false).unwrap();
    let mut events = Vec::new();
    wait(|| {
        events.extend(player.poll_events());
        !events.is_empty()
    });
    assert_eq!(events, [AudioEvent::Finished(7)]);
    assert!((player.snapshot().position - 0.25).abs() < 0.02);
    assert!(player.poll_events().is_empty());
    player.play(&path, 0.0, 48000, 8, true).unwrap();
    thread::sleep(Duration::from_millis(60));
    assert!(player.poll_events().is_empty());
    player.stop();
    assert!(player.poll_events().is_empty());
}
#[test]
#[ignore = "requires desktop or virtual audio output; run with --include-ignored"]
fn native_audio_rapid_seeks_and_unsupported_rate_fallback() {
    let root = tempfile::tempdir().unwrap();
    let path = tone(root.path(), 3.0, 48000);
    let mut player = AudioPlayer::new().unwrap();
    player.set_volume(0.0);
    player.play(&path, 0.0, 48000, 13, false).unwrap();
    for offset in [0.2, 0.8, 0.1, 1.0, 0.3] {
        player.seek(offset).unwrap();
    }
    wait(|| player.snapshot().position > 0.3);
    assert!(player.poll_events().is_empty());
    player.play(&path, 0.0, 1, 14, false).unwrap();
    assert!(player.snapshot().sample_rate >= 8000);
    wait(|| player.snapshot().position > 0.02);
}

#[test]
#[ignore = "requires desktop or virtual audio output; run with --include-ignored"]
fn native_audio_corrupt_media_emits_failure_once_and_can_recover() {
    let root = tempfile::tempdir().unwrap();
    let broken = root.path().join("broken.mp3");
    std::fs::write(&broken, b"garbage").unwrap();
    let mut player = AudioPlayer::new().unwrap();
    player.set_volume(0.0);
    player.play(&broken, 0.0, 48000, 50, false).unwrap();
    let mut events = Vec::new();
    wait(|| {
        events.extend(player.poll_events());
        !events.is_empty()
    });
    assert!(matches!(&events[..], [AudioEvent::Failed(50, message)] if !message.is_empty()));
    assert!(player.poll_events().is_empty());
    let valid = tone(root.path(), 0.5, 48000);
    player.play(&valid, 0.0, 48000, 51, false).unwrap();
    wait(|| player.snapshot().position > 0.02);
    assert!(player.snapshot().error.is_none());
}

#[test]
#[ignore = "requires PulseAudio-compatible service and pactl; run with --include-ignored"]
fn native_audio_removed_device_is_reported_without_starting_decoder() {
    use cpal::traits::HostTrait;
    struct Sink(String);
    impl Drop for Sink {
        fn drop(&mut self) {
            let _ = Command::new("pactl")
                .args(["unload-module", &self.0])
                .output();
        }
    }
    let name = format!("omamusi_test_{}", std::process::id());
    let created = Command::new("pactl")
        .args([
            "load-module",
            "module-null-sink",
            &format!("sink_name={name}"),
            &format!("sink_properties=device.description={name}"),
        ])
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let sink = Sink(String::from_utf8(created.stdout).unwrap().trim().to_owned());
    let host = cpal::host_from_id(cpal::HostId::PulseAudio).unwrap();
    let device = host
        .output_devices()
        .unwrap()
        .find(|d| d.to_string().contains(&name))
        .unwrap();
    drop(sink);
    let root = tempfile::tempdir().unwrap();
    let path = tone(root.path(), 0.5, 48000);
    let mut player = AudioPlayer::with_device(device).unwrap();
    player.set_volume(0.0);
    assert!(player.play(&path, 0.0, 48000, 3, false).is_err());
    assert!(!player.snapshot().has_stream);
    assert!(player.poll_events().is_empty());
}
