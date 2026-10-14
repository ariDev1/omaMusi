//! Bounded embedded and album-folder images; audio files are never modified.
use crate::{library::casefold, process::capture};
use anyhow::Result;
use image::{DynamicImage, ImageDecoder, ImageReader, imageops::FilterType};
use std::{
    collections::HashMap,
    fs,
    io::{Cursor, Read},
    path::Path,
    process::Command,
    time::Duration,
};
#[derive(Clone, Debug)]
pub struct Artwork {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
impl Artwork {
    pub fn thumbnail(&self, size: u32) -> Option<Self> {
        if self.width == 0 || self.height == 0 || size == 0 {
            return None;
        }
        let image = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone())?;
        let image = DynamicImage::ImageRgba8(image)
            .resize(size, size, FilterType::Triangle)
            .to_rgba8();
        Some(Self {
            width: image.width(),
            height: image.height(),
            rgba: image.into_raw(),
        })
    }
}
fn decode(bytes: &[u8]) -> Option<Artwork> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().ok()?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || width as u64 * height as u64 > 64_000_000 {
        return None;
    }
    let orientation = decoder.orientation().ok()?;
    let mut image = DynamicImage::from_decoder(decoder).ok()?;
    image.apply_orientation(orientation);
    if image.width().max(image.height()) > 1200 {
        image = image.resize(1200, 1200, FilterType::Triangle);
    }
    let image = image.to_rgba8();
    Some(Artwork {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}
fn sidecar(path: &Path) -> Option<Artwork> {
    const LIMIT: u64 = 16 * 1024 * 1024;
    let file = fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > LIMIT {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > LIMIT {
        return None;
    }
    decode(&bytes)
}
pub fn load_artwork(path: &Path) -> Result<Option<Artwork>> {
    let mut command = Command::new("ffprobe");
    command
        .args([
            "-v",
            "error",
            "-select_streams",
            "v",
            "-show_entries",
            "stream=index:stream_disposition=attached_pic",
            "-of",
            "json",
        ])
        .arg(path);
    if let Ok(result) = capture(command, Duration::from_secs(5), 1024 * 1024)
        && let Ok(data) = serde_json::from_slice::<serde_json::Value>(&result.stdout)
        && let Some(streams) = data["streams"].as_array()
    {
        for stream in streams {
            if stream["disposition"]["attached_pic"].as_i64() != Some(1) {
                continue;
            }
            let Some(index) = stream["index"].as_u64() else {
                continue;
            };
            let mut command = Command::new("ffmpeg");
            command
                .args(["-nostdin", "-v", "error", "-max_alloc", "67108864", "-i"])
                .arg(path)
                .args([
                    "-map",
                    &format!("0:{index}"),
                    "-frames:v",
                    "1",
                    "-vf",
                    "scale=w='min(1200,iw)':h='min(1200,ih)':force_original_aspect_ratio=decrease",
                    "-c:v",
                    "png",
                    "-threads",
                    "1",
                    "-f",
                    "image2pipe",
                    "pipe:1",
                ]);
            if let Ok(output) = capture(command, Duration::from_secs(5), 8 * 1024 * 1024)
                && let Some(image) = decode(&output.stdout)
            {
                return Ok(Some(image));
            }
        }
    }
    let parent = path.parent().unwrap_or(Path::new("."));
    let files: HashMap<_, _> = fs::read_dir(parent)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| (casefold(&entry.file_name().to_string_lossy()), entry.path()))
        .collect();
    for name in ["cover", "folder", "front", "album"] {
        for extension in ["jpg", "jpeg", "png", "webp"] {
            if let Some(path) = files.get(&format!("{name}.{extension}"))
                && let Some(image) = sidecar(path)
            {
                return Ok(Some(image));
            }
        }
    }
    Ok(None)
}
