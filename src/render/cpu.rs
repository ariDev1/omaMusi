//! Cairo port of the baseline software visuals.
use super::Scene;
use crate::{
    artwork::Artwork,
    gallery::Rect,
    visual::{VisualFrame, VisualMode},
};
use anyhow::{Result, bail};
use cairo::{Context, Format, ImageSurface, LinearGradient, Operator, RadialGradient};
use std::f64::consts::TAU;

// Oversized torus, matching the GPU swarm shader: the gold ring is 3x
// larger than the surrounding swarm, so its arcs sweep across and beyond
// the frame instead of sitting as a small ring in the middle.
const TORUS_SCALE: f64 = 3.0;

pub struct CpuRenderer;
impl CpuRenderer {
    pub fn surface(scene: &mut Scene<'_>, width: i32, height: i32) -> Result<ImageSurface> {
        if width <= 0 || height <= 0 || width as i64 * height as i64 > 64_000_000 {
            bail!("Invalid render size");
        }
        let surface = ImageSurface::create(Format::ARgb32, width, height)?;
        {
            let ctx = Context::new(&surface)?;
            Self::draw(&ctx, scene, width as f64, height as f64)?;
        }
        surface.flush();
        Ok(surface)
    }
    pub fn draw(ctx: &Context, scene: &mut Scene<'_>, width: f64, height: f64) -> Result<()> {
        if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
            return Ok(());
        }
        ctx.save()?;
        let result = (|| -> Result<()> {
            ctx.set_operator(Operator::Source);
            ctx.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            ctx.paint()?;
            ctx.set_operator(Operator::Over);
            let frame = scene.frame;
            if !matches!(
                frame.mode,
                VisualMode::Waveform | VisualMode::CoverArt | VisualMode::CoverGallery
            ) {
                color(ctx, frame.colors[0], 1.0);
                ctx.paint()?;
            }
            match frame.mode {
                VisualMode::Warp => warp(ctx, frame, width, height)?,
                VisualMode::Spectrum => spectrum(ctx, frame, width, height)?,
                VisualMode::Waveform => waveform(ctx, scene, width, height)?,
                VisualMode::Spectrogram => spectrogram(ctx, frame, width, height)?,
                VisualMode::PhiCathedral => phi(ctx, frame, width, height)?,
                VisualMode::EventHorizon => horizon(ctx, frame, width, height)?,
                VisualMode::ParticleDance => particle_dance(ctx, frame, width, height)?,
                VisualMode::CoverArt => cover(ctx, scene, width, height)?,
                VisualMode::CoverGallery => gallery(ctx, scene, width, height)?,
            }
            Ok(())
        })();
        let restore = ctx.restore();
        result?;
        restore?;
        Ok(())
    }
}

fn color(ctx: &Context, rgb: [f32; 3], alpha: f64) {
    ctx.set_source_rgba(
        rgb[0] as f64,
        rgb[1] as f64,
        rgb[2] as f64,
        alpha.clamp(0.0, 1.0),
    );
}
fn ellipse(ctx: &Context, x: f64, y: f64, rx: f64, ry: f64) -> Result<()> {
    ctx.save()?;
    ctx.translate(x, y);
    ctx.scale(rx.max(0.0001), ry.max(0.0001));
    ctx.new_sub_path();
    ctx.arc(0.0, 0.0, 1.0, 0.0, TAU);
    ctx.restore()?;
    Ok(())
}
fn circle(ctx: &Context, x: f64, y: f64, radius: f64, rgb: [f32; 3], alpha: f64) -> Result<()> {
    color(ctx, rgb, alpha);
    ellipse(ctx, x, y, radius, radius)?;
    ctx.fill()?;
    Ok(())
}
fn line(
    ctx: &Context,
    from: (f64, f64),
    to: (f64, f64),
    width: f64,
    rgb: [f32; 3],
    alpha: f64,
) -> Result<()> {
    ctx.new_path();
    ctx.move_to(from.0, from.1);
    ctx.line_to(to.0, to.1);
    ctx.set_line_width(width);
    color(ctx, rgb, alpha);
    ctx.stroke()?;
    Ok(())
}
fn rounded(ctx: &Context, r: Rect, radius: f64) {
    let radius = radius.min(r.width / 2.0).min(r.height / 2.0).max(0.0);
    ctx.new_sub_path();
    ctx.arc(
        r.x + r.width - radius,
        r.y + radius,
        radius,
        -TAU / 4.0,
        0.0,
    );
    ctx.arc(
        r.x + r.width - radius,
        r.y + r.height - radius,
        radius,
        0.0,
        TAU / 4.0,
    );
    ctx.arc(
        r.x + radius,
        r.y + r.height - radius,
        radius,
        TAU / 4.0,
        TAU / 2.0,
    );
    ctx.arc(
        r.x + radius,
        r.y + radius,
        radius,
        TAU / 2.0,
        TAU * 3.0 / 4.0,
    );
    ctx.close_path();
}

