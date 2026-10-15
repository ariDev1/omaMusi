use omamusi::{
    artwork::Artwork,
    audio::PcmSnapshot,
    gallery::{CoverGallery, Rect},
    render::{Scene, cpu::CpuRenderer},
    visual::{VisualMode, VisualState},
};
use std::collections::HashSet;

fn pixels(mut surface: cairo::ImageSurface) -> Vec<[u8; 4]> {
    surface.flush();
    let stride = surface.stride() as usize;
    let width = surface.width() as usize;
    let height = surface.height() as usize;
    let data = surface.data().unwrap();
    let mut pixels = Vec::new();
    for row in 0..height {
        for column in 0..width {
            let offset = row * stride + column * 4;
            let pixel = u32::from_ne_bytes(data[offset..offset + 4].try_into().unwrap());
            pixels.push([
                ((pixel >> 16) & 255) as u8,
                ((pixel >> 8) & 255) as u8,
                (pixel & 255) as u8,
                (pixel >> 24) as u8,
            ]);
        }
    }
    pixels
}
fn artwork(color: [u8; 4]) -> Artwork {
    Artwork {
        width: 200,
        height: 100,
        rgba: color.repeat(200 * 100),
    }
}
fn draw(
    visual: &VisualState,
    cover: Option<&Artwork>,
    gallery: &mut CoverGallery,
    width: i32,
    height: i32,
) -> Vec<[u8; 4]> {
    let mut scene = Scene {
        frame: visual.frame(),
        wave: &visual.waveform,
        cover,
        gallery,
        gallery_bounds: Rect {
            x: width as f64 * 0.38,
            y: 50.0,
            width: width as f64 * 0.58,
            height: (height as f64 * 0.6).max(40.0),
        },
    };
    let surface = CpuRenderer::surface(&mut scene, width, height).unwrap();
    if let Some(directory) = std::env::var_os("OMAMUSI_RENDER_ARTIFACTS") {
        std::fs::create_dir_all(&directory).unwrap();
        let path = std::path::PathBuf::from(directory)
            .join(format!("{:?}-{width}x{height}.png", visual.frame().mode));
        surface
            .write_to_png(&mut std::fs::File::create(path).unwrap())
            .unwrap();
    }
    pixels(surface)
}

#[test]
fn all_nine_software_visuals_render_distinct_frames_without_mutating_audio() {
    let mut visual = VisualState::new();
    visual.mode = VisualMode::Waveform;
    let snapshot = PcmSnapshot {
        samples: std::array::from_fn(|i| (0.2 * (i as f64 * 0.12).sin()) as f32),
        sample_rate: 48000,
        revision: 1,
        generation: 1,
    };
    visual.feed(&snapshot);
    visual.tick(0.016, true);
    let original = snapshot.samples;
    let cover = artwork([237, 99, 63, 255]);
    let mut gallery = CoverGallery::new();
    gallery.add("/red.mp3".into(), cover.clone());
    gallery.add("/blue.mp3".into(), artwork([0, 0, 255, 255]));
    let mut hashes = HashSet::new();
    for mode in VisualMode::ALL {
        visual.mode = mode;
        visual.tick(0.016, true);
        let pixels = draw(&visual, Some(&cover), &mut gallery, 640, 420);
        assert_eq!(pixels.len(), 640 * 420);
        let digest = sha2::Sha256::digest(
            pixels
                .iter()
                .flat_map(|p| p.iter().copied())
                .collect::<Vec<_>>(),
        );
        hashes.insert(digest);
        if mode != VisualMode::Waveform {
            assert!(pixels.iter().all(|p| p[3] == 255), "{} alpha", mode.name());
        }
    }
    assert_eq!(hashes.len(), 9);
    assert_eq!(snapshot.samples, original);
}

