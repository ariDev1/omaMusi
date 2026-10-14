use image::{Rgb, RgbImage};
use omamusi::artwork::load_artwork;
use std::{fs, process::Command};

#[test]
fn embedded_mp3_and_flac_artwork_precedes_sidecar() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.png");
    RgbImage::from_pixel(200, 100, Rgb([237, 99, 63]))
        .save(&source)
        .unwrap();
    RgbImage::from_pixel(80, 80, Rgb([0, 0, 255]))
        .save(root.path().join("cover.png"))
        .unwrap();
    for (extension, codec) in [("mp3", "libmp3lame"), ("flac", "flac")] {
        let path = root.path().join(format!("track.{extension}"));
        assert!(
            Command::new("ffmpeg")
                .args([
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=frequency=440:duration=0.2",
                    "-i"
                ])
                .arg(&source)
                .args([
                    "-map",
                    "0:a",
                    "-map",
                    "1:v",
                    "-c:a",
                    codec,
                    "-c:v",
                    "copy",
                    "-disposition:v",
                    "attached_pic"
                ])
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        let art = load_artwork(&path).unwrap().unwrap();
        assert_eq!(art.width, 200);
        assert_eq!(art.height, 100);
        assert_eq!(&art.rgba[..4], [237, 99, 63, 255]);
    }
}

#[test]
fn conventional_sidecars_fallback_and_casefold_work() {
    for extension in ["jpg", "jpeg", "png", "webp"] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("song.wav");
        fs::write(&path, b"").unwrap();
        fs::write(root.path().join("cover.jpg"), b"invalid image").unwrap();
        RgbImage::from_pixel(200, 100, Rgb([200, 80, 40]))
            .save(root.path().join(format!("FoLdEr.{extension}")))
            .unwrap();
        let art = load_artwork(&path).unwrap().unwrap();
        assert_eq!((art.width, art.height), (200, 100));
        assert!(art.rgba[0].abs_diff(200) < 4);
    }
}

#[test]
fn missing_artwork_is_empty_and_large_sidecars_downscale() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("missing.wav");
    assert!(load_artwork(&path).unwrap().is_none());
    RgbImage::from_pixel(2400, 1200, Rgb([255, 0, 0]))
        .save(root.path().join("cover.png"))
        .unwrap();
    let art = load_artwork(&path).unwrap().unwrap();
    assert_eq!((art.width, art.height), (1200, 600));
}

#[test]
fn oversized_compressed_sidecar_is_rejected_without_reading_whole_file() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("song.wav");
    let file = fs::File::create(root.path().join("cover.png")).unwrap();
    file.set_len(17 * 1024 * 1024).unwrap();
    RgbImage::from_pixel(30, 30, Rgb([0, 255, 0]))
        .save(root.path().join("front.png"))
        .unwrap();
    let art = load_artwork(&path).unwrap().unwrap();
    assert_eq!((art.width, art.height), (30, 30));
    assert_eq!(&art.rgba[..4], [0, 255, 0, 255]);
}

#[test]
fn bounded_capture_times_out_and_caps_output() {
    use omamusi::process::capture;
    let mut sleep = Command::new("sleep");
    sleep.arg("10");
    let start = std::time::Instant::now();
    assert!(capture(sleep, std::time::Duration::from_millis(40), 100).is_err());
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
    let mut output = Command::new("head");
    output.args(["-c", "10000", "/dev/zero"]);
    assert!(capture(output, std::time::Duration::from_secs(1), 100).is_err());
}

#[test]
fn jpeg_orientation_is_applied_before_cover_layout() {
    let root = tempfile::tempdir().unwrap();
    let cover = root.path().join("cover.jpg");
    RgbImage::from_pixel(200, 100, Rgb([200, 80, 40]))
        .save(&cover)
        .unwrap();
    let original = fs::read(&cover).unwrap();
    // EXIF little-endian TIFF: one SHORT orientation tag, value 6 (rotate 90°).
    let payload = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let mut encoded = original[..2].to_vec();
    encoded.extend_from_slice(&[0xff, 0xe1]);
    encoded.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    encoded.extend_from_slice(payload);
    encoded.extend_from_slice(&original[2..]);
    fs::write(&cover, encoded).unwrap();
    let artwork = load_artwork(&root.path().join("song.mp3"))
        .unwrap()
        .unwrap();
    assert_eq!((artwork.width, artwork.height), (100, 200));
}

#[test]
fn late_artwork_from_old_track_is_discarded_for_current_track() {
    use omamusi::background::BackgroundTasks;
    use std::{
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };
    let root = tempfile::tempdir().unwrap();
    let old = root.path().join("old");
    let current = root.path().join("current");
    fs::create_dir(&old).unwrap();
    fs::create_dir(&current).unwrap();
    RgbImage::from_pixel(10, 10, Rgb([255, 0, 0]))
        .save(old.join("cover.png"))
        .unwrap();
    RgbImage::from_pixel(10, 10, Rgb([0, 0, 255]))
        .save(current.join("cover.png"))
        .unwrap();
    let tasks = BackgroundTasks::new();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    tasks.submit("artwork", move || {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        load_artwork(&old.join("song.mp3"))
    });
    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    tasks.invalidate("artwork");
    let token = tasks
        .submit("artwork", move || load_artwork(&current.join("song.mp3")))
        .unwrap();
    release_tx.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let results = loop {
        let results = tasks.poll();
        if !results.is_empty() {
            break results;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].generation, token);
    assert_eq!(
        &results[0].result.as_ref().unwrap().as_ref().unwrap().rgba[..4],
        [0, 0, 255, 255]
    );
}