pub fn rgba_surface(width: u32, height: u32, rgba: &[u8]) -> Result<ImageSurface> {
    if width == 0
        || height == 0
        || width as u64 * height as u64 > 64_000_000
        || rgba.len() as u64 != width as u64 * height as u64 * 4
    {
        bail!("Invalid image dimensions");
    }
    let mut bytes = vec![0u8; rgba.len()];
    for (source, dest) in rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(bytes.as_chunks_mut::<4>().0)
    {
        let alpha = source[3] as u32;
        let premultiply = |v: u8| (v as u32 * alpha + 127) / 255;
        let pixel = (alpha << 24)
            | (premultiply(source[0]) << 16)
            | (premultiply(source[1]) << 8)
            | premultiply(source[2]);
        dest.copy_from_slice(&pixel.to_ne_bytes());
    }
    Ok(ImageSurface::create_for_data(
        bytes,
        Format::ARgb32,
        width as i32,
        height as i32,
        width as i32 * 4,
    )?)
}
fn image(ctx: &Context, surface: &ImageSurface, r: Rect, alpha: f64) -> Result<()> {
    if alpha <= 0.0 || r.width <= 0.0 || r.height <= 0.0 {
        return Ok(());
    }
    ctx.save()?;
    ctx.rectangle(r.x, r.y, r.width, r.height);
    ctx.clip();
    ctx.translate(r.x, r.y);
    ctx.scale(
        r.width / surface.width() as f64,
        r.height / surface.height() as f64,
    );
    ctx.set_source_surface(surface, 0.0, 0.0)?;
    ctx.source().set_filter(cairo::Filter::Bilinear);
    let result = ctx.paint_with_alpha(alpha);
    ctx.restore()?;
    result?;
    Ok(())
}
fn fit(art: &Artwork, r: Rect) -> Rect {
    let scale = (r.width / art.width as f64).min(r.height / art.height as f64);
    let (x, y) = r.center();
    Rect {
        x: x - art.width as f64 * scale / 2.0,
        y: y - art.height as f64 * scale / 2.0,
        width: art.width as f64 * scale,
        height: art.height as f64 * scale,
    }
}
fn draw_art(ctx: &Context, art: &Artwork, r: Rect, alpha: f64) -> Result<()> {
    let surface = rgba_surface(art.width, art.height, &art.rgba)?;
    image(ctx, &surface, fit(art, r), alpha)
}