#[test]
fn cover_stage_is_black_when_empty_and_cover_stays_on_right() {
    let mut visual = VisualState::new();
    visual.mode = VisualMode::CoverArt;
    visual.tick(0.016, false);
    let mut gallery = CoverGallery::new();
    assert!(
        draw(&visual, None, &mut gallery, 640, 420)
            .iter()
            .all(|p| *p == [0, 0, 0, 255])
    );
    let cover = artwork([255, 0, 0, 255]);
    for (width, height) in [(640, 420), (1280, 360), (420, 700)] {
        let pixels = draw(&visual, Some(&cover), &mut gallery, width, height);
        let red: Vec<_> = pixels
            .iter()
            .enumerate()
            .filter(|(_, p)| p[0] > 220 && p[1] < 20)
            .map(|(index, _)| (index % width as usize, index / width as usize))
            .collect();
        assert!(!red.is_empty());
        assert!(red.iter().all(|(x, _)| *x > width as usize / 2));
        let minx = red.iter().map(|p| p.0).min().unwrap();
        let maxx = red.iter().map(|p| p.0).max().unwrap();
        let miny = red.iter().map(|p| p.1).min().unwrap();
        let maxy = red.iter().map(|p| p.1).max().unwrap();
        assert!(((maxx - minx) as f64 / (maxy - miny) as f64 - 2.0).abs() < 0.06);
    }
}

#[test]
fn waveform_has_transparent_pixels_and_retains_color_between_audio_updates() {
    let mut visual = VisualState::new();
    visual.mode = VisualMode::Waveform;
    visual.feed(&PcmSnapshot {
        samples: std::array::from_fn(|i| (0.2 * (i as f64 * 0.1).sin()) as f32),
        sample_rate: 48000,
        revision: 1,
        generation: 1,
    });
    visual.tick(0.016, true);
    let mut gallery = CoverGallery::new();
    let first = draw(&visual, None, &mut gallery, 640, 420);
    assert_eq!(first[0][3], 0);
    assert!(first.iter().any(|p| p[3] > 0 && p[2] > p[0]));
    visual.tick(0.1, false);
    let paused = draw(&visual, None, &mut gallery, 640, 420);
    assert!(paused.iter().any(|p| p[3] > 0));
    assert!(
        paused.iter().map(|p| p[3] as u64).sum::<u64>()
            < first.iter().map(|p| p[3] as u64).sum::<u64>()
    );
}

#[test]
fn gallery_midpoint_crossfade_preserves_brightness_and_size() {
    let mut visual = VisualState::new();
    visual.mode = VisualMode::CoverGallery;
    visual.tick(0.016, false);
    let mut gallery = CoverGallery::new();
    gallery.add("/red.mp3".into(), artwork([255, 0, 0, 255]));
    gallery.add("/blue.mp3".into(), artwork([0, 0, 255, 255]));
    let area = Rect {
        x: 0.0,
        y: 0.0,
        width: 300.0,
        height: 300.0,
    };
    let rectangle = gallery.layout(area)[0];
    gallery.tick(5.0);
    gallery.tiles[0].fade = 0.5;
    let mut scene = Scene {
        frame: visual.frame(),
        wave: &visual.waveform,
        cover: None,
        gallery: &mut gallery,
        gallery_bounds: area,
    };
    let pixels = pixels(CpuRenderer::surface(&mut scene, 300, 300).unwrap());
    let (x, y) = rectangle.center();
    let midpoint = pixels[y as usize * 300 + x as usize];
    assert!(midpoint[0].abs_diff(128) <= 2);
    assert!(midpoint[2].abs_diff(128) <= 2);
    assert_eq!(midpoint[3], 255);
}

