use super::*;
use crate::{
    artwork::Artwork,
    audio::PcmSnapshot,
    gallery::{CoverGallery, Rect},
    visual::{VisualMode, VisualState},
};
use gtk::{gdk, prelude::*};

#[test]
#[ignore = "requires a desktop OpenGL context; cargo run --bin render-smoke"]
fn hardware_rendering() {
    gtk::init().unwrap();
    let display = gdk::Display::default().unwrap();
    let context = display.create_gl_context().unwrap();
    context.set_allowed_apis(gdk::GLAPI::GL);
    context.set_required_version(3, 3);
    context.realize().unwrap();
    context.make_current();
    let mut gpu = GlRenderer::new(&context).unwrap();
    eprintln!("GPU renderer: {}", gpu.description());
    let (width, height) = (640, 420);
    let mut fbo = 0;
    let mut texture = 0;
    unsafe {
        api::GenFramebuffers(1, &mut fbo);
        api::BindFramebuffer(api::FRAMEBUFFER, fbo);
        api::GenTextures(1, &mut texture);
        api::BindTexture(api::TEXTURE_2D, texture);
        api::TexImage2D(
            api::TEXTURE_2D,
            0,
            api::RGBA8 as i32,
            width,
            height,
            0,
            api::RGBA,
            api::UNSIGNED_BYTE,
            std::ptr::null(),
        );
        api::FramebufferTexture2D(
            api::FRAMEBUFFER,
            api::COLOR_ATTACHMENT0,
            api::TEXTURE_2D,
            texture,
            0,
        );
        assert_eq!(
            api::CheckFramebufferStatus(api::FRAMEBUFFER),
            api::FRAMEBUFFER_COMPLETE
        );
    }
    let mut visual = VisualState::new();
    visual.mode = VisualMode::Waveform;
    visual.feed(&PcmSnapshot {
        samples: std::array::from_fn(|i| (0.2 * (i as f64 * 0.12).sin()) as f32),
        sample_rate: 48000,
        revision: 1,
        generation: 1,
    });
    visual.tick(0.016, true);
    let cover = Artwork {
        width: 200,
        height: 100,
        rgba: [237, 99, 63, 255].repeat(200 * 100),
    };
    let mut gallery = CoverGallery::new();
    gallery.add("/one.mp3".into(), cover.clone());
    gallery.add(
        "/two.mp3".into(),
        Artwork {
            width: 200,
            height: 100,
            rgba: [0, 0, 255, 255].repeat(200 * 100),
        },
    );
    let mut frame = visual.frame().clone();
    let read = || {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        unsafe {
            api::ReadPixels(
                0,
                0,
                width,
                height,
                api::RGBA,
                api::UNSIGNED_BYTE,
                pixels.as_mut_ptr().cast(),
            );
        }
        pixels
    };
    let mut seen = std::collections::HashSet::new();
    for mode in VisualMode::ALL {
        frame.mode = mode;
        let mut scene = Scene {
            frame: &frame,
            wave: &visual.waveform,
            cover: Some(&cover),
            gallery: &mut gallery,
            gallery_bounds: Rect {
                x: 250.0,
                y: 40.0,
                width: 360.0,
                height: 280.0,
            },
        };
        gpu.draw(&mut scene, width, height, 1.0, 0.016).unwrap();
        let pixels = read();
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[0] > 0 || p[1] > 0 || p[2] > 0),
            "{} empty",
            mode.name()
        );
        seen.insert(pixels);
    }
    assert_eq!(seen.len(), VisualMode::ALL.len());
    frame.mode = VisualMode::Warp;
    let mut scene = Scene {
        frame: &frame,
        wave: &visual.waveform,
        cover: None,
        gallery: &mut gallery,
        gallery_bounds: Rect {
            x: 250.0,
            y: 40.0,
            width: 360.0,
            height: 280.0,
        },
    };
    gpu.draw(&mut scene, width, height, 1.0, 0.016).unwrap();
    let original = read();
    for index in 0..64 {
        frame.mode = VisualMode::ALL[index % VisualMode::ALL.len()];
        let mut scene = Scene {
            frame: &frame,
            wave: &visual.waveform,
            cover: None,
            gallery: &mut gallery,
            gallery_bounds: Rect {
                x: 250.0,
                y: 40.0,
                width: 360.0,
                height: 280.0,
            },
        };
        gpu.draw(&mut scene, width, height, 1.0, 0.016).unwrap();
    }
    frame.mode = VisualMode::Warp;
    let mut scene = Scene {
        frame: &frame,
        wave: &visual.waveform,
        cover: None,
        gallery: &mut gallery,
        gallery_bounds: Rect {
            x: 250.0,
            y: 40.0,
            width: 360.0,
            height: 280.0,
        },
    };
    gpu.draw(&mut scene, width, height, 1.0, 0.016).unwrap();
    assert_eq!(read(), original);
    frame.mode = VisualMode::CoverArt;
    let mut scene = Scene {
        frame: &frame,
        wave: &visual.waveform,
        cover: None,
        gallery: &mut gallery,
        gallery_bounds: Rect {
            x: 250.0,
            y: 40.0,
            width: 360.0,
            height: 280.0,
        },
    };
    gpu.draw(&mut scene, width, height, 1.0, 0.016).unwrap();
    assert!(
        read()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [0, 0, 0, 255])
    );
    visual.mode = VisualMode::Waveform;
    visual.feed(&PcmSnapshot {
        samples: std::array::from_fn(|i| (0.2 * (i as f64 * 0.1).sin()) as f32),
        sample_rate: 48000,
        revision: 2,
        generation: 1,
    });
    visual.tick(0.016, true);
    let mut scene = Scene {
        frame: visual.frame(),
        wave: &visual.waveform,
        cover: None,
        gallery: &mut gallery,
        gallery_bounds: Rect {
            x: 250.0,
            y: 40.0,
            width: 360.0,
            height: 280.0,
        },
    };
    gpu.draw(&mut scene, width, height, 1.0, 0.016).unwrap();
    let wave = read();
    assert!(wave.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
    assert!(
        wave.as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] > 0 && p[2] > p[0])
    );
    // Match the original particle color regression: a black stage and maximum input.
    frame.mode = VisualMode::ParticleDance;
    frame.colors[0] = [0.0; 3];
    frame.energy = 1.0;
    frame.bass = 1.0;
    frame.treble = 1.0;
    frame.aether_onset = 1.0;
    frame.aether_beat_pulse = 1.0;
    frame.phi_event = 1.0;
    frame.phi_impulse = 1.0;
    frame.aether_density = 1.0;
    frame.aether_beat_confidence = 1.0;
    frame.bands.fill(1.0);
    frame.phi_velocity = 2.6;
    for index in 0..240 {
        frame.time = 12.0 + index as f64 / 30.0 * 4.5;
        frame.aether_world_turn = index as f64 / 30.0 * 0.12;
        frame.aether_beat_phase = (index as f64 / 30.0 * 1.8) % 1.0;
        let mut scene = Scene {
            frame: &frame,
            wave: &visual.waveform,
            cover: None,
            gallery: &mut gallery,
            gallery_bounds: Rect {
                x: 250.0,
                y: 40.0,
                width: 360.0,
                height: 280.0,
            },
        };
        gpu.draw(&mut scene, width, height, 1.0, 1.0 / 30.0)
            .unwrap();
    }
    let settled = read();
    save_particle_artifact("ParticleDance-gpu-maximum.png", &settled, width, height);
    let saturated: Vec<_> = settled
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            let max = *[p[0], p[1], p[2]].iter().max().unwrap();
            let min = *[p[0], p[1], p[2]].iter().min().unwrap();
            max > 15 && max - min > 12
        })
        .map(|(index, _)| (index % width as usize, index / width as usize))
        .collect();
    assert!(
        saturated.len() as f64 / (width * height) as f64 > 0.01,
        "particle streams disappeared"
    );
    let edges = saturated
        .iter()
        .filter(|(x, y)| {
            *x < width as usize / 20
                || *x > width as usize * 19 / 20
                || *y < height as usize / 20
                || *y > height as usize * 19 / 20
        })
        .count();
    assert!(
        (edges as f64 / saturated.len() as f64) > 0.05,
        "streams do not reach the frame edges"
    );
    assert!(
        (settled
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[..3].iter().all(|c| *c > 220))
            .count() as f64
            / (width * height) as f64)
            < 0.001,
        "sustained white-out"
    );
    // Exercise production analyzer uniforms after sustained energetic input.
    // Pixel coverage above a readable saturation threshold catches near-invisible
    // streams that a nonblack-pixel assertion would miss.
    // Reset the swarm to its deterministic initial seeds so this run does
    // not inherit the maximum run's end positions. Tile up to the live
    // swarm count like the renderer does.
    let raw = include_bytes!("../../assets/swarm-seeds.bin");
    let mut tiled: Vec<u8> = Vec::with_capacity(gpu.swarm_count as usize * 24);
    while tiled.len() < gpu.swarm_count as usize * 24 {
        let take = (gpu.swarm_count as usize * 24 - tiled.len()).min(raw.len());
        tiled.extend_from_slice(&raw[..take]);
    }
    for buffer in gpu.swarm_buffers {
        unsafe {
            api::BindBuffer(api::ARRAY_BUFFER, buffer);
            api::BufferData(
                api::ARRAY_BUFFER,
                tiled.len() as isize,
                tiled.as_ptr().cast(),
                api::DYNAMIC_COPY,
            );
        }
    }
    visual.mode = VisualMode::ParticleDance;
    // Sample stream presence through the run: dense multicolor streams must
    // stay in view the whole time, not just in the final frame.
    let mut fewest_stream = usize::MAX;
    for index in 0..600 {
        visual.feed(&PcmSnapshot {
            samples: std::array::from_fn(|i| (0.65 * (i as f64 * 0.12).sin()) as f32),
            sample_rate: 48000,
            revision: index + 3,
            generation: 1,
        });
        visual.tick(1.0 / 30.0, true);
        let mut scene = Scene {
            frame: visual.frame(),
            wave: &visual.waveform,
            cover: None,
            gallery: &mut gallery,
            gallery_bounds: Rect {
                x: 250.0,
                y: 40.0,
                width: 360.0,
                height: 280.0,
            },
        };
        gpu.draw(&mut scene, width, height, 1.0, 1.0 / 30.0)
            .unwrap();
        if index % 100 == 99 {
            let sample = read();
            let count = sample
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| {
                    let max = *[p[0], p[1], p[2]].iter().max().unwrap();
                    let min = *[p[0], p[1], p[2]].iter().min().unwrap();
                    max > 30 && max - min > 20
                })
                .count();
            eprintln!("Particle Dance live frame {index}: {count} stream pixels");
            fewest_stream = fewest_stream.min(count);
        }
    }
    let live = read();
    save_particle_artifact("ParticleDance-gpu-analyzed.png", &live, width, height);
    assert!(
        fewest_stream > 200,
        "particle streams left the frame mid-run: {fewest_stream}"
    );
    let saturated_outside_core = live
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(i, p)| {
            let x = (*i % width as usize) as f64 - width as f64 / 2.0;
            let y = (*i / width as usize) as f64 - height as f64 / 2.0;
            let max = *[p[0], p[1], p[2]].iter().max().unwrap();
            let min = *[p[0], p[1], p[2]].iter().min().unwrap();
            x.hypot(y) > height as f64 * 0.18 && max > 30 && max - min > 20
        })
        .count();
    let cool = live
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[1] > 60 && p[2] > 60)
        .count();
    let warm = live
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 70 && p[1] > 45 && p[1] > p[0].saturating_sub(20) && p[1] > p[2])
        .count();
    eprintln!(
        "Particle Dance: {saturated_outside_core} stream pixels outside center, {cool} cool, {warm} warm"
    );
    assert!(
        saturated_outside_core > (width * height) as usize / 50,
        "particle streams are too faint: {saturated_outside_core} visible pixels (need 2% of stage)"
    );
    assert!(
        cool > 100,
        "music-driven Particle Dance lost its cool hues: {cool}"
    );
    assert!(
        warm > 100,
        "music-driven Particle Dance lost its warm hues: {warm}"
    );
    // Isolate one production particle using its existing resource ownership,
    // rather than adding test-only methods to the renderer.
    gpu.swarm_count = 1;
    for buffer in gpu.swarm_buffers {
        unsafe {
            api::BindBuffer(api::ARRAY_BUFFER, buffer);
            api::BufferSubData(api::ARRAY_BUFFER, 0, 24, [0.0f32; 6].as_ptr().cast());
        }
    }
    frame.mode = VisualMode::ParticleDance;
    frame.energy = 1.0;
    frame.bass = 1.0;
    frame.treble = 1.0;
    frame.aether_onset = 1.0;
    frame.aether_beat_pulse = 1.0;
    frame.phi_event = 1.0;
    frame.phi_impulse = 1.0;
    let mut scene = Scene {
        frame: &frame,
        wave: &visual.waveform,
        cover: None,
        gallery: &mut gallery,
        gallery_bounds: Rect {
            x: 250.0,
            y: 40.0,
            width: 360.0,
            height: 280.0,
        },
    };
    gpu.draw(&mut scene, width, height, 1.0, 0.016).unwrap();
    let pixels = read();
    let bright: Vec<_> = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, p)| p[..3].iter().any(|c| *c > 2))
        .map(|(index, _)| (index % width as usize, index / width as usize))
        .collect();
    assert!(!bright.is_empty());
    let span =
        bright.iter().map(|p| p.0).max().unwrap() - bright.iter().map(|p| p.0).min().unwrap();
    let vertical =
        bright.iter().map(|p| p.1).max().unwrap() - bright.iter().map(|p| p.1).min().unwrap();
    assert!(
        span < 3 && vertical < 3,
        "particle too large: {}x{}px",
        span + 1,
        vertical + 1
    );
    gdk::GLContext::clear_current();
    assert!(gpu.cleanup().is_err());
    context.make_current();
    gpu.cleanup().unwrap();
    gpu.cleanup().unwrap();
    assert!(gpu.draw(&mut scene, width, height, 1.0, 0.016).is_err());
    unsafe {
        api::DeleteTextures(1, &texture);
        api::DeleteFramebuffers(1, &fbo);
    }
    let painted = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = painted.clone();
    let canvas = crate::render::canvas::RenderCanvas::new(
        move |ctx, _, _| {
            count.set(count.get() + 1);
            ctx.set_source_rgb(0.0, 0.0, 0.0);
            ctx.paint()?;
            Ok(())
        },
        |_, _, _, _, _| anyhow::bail!("forced rendering failure"),
    );
    let window = gtk::Window::builder()
        .title("omaMusi renderer fallback check")
        .default_width(200)
        .default_height(120)
        .child(&canvas.widget)
        .build();
    window.present();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !canvas.is_cpu() || painted.get() == 0 {
        while gtk::glib::MainContext::default().pending() {
            gtk::glib::MainContext::default().iteration(false);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "software fallback did not paint"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(canvas.failure().is_some());
    window.close();
    while gtk::glib::MainContext::default().pending() {
        gtk::glib::MainContext::default().iteration(false);
    }
    eprintln!(
        "All {} GPU visuals, 64 switches, cover clearing, waveform alpha/color, particle size and context-safe cleanup passed",
        VisualMode::ALL.len()
    );
}

fn save_particle_artifact(name: &str, pixels: &[u8], width: i32, height: i32) {
    if let Some(directory) = std::env::var_os("OMAMUSI_RENDER_ARTIFACTS") {
        std::fs::create_dir_all(&directory).unwrap();
        image::save_buffer(
            std::path::PathBuf::from(directory).join(name),
            pixels,
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}