pub fn cover_rectangle(width: f64, height: f64, art: &Artwork, bass: f64) -> Rect {
    let scale = (width * 0.40 / art.width as f64).min(height * 0.65 / art.height as f64)
        * (1.0 + 0.055 * bass.clamp(0.0, 1.0));
    let w = art.width as f64 * scale;
    let h = art.height as f64 * scale;
    Rect {
        x: width * 0.74 - w / 2.0,
        y: height * 0.46 - h / 2.0,
        width: w,
        height: h,
    }
}
fn cover(ctx: &Context, scene: &Scene<'_>, width: f64, height: f64) -> Result<()> {
    ctx.set_source_rgb(0.0, 0.0, 0.0);
    ctx.paint()?;
    if let Some(art) = scene.cover.filter(|a| a.width > 0 && a.height > 0) {
        let r = cover_rectangle(width, height, art, scene.frame.bass);
        draw_art(ctx, art, r, 1.0)?;
        ctx.rectangle(r.x, r.y, r.width, r.height);
        ctx.set_source_rgba(1.0, 1.0, 1.0, 28.0 / 255.0);
        ctx.set_line_width(1.0);
        ctx.stroke()?;
    }
    Ok(())
}
fn text(ctx: &Context, label: &str, center: (f64, f64), size: f64) -> Result<()> {
    ctx.select_font_face(
        "monospace",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );
    ctx.set_font_size(size);
    let ext = ctx.text_extents(label)?;
    ctx.move_to(center.0 - (ext.width() / 2.0 + ext.x_bearing()), center.1);
    ctx.show_text(label)?;
    Ok(())
}
fn gallery(ctx: &Context, scene: &mut Scene<'_>, _width: f64, _height: f64) -> Result<()> {
    ctx.set_source_rgb(0.0, 0.0, 0.0);
    ctx.paint()?;
    let area = scene.gallery_bounds;
    let rectangles = scene.gallery.layout(area);
    if rectangles.is_empty() {
        ctx.set_source_rgba(1.0, 1.0, 1.0, 110.0 / 255.0);
        let (x, y) = area.center();
        if scene.gallery.scanning {
            text(ctx, "Finding album covers…", (x, y), 14.0)?;
        } else {
            text(ctx, "No album artwork found", (x, y - 9.0), 14.0)?;
            text(
                ctx,
                "Add a cover image to your music folders",
                (x, y + 9.0),
                14.0,
            )?;
        }
        return Ok(());
    }
    for (index, r) in rectangles.into_iter().enumerate() {
        let tile = &scene.gallery.tiles[index];
        ctx.set_source_rgb(17.0 / 255.0, 17.0 / 255.0, 20.0 / 255.0);
        rounded(ctx, r, 5.0);
        ctx.fill()?;
        let inset = r.width * 0.025;
        let target = Rect {
            x: r.x + inset,
            y: r.y + inset,
            width: r.width - inset * 2.0,
            height: r.height - inset * 2.0,
        };
        let fade = tile.fade * tile.fade * (3.0 - 2.0 * tile.fade);
        let cover = &scene.gallery.covers[&tile.key];
        if let Some(previous) = &tile.previous {
            ctx.push_group();
            ctx.set_operator(Operator::Add);
            if let Some(art) = &scene.gallery.covers[previous].image {
                draw_art(ctx, art, target, 1.0 - fade)?;
            }
            if let Some(art) = &cover.image {
                draw_art(ctx, art, target, fade)?;
            }
            let layer = ctx.pop_group()?;
            ctx.set_operator(Operator::Over);
            ctx.set_source(&layer)?;
            ctx.paint()?;
        } else if let Some(art) = &cover.image {
            draw_art(ctx, art, target, 1.0)?;
        }
        let hovered = scene.gallery.hovered == Some(index);
        ctx.set_source_rgba(1.0, 1.0, 1.0, if hovered { 150.0 } else { 28.0 } / 255.0);
        ctx.set_line_width(1.0);
        rounded(ctx, r, 5.0);
        ctx.stroke()?;
        if hovered {
            let height = r.height.min(52.0);
            ctx.set_source_rgba(0.0, 0.0, 0.0, 215.0 / 255.0);
            ctx.rectangle(r.x, r.y + r.height - height, r.width, height);
            ctx.fill()?;
            ctx.set_source_rgb(238.0 / 255.0, 238.0 / 255.0, 238.0 / 255.0);
            if let Some(path) = cover.tracks.first() {
                let label = if cover.tracks.len() == 1 {
                    path.file_stem()
                } else {
                    path.parent().and_then(|p| p.file_name())
                }
                .unwrap_or_default()
                .to_string_lossy();
                let max = ((r.width - 14.0) / 8.0).max(1.0) as usize;
                let label = if label.chars().count() > max {
                    format!(
                        "{}…",
                        label
                            .chars()
                            .take(max.saturating_sub(1))
                            .collect::<String>()
                    )
                } else {
                    label.into_owned()
                };
                text(
                    ctx,
                    &label,
                    (r.center().0, r.y + r.height - height + 20.0),
                    12.0,
                )?;
                let detail = if cover.tracks.len() == 1 {
                    "Click to play".into()
                } else {
                    format!("{} songs · click for random", cover.tracks.len())
                };
                ctx.save()?;
                ctx.rectangle(r.x + 7.0, r.y + r.height - height, r.width - 14.0, height);
                ctx.clip();
                text(ctx, &detail, (r.center().0, r.y + r.height - 9.0), 11.0)?;
                ctx.restore()?;
            }
        }
    }
    Ok(())
}