#[test]
fn particle_dance_renders_dense_multicolor_streams_without_whiteout() {
    // No torus/ring: the visual is a free-flowing multicolor stream field.
    // Feed sustained hot input first: a fresh quiet frame leaves the
    // spread-out streams too dim to read.
    let mut visual = VisualState::new();
    visual.mode = VisualMode::ParticleDance;
    for index in 0..300 {
        visual.feed(&PcmSnapshot {
            samples: std::array::from_fn(|i| (0.65 * (i as f64 * 0.12).sin()) as f32),
            sample_rate: 48000,
            revision: index + 1,
            generation: 1,
        });
        visual.tick(1.0 / 30.0, true);
    }
    let mut gallery = CoverGallery::new();
    let pixels = draw(&visual, None, &mut gallery, 960, 640);
    let saturated: Vec<(usize, &[u8; 4])> = pixels
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            let max = *[p[0], p[1], p[2]].iter().max().unwrap();
            let min = *[p[0], p[1], p[2]].iter().min().unwrap();
            max > 30 && max - min > 25
        })
        .collect();
    let cool = saturated
        .iter()
        .filter(|(_, p)| p[1] > 20 && p[2] > 20)
        .count();
    let warm = saturated
        .iter()
        .filter(|(_, p)| p[0] > 25 && p[1] > 15 && p[1] > p[2])
        .count();
    let violet = saturated
        .iter()
        .filter(|(_, p)| p[0] > 15 && p[2] > 25 && p[1] < p[2])
        .count();
    eprintln!(
        "streams: total={} cool={cool} warm={warm} violet={violet}",
        saturated.len()
    );
    assert!(
        saturated.len() > 4000,
        "particle streams too faint: {} visible pixels",
        saturated.len()
    );
    // Color flow: no single hue dominates. Cool, warm amber, and
    // violet/magenta families must all be present.
    assert!(cool > 1000, "cool stream colors missing: {cool}");
    assert!(warm > 300, "warm stream colors missing: {warm}");
    assert!(
        violet > 1000,
        "violet/magenta stream colors missing: {violet}"
    );
    let min_x = saturated.iter().map(|(i, _)| i % 960).min().unwrap();
    let max_x = saturated.iter().map(|(i, _)| i % 960).max().unwrap();
    let min_y = saturated.iter().map(|(i, _)| i / 960).min().unwrap();
    let max_y = saturated.iter().map(|(i, _)| i / 960).max().unwrap();
    assert!(
        max_x - min_x > 250 && max_y - min_y > 200,
        "streams collapsed instead of filling the center"
    );
    assert!(
        pixels
            .iter()
            .filter(|p| p[0] > 245 && p[1] > 245 && p[2] > 245)
            .count()
            < 100
    );
}

#[test]
fn invalid_or_tiny_surface_sizes_are_handled() {
    let visual = VisualState::new();
    let mut gallery = CoverGallery::new();
    assert_eq!(draw(&visual, None, &mut gallery, 1, 1).len(), 1);
    let mut scene = Scene {
        frame: visual.frame(),
        wave: &visual.waveform,
        cover: None,
        gallery: &mut gallery,
        gallery_bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
    };
    assert!(CpuRenderer::surface(&mut scene, 0, 0).is_err());
}

#[test]
fn software_particle_geometry_keeps_small_dots_under_maximum_input() {
    use omamusi::render::cpu::particle_points;
    let visual = VisualState::new();
    let mut frame = visual.frame().clone();
    frame.time = 12.0;
    let quiet = particle_points(&frame, 960.0, 640.0);
    frame.energy = 1.0;
    frame.bass = 1.0;
    frame.treble = 1.0;
    frame.phi_impulse = 1.0;
    frame.phi_event = 1.0;
    frame.aether_onset = 1.0;
    frame.aether_beat_pulse = 1.0;
    frame.aether_density = 1.0;
    frame.bands.fill(1.0);
    let loud = particle_points(&frame, 960.0, 640.0);
    assert_eq!(quiet.len(), 2400);
    assert_eq!(
        quiet.iter().map(|p| p.radius).collect::<Vec<_>>(),
        loud.iter().map(|p| p.radius).collect::<Vec<_>>()
    );
    assert!(loud.iter().all(|p| p.radius <= 1.5));
    assert!(
        quiet
            .iter()
            .zip(&loud)
            .any(|(a, b)| (a.x - b.x).abs() > 1.0)
    );
}

use sha2::Digest;