fn spectrum(ctx: &Context, f: &VisualFrame, w: f64, h: f64) -> Result<()> {
    let baseline = h * 0.76;
    let step = w / 96.0;
    let gradient = LinearGradient::new(0.0, baseline, 0.0, h * 0.2);
    for (stop, rgb, alpha) in [
        (0.0, f.colors[2], 80.0),
        (0.5, f.colors[1], 190.0),
        (1.0, f.colors[3], 210.0),
    ] {
        gradient.add_color_stop_rgba(
            stop,
            rgb[0] as f64,
            rgb[1] as f64,
            rgb[2] as f64,
            alpha / 255.0,
        );
    }
    ctx.set_source(&gradient)?;
    for (i, level) in f.bands.iter().enumerate() {
        let height = (level * h * 0.52).max(2.0);
        rounded(
            ctx,
            Rect {
                x: i as f64 * step + 2.0,
                y: baseline - height,
                width: (step - 4.0).max(2.0),
                height,
            },
            2.0,
        );
        ctx.fill()?;
    }
    for (i, peak) in f.peaks.iter().enumerate() {
        let y = baseline - peak * h * 0.52 - 3.0;
        line(
            ctx,
            (i as f64 * step + 2.0, y),
            ((i + 1) as f64 * step - 2.0, y),
            1.5,
            f.colors[1],
            185.0 / 255.0,
        )?;
    }
    Ok(())
}
fn waveform(ctx: &Context, scene: &Scene<'_>, w: f64, h: f64) -> Result<()> {
    let wave = scene.wave;
    let surface = rgba_surface(768, 256, &wave.rgba())?;
    image(
        ctx,
        &surface,
        Rect {
            x: 0.0,
            y: 0.0,
            width: w,
            height: h,
        },
        1.0,
    )?;
    if wave.latest_alpha >= 0.004 {
        let stride = (wave.latest.len() / (w as usize).max(1)).max(1);
        let values: Vec<_> = wave.latest.iter().step_by(stride).collect();
        ctx.new_path();
        for (i, value) in values.iter().enumerate() {
            let x = i as f64 * w / (values.len() - 1).max(1) as f64;
            let y = h * 0.47 - **value as f64 * h * 0.28;
            if i == 0 {
                ctx.move_to(x, y);
            } else {
                ctx.line_to(x, y);
            }
        }
        ctx.set_source_rgba(235.0 / 255.0, 252.0 / 255.0, 1.0, wave.latest_alpha);
        ctx.set_line_width(1.1);
        ctx.stroke()?;
    }
    Ok(())
}
fn spectrogram(ctx: &Context, f: &VisualFrame, w: f64, h: f64) -> Result<()> {
    let rgba: Vec<_> = f
        .history
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect();
    let surface = rgba_surface(360, 96, &rgba)?;
    image(
        ctx,
        &surface,
        Rect {
            x: 0.0,
            y: 0.0,
            width: w,
            height: h,
        },
        0.85,
    )
}

fn horizon(ctx: &Context, f: &VisualFrame, w: f64, h: f64) -> Result<()> {
    ctx.set_source_rgb(1.0 / 255.0, 3.0 / 255.0, 4.0 / 255.0);
    ctx.paint()?;
    ctx.save()?;
    ctx.translate(w * 0.82, h * 0.36);
    ctx.rotate(-18.0f64.to_radians());
    let radius = h * 0.49;
    let glow = RadialGradient::new(0.0, 0.0, 0.0, 0.0, 0.0, radius * 1.5);
    for (stop, rgb, alpha) in [
        (0.0, [0, 0, 0], 255),
        (0.665, [0, 0, 0], 255),
        (0.69, [255, 233, 186], 255),
        (0.80, [255, 214, 152], 255),
        (0.91, [132, 65, 25], 255),
        (1.0, [0, 0, 0], 0),
    ] {
        glow.add_color_stop_rgba(
            stop,
            rgb[0] as f64 / 255.0,
            rgb[1] as f64 / 255.0,
            rgb[2] as f64 / 255.0,
            alpha as f64 / 255.0,
        );
    }
    ctx.set_source(&glow)?;
    ellipse(ctx, 0.0, 0.0, radius * 1.5, radius * 1.5)?;
    ctx.fill()?;
    for [age, strength] in &f.bursts {
        let fade = (age / 0.07).min(1.0) * (1.0 - age / 1.8).max(0.0);
        let r = radius + h * (0.065 + age * 0.38);
        color(
            ctx,
            [0.8, 135.0 / 255.0, 71.0 / 255.0],
            (55.0 * strength * fade).floor() / 255.0,
        );
        ctx.set_line_width(1.5);
        ellipse(ctx, 0.0, 0.0, r, r)?;
        ctx.stroke()?;
    }
    let baseline = h * 0.19;
    let disk = LinearGradient::new(0.0, baseline - h * 0.14, 0.0, baseline + h * 0.14);
    for (stop, rgb, alpha) in [
        (0.0, [0, 0, 0], 0),
        (0.25, [0, 0, 0], 0),
        (0.43, [255, 209, 149], 0),
        (0.5, [255, 248, 221], 255),
        (0.57, [255, 209, 149], 220),
        (0.75, [166, 89, 40], 80),
        (1.0, [0, 0, 0], 0),
    ] {
        disk.add_color_stop_rgba(
            stop,
            rgb[0] as f64 / 255.0,
            rgb[1] as f64 / 255.0,
            rgb[2] as f64 / 255.0,
            alpha as f64 / 255.0,
        );
    }
    ctx.set_source(&disk)?;
    ctx.rectangle(-w * 1.5, baseline - h * 0.14, w * 3.0, h * 0.28);
    ctx.fill()?;
    for spot in f.spots.iter().filter(|s| s[0] >= 0.0) {
        let [age, strength, seed] = spot.map(f64::from);
        let orbit = seed * TAU + age * (1.35 + seed * 0.40);
        let orbit_r = h * (0.58 + seed * 0.21);
        let fade = (age / 0.09).min(1.0) * ((4.2 - age) / 2.4).clamp(0.0, 1.0);
        let x = orbit.cos() * orbit_r;
        let y = baseline - orbit.sin() * h * 0.12;
        if orbit.sin() < 0.0 || x.hypot(y) > radius {
            circle(
                ctx,
                x,
                y,
                h * 0.009,
                [1.0, 232.0 / 255.0, 179.0 / 255.0],
                (210.0 * strength * fade).floor() / 255.0,
            )?;
            for step in 1..6 {
                let trail = orbit - step as f64 * 0.06;
                circle(
                    ctx,
                    trail.cos() * orbit_r,
                    baseline - trail.sin() * h * 0.12,
                    h * 0.009,
                    [1.0, 117.0 / 255.0, 36.0 / 255.0],
                    (100.0 * strength * fade * (-step as f64 * 0.45).exp()).floor() / 255.0,
                )?;
            }
        }
        for (delay, attenuation) in [(0.26, 0.75), (0.62, 0.28)] {
            let echo = age - delay;
            if echo <= 0.0 {
                continue;
            }
            let past = seed * TAU + echo * (1.35 + seed * 0.40);
            if past.sin() <= 0.0 {
                continue;
            }
            let angle = (past.sin().abs() * 0.9 + 0.18).atan2(past.cos());
            let r = radius + h * (0.040 + seed * 0.09);
            for side in [-1.0, 1.0] {
                circle(
                    ctx,
                    angle.cos() * r,
                    side * angle.sin() * r,
                    h * 0.010,
                    [1.0, 184.0 / 255.0, 97.0 / 255.0],
                    (170.0 * strength * fade * attenuation).floor() / 255.0,
                )?;
            }
        }
    }
    ctx.restore()?;
    Ok(())
}

#[derive(Clone, Copy)]
pub struct ParticlePoint {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub color: [f32; 3],
    pub alpha: f64,
}
#[allow(
    clippy::approx_constant,
    reason = "Preserve the reference renderer's coefficients"
)]
pub fn particle_points(f: &VisualFrame, w: f64, h: f64) -> Vec<ParticlePoint> {
    let dance = f.time + f.aether_world_turn * 1.5 + f.aether_beat_phase * TAU * 0.05;
    let beat = f.aether_beat_pulse * 0.6 + f.phi_event * 0.4;
    let boom =
        f.aether_beat_pulse * 1.4 + f.aether_onset * 1.2 + f.phi_impulse * 1.6 + f.phi_event * 1.8;
    let drive = 1.0 - (-boom * 0.9).exp();
    let scale = w.min(h) * 0.42 / (1.0 + 0.20 * f.energy.clamp(0.0, 1.0));
    (0..1500)
        .map(|i| {
            let group = (i % 13) as f64;
            let seed = (i as f64 * 0.61803398875) % 1.0;
            let per = 0.45 + 1.10 * ((seed * 7.31) % 1.0);
            let heading = f.aether_world_turn * (0.70 + group / 13.0 * 1.05) + group / 13.0 * TAU;
            let orbit = dance * (0.07 + 0.022 * group) + heading;
            let cx = orbit.cos() * scale * (0.45 + boom * 0.22 * per);
            let cy = (orbit / 1.61803398875).sin() * scale * (0.35 + boom * 0.18 * per);
            let shell = (0.38 + (1.48 - 0.38) * (0.5 + 0.5 * (dance * 0.12 + group * 0.43).sin()))
                * (1.0 + f.aether_beat_pulse * 0.28 + beat * 0.2 + drive * 0.30);
            let angle = seed * TAU + dance * (0.70 + f.phi_velocity * 0.20) + beat * 0.8;
            let radius = shell
                * scale
                * (0.25 + 0.75 * ((i as f64 * 0.754877666) % 1.0))
                * (1.0 + drive * 0.50 * per);
            let jiggle =
                (0.10 + 0.22 * (dance * 0.7 + i as f64 * 0.031 + boom).sin()) * scale * 0.08;
            let mut x =
                w * 0.5 + cx + angle.cos() * radius * 0.35 + jiggle * (i as f64 * 1.7).sin();
            let mut y =
                h * 0.5 + cy + angle.sin() * radius * 0.35 + jiggle * (i as f64 * 2.3).cos();
            let band = 1.0 - (-f.bands[(i * 13) % 96] * 2.5).exp();
            let alpha = (52.0 + band * 18.0 + drive * 16.0)
                .floor()
                .clamp(10.0, 170.0)
                / 255.0;
            let rider = i % 10 < 3;
            if rider {
                let u = seed * TAU
                    + dance * 0.38 * (0.7 + drive * 0.6 + f.phi_velocity * 0.2)
                    + beat * 0.1;
                let v = ((seed * 7.77) % 1.0) * TAU * 3.0 + dance * 1.2;
                let ring = scale * 0.86 * TORUS_SCALE * (1.0 + drive * 0.14);
                let tube = scale * 0.075 * TORUS_SCALE * (1.0 + drive * 0.45);
                let tilt = 0.42 + 0.16 * (dance * 0.09 + 1.0).sin();
                let lx = (ring + tube * v.cos()) * u.cos();
                let ly = (ring + tube * v.cos()) * u.sin();
                let lz = tube * v.sin();
                // Displaced giant ring (matching the GPU shader): the ring
                // circle passes near the frame instead of enclosing it far
                // outside, so its near arc sweeps through the view. Screen y
                // grows downward, mirroring the shader's downward bias. The
                // direction is normalized so the offset stays at 0.92x the
                // ring radius in quiet and loud passages alike.
                let arc_mag = ring * 0.92;
                let arc_dx = (dance * 0.05 + 0.7).sin();
                let arc_dy = 0.9 - 0.2 * (dance * 0.043 + 2.0).sin();
                let arc_len = (arc_dx * arc_dx + arc_dy * arc_dy).sqrt().max(0.001);
                let arc_ox = arc_mag * arc_dx / arc_len;
                let arc_oy = arc_mag * arc_dy / arc_len;
                x = w * 0.5
                    + scale * 0.05 * (dance * 0.11 + f.aether_world_turn * 0.4).sin()
                    + arc_ox
                    + lx;
                y = h * 0.5
                    + scale * 0.04 * (dance * 0.083 + 1.2).cos()
                    + arc_oy
                    + (ly * tilt.cos() - lz * tilt.sin()) * 0.9;
            }
            let color = if rider {
                [1.0, 0.68, 0.20]
            } else {
                let shade = group as f32 / 13.0;
                [
                    0.12 + 0.30 * shade,
                    0.56 - 0.34 * shade,
                    0.94 - 0.16 * shade,
                ]
            };
            ParticlePoint {
                x,
                y,
                radius: 1.3,
                color,
                alpha,
            }
        })
        .collect()
}
fn particle_dance(ctx: &Context, f: &VisualFrame, w: f64, h: f64) -> Result<()> {
    for point in particle_points(f, w, h) {
        circle(
            ctx,
            point.x,
            point.y,
            point.radius,
            point.color,
            point.alpha,
        )?;
    }
    Ok(())
}

fn palette(f: &VisualFrame, index: usize) -> [f32; 3] {
    match index % 4 {
        0 => f.colors[2],
        1 => f.colors[1],
        2 => f.green,
        _ => f.magenta,
    }
}
fn phi(ctx: &Context, f: &VisualFrame, w: f64, h: f64) -> Result<()> {
    let phi = (1.0 + 5.0f64.sqrt()) / 2.0;
    let golden = TAU * (1.0 - 1.0 / phi);
    let turn = f.aether_beat_phase * TAU;
    let kick = f.aether_beat_pulse * 0.6 + f.phi_event * 0.4;
    let sway = f.aether_scene_morph - 0.5;
    let dance = f.time + f.aether_world_turn * 1.5 + turn * 0.05;
    let cx = w * (0.5 + 0.025 * (dance / phi + sway * 0.2).sin() + kick * 0.008);
    let cy = h * (0.5 + 0.025 * (dance / (phi * phi)).cos() + kick * 0.008);
    let scale = w.min(h) * (0.43 + f.phi_bloom * 0.08 + f.aether_beat_pulse * 0.02);
    ctx.set_operator(Operator::Add);
    let symmetry = 5.0 + 3.0 * (0.5 + 0.5 * (dance / phi).sin()) + 5.0 * f.phi_tension;
    for arm in 0..13 {
        let phase = arm as f64 * TAU / symmetry.max(5.0);
        ctx.new_path();
        for step in 0..96 {
            let theta = step as f64 * 0.12 + phase + dance * 0.18 + turn * 0.03;
            let radius = 2.0
                * (step as f64 * 0.018 * phi).exp()
                * (1.0 + f.phi_pulse * 0.08 * (theta * 8.0 - dance * phi).sin());
            let x = cx + theta.cos() * radius;
            let y = cy + theta.sin() * radius;
            if step == 0 {
                ctx.move_to(x, y);
            } else {
                ctx.line_to(x, y);
            }
        }
        color(
            ctx,
            palette(f, arm),
            (16.0 + 36.0 * f.energy + 22.0 * f.phi_event + 20.0 * f.aether_beat_pulse).floor()
                / 255.0,
        );
        ctx.set_line_width(1.0);
        ctx.stroke()?;
    }
    for i in 1..=377 {
        let n = i as f64 / 377.0;
        let band = f.bands[(i * 13) % 96];
        let wave = f.waveform[(i * 21) % 1024] as f64;
        let mut angle = i as f64 * golden + dance * 0.12 + turn * 0.03 + kick * 0.20;
        let radius = n.sqrt() * scale * (1.0 + f.phi_bloom * 0.16 + band * 0.09 + kick * 0.06);
        angle += wave * 0.22 + band * 0.18;
        let size =
            1.1 + band * 3.0 + f.treble * 1.4 + f.phi_event * 1.8 + f.aether_beat_pulse * 0.9;
        circle(
            ctx,
            cx + angle.cos() * radius,
            cy + angle.sin() * radius,
            size,
            palette(f, i),
            (45.0 + 125.0 * band + 55.0 * f.energy + 40.0 * f.aether_beat_pulse).floor() / 255.0,
        )?;
    }
    for fib in [5, 8, 13, 21] {
        let radius = scale * (fib as f64 / 21.0) * (0.65 + f.phi_pulse * 0.18 + kick * 0.08);
        color(
            ctx,
            f.colors[1],
            (8.0 + f.phi_event * 45.0 + f.bass * 18.0 + f.aether_beat_pulse * 30.0).floor() / 255.0,
        );
        ellipse(ctx, cx, cy, radius, radius)?;
        ctx.set_line_width(1.0);
        ctx.stroke()?;
    }
    ctx.set_operator(Operator::Over);
    Ok(())
}
fn warp(ctx: &Context, f: &VisualFrame, w: f64, h: f64) -> Result<()> {
    let t = f.time;
    let amp = 0.7 + f.energy * 0.9;
    let fx = ((t * 0.45).sin() * 0.16 + (t * 1.10 + 1.7).sin() * 0.07) * amp;
    let fy = ((t * 0.33).cos() * 0.13 + (t * 0.80 + 0.6).sin() * 0.05) * amp;
    let cx = w * (0.57 + fx);
    let cy = h * (0.47 + fy);
    let curve_x = (t * 0.65).sin() * h * 0.22 * (0.5 + f.energy);
    let curve_y = (t * 0.52).cos() * h * 0.22 * (0.5 + f.energy);
    let bank = (t * 0.31).sin() * 0.65 + f.bass * 0.18 + f.energy * 0.10;
    let zoom = 1.0 + 0.12 * (t * 0.9).sin() + f.energy * 0.15 + f.bass * 0.08;
    let scale = h * 0.72 * zoom;
    let rgb = f.colors[1];
    let glow = RadialGradient::new(cx, cy, 0.0, cx, cy, h * 0.65);
    glow.add_color_stop_rgba(
        0.0,
        rgb[0] as f64,
        rgb[1] as f64,
        rgb[2] as f64,
        (16.0 + f.energy * 22.0).floor() / 255.0,
    );
    glow.add_color_stop_rgba(1.0, rgb[0] as f64, rgb[1] as f64, rgb[2] as f64, 0.0);
    ctx.set_source(&glow)?;
    ctx.paint()?;
    ctx.set_operator(Operator::Add);
    for i in 0..18 {
        let z = (i as f64 * 28.0 / 18.0 - t * 8.0).rem_euclid(28.0) + 0.7;
        let radius = scale * 1.6 / z;
        color(
            ctx,
            f.colors[if i % 2 == 0 { 1 } else { 2 }],
            (12.0 + f.energy * 18.0).floor() / 255.0,
        );
        ctx.set_line_width(1.2);
        ellipse(ctx, cx, cy, radius, radius)?;
        ctx.stroke()?;
    }
    let angle = t * 0.12 + bank;
    let ca = angle.cos();
    let sa = angle.sin();
    for (i, seed) in f.particle_vertices.iter().enumerate() {
        let [sx, sy, sz] = seed.map(f64::from);
        let x = sx * ca - sy * sa;
        let y = sx * sa + sy * ca;
        let band = f.bands[i % 96];
        let wave = f.waveform[(i * 7) % 1024] as f64;
        let surge = 1.0 + 0.35 * (t * 0.9).sin() + f.energy * 0.6;
        let perp = wave * 0.45 * scale / sz.max(1.0);
        let speed = (8.0 + band * 10.0 + f.energy * 3.5 + f.bass * 2.0) * surge;
        let z = (sz - t * speed).rem_euclid(28.0) + 0.7;
        let tail_z = z + 0.7 + f.energy * 2.1 + f.bass * 0.8 + band * 1.6;
        let hx = cx + (x + perp) * scale / z + curve_x * (1.0 - z / 28.0);
        let hy = cy - (y + perp * 0.6) * scale / z + curve_y * (1.0 - z / 28.0);
        if hx < -100.0 || hx > w + 100.0 || hy < -100.0 || hy > h + 100.0 {
            continue;
        }
        let tail = (
            cx + x * scale / tail_z + curve_x * (1.0 - tail_z / 28.0),
            cy - y * scale / tail_z + curve_y * (1.0 - tail_z / 28.0),
        );
        let tint = f.tints[i % f.tints.len()] as usize;
        let rgb = match tint % 4 {
            0 => f.colors[1],
            1 => f.colors[2],
            2 => f.green,
            _ => f.magenta,
        };
        line(
            ctx,
            tail,
            (hx, hy),
            1.0 + f.energy + band * 1.7,
            rgb,
            (0.30 + f.energy * 0.40 + band * 0.30).min(1.0),
        )?;
        let size = 4.0 + f.energy * 6.0 + band * 4.0;
        let sprite = RadialGradient::new(hx, hy, 0.0, hx, hy, size * 31.0 / 64.0);
        for (stop, alpha) in [
            (0.0, 245.0),
            (0.08, 230.0),
            (0.22, 100.0),
            (0.52, 23.0),
            (1.0, 0.0),
        ] {
            sprite.add_color_stop_rgba(
                stop,
                rgb[0] as f64,
                rgb[1] as f64,
                rgb[2] as f64,
                alpha / 255.0,
            );
        }
        ctx.set_source(&sprite)?;
        ellipse(ctx, hx, hy, size / 2.0, size / 2.0)?;
        ctx.fill()?;
        let arm = size * 9.0 / 64.0;
        line(
            ctx,
            (hx - arm, hy),
            (hx + arm, hy),
            size * 0.65 / 64.0,
            rgb,
            1.0,
        )?;
        line(
            ctx,
            (hx, hy - arm),
            (hx, hy + arm),
            size * 0.65 / 64.0,
            rgb,
            1.0,
        )?;
    }
    ctx.set_operator(Operator::Over);
    let core = h * (0.038 + f.bass * 0.012 + f.energy * 0.006 + (t * 2.3).sin() * 0.003);
    color(ctx, f.colors[0].map(|v| v / 1.8), 1.0);
    ellipse(ctx, cx, cy, core, core)?;
    ctx.fill_preserve()?;
    color(
        ctx,
        f.colors[2],
        (28.0 + f.energy * 20.0 + f.bass * 10.0).floor() / 255.0,
    );
    ctx.set_line_width(1.0);
    ctx.stroke()?;
    Ok(())
}
