//! OpenGL 3.3 host for the original shaders and transform-feedback swarm.
use super::{Scene, cpu::CpuRenderer};
use crate::visual::{VisualFrame, VisualMode};
use ::gl as api;
use anyhow::{Context, Result, bail};
use gtk::gdk;
use std::{
    collections::HashMap,
    ffi::{CStr, CString, c_void},
    sync::OnceLock,
};

const SWARM_COUNT: i32 = 32000;
const QUAD: usize = 0;
const PARTICLES: usize = 1;
const HORIZON: usize = 2;
const PHI: usize = 3;
const PHI_PARTICLES: usize = 4;
const SWARM_UPDATE: usize = 5;
const SWARM_RENDER: usize = 6;
const IMAGE: usize = 7;
const NAMES: &[&str] = &[
    "mode",
    "resolution",
    "background",
    "accent",
    "cyan",
    "bright",
    "green",
    "magenta",
    "energy",
    "phase",
    "bass",
    "treble",
    "phiPulse",
    "phiBloom",
    "phiTension",
    "phiEvent",
    "phiVelocity",
    "phiImpulse",
    "aetherOnset",
    "aetherBeatPhase",
    "aetherBeatConfidence",
    "aetherDensity",
    "aetherSceneMorph",
    "aetherWorldTurn",
    "aetherBeatPulse",
    "pixelRatio",
    "audioData",
    "historyData",
    "waveDensity",
    "waveAlpha",
    "dt",
    "canary",
    "imageData",
    "audioWaves[0]",
    "audioSpots[0]",
];
const QUAD_VERTEX: &str = include_str!("../../assets/shaders/quad.vert");
const IMAGE_FRAGMENT: &str = "#version 330 core\nin vec2 uv; out vec4 frag; uniform sampler2D imageData; void main(){ frag=texture(imageData,vec2(uv.x,1.0-uv.y)); }";

fn load_api() -> Result<()> {
    // Epoxy exports lazy GL function-pointer variables and supports both EGL and GLX.
    // Retain the library for the process lifetime; global gl-rs pointers must not dangle.
    static EPOXY: OnceLock<std::result::Result<libloading::Library, String>> = OnceLock::new();
    let library = EPOXY
        .get_or_init(|| {
            unsafe { libloading::Library::new("libepoxy.so.0") }.map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| anyhow::anyhow!("Cannot load OpenGL dispatch: {e}"))?;
    api::load_with(|name| {
        let symbol = CString::new(format!("epoxy_{name}")).unwrap();
        unsafe {
            library
                .get::<*const *const c_void>(symbol.as_bytes_with_nul())
                .map(|pointer| **pointer)
                .unwrap_or(std::ptr::null())
        }
    });
    if !api::GetString::is_loaded()
        || !api::CreateShader::is_loaded()
        || !api::GenVertexArrays::is_loaded()
    {
        bail!("OpenGL 3.3 entry points unavailable");
    }
    Ok(())
}
fn gl_string(name: u32) -> String {
    unsafe {
        let pointer = api::GetString(name);
        if pointer.is_null() {
            String::new()
        } else {
            CStr::from_ptr(pointer.cast())
                .to_string_lossy()
                .into_owned()
        }
    }
}
struct Program {
    id: u32,
    uniforms: HashMap<&'static str, i32>,
}
impl Program {
    fn new(vertex: &str, fragment: Option<&str>, feedback: bool) -> Result<Self> {
        unsafe {
            fn shader(source: &str, kind: u32) -> Result<u32> {
                let source = CString::new(source)?;
                unsafe {
                    let shader = api::CreateShader(kind);
                    api::ShaderSource(shader, 1, &source.as_ptr(), std::ptr::null());
                    api::CompileShader(shader);
                    let mut status = 0;
                    api::GetShaderiv(shader, api::COMPILE_STATUS, &mut status);
                    if status == 0 {
                        let mut length = 0;
                        api::GetShaderiv(shader, api::INFO_LOG_LENGTH, &mut length);
                        let mut log = vec![0u8; length.max(1) as usize];
                        api::GetShaderInfoLog(
                            shader,
                            length,
                            std::ptr::null_mut(),
                            log.as_mut_ptr().cast(),
                        );
                        api::DeleteShader(shader);
                        bail!(
                            "Shader compilation failed: {}",
                            String::from_utf8_lossy(&log)
                        );
                    }
                    Ok(shader)
                }
            }
            let vertex = shader(vertex, api::VERTEX_SHADER)?;
            let fragment = match fragment
                .map(|s| shader(s, api::FRAGMENT_SHADER))
                .transpose()
            {
                Ok(v) => v,
                Err(e) => {
                    api::DeleteShader(vertex);
                    return Err(e);
                }
            };
            let id = api::CreateProgram();
            api::AttachShader(id, vertex);
            if let Some(fragment) = fragment {
                api::AttachShader(id, fragment);
            }
            if feedback {
                let names = [
                    CString::new("outPosition").unwrap(),
                    CString::new("outVelocity").unwrap(),
                ];
                let pointers = [names[0].as_ptr(), names[1].as_ptr()];
                api::TransformFeedbackVaryings(id, 2, pointers.as_ptr(), api::INTERLEAVED_ATTRIBS);
            }
            api::LinkProgram(id);
            api::DetachShader(id, vertex);
            api::DeleteShader(vertex);
            if let Some(fragment) = fragment {
                api::DetachShader(id, fragment);
                api::DeleteShader(fragment);
            }
            let mut status = 0;
            api::GetProgramiv(id, api::LINK_STATUS, &mut status);
            if status == 0 {
                let mut length = 0;
                api::GetProgramiv(id, api::INFO_LOG_LENGTH, &mut length);
                let mut log = vec![0u8; length.max(1) as usize];
                api::GetProgramInfoLog(id, length, std::ptr::null_mut(), log.as_mut_ptr().cast());
                api::DeleteProgram(id);
                bail!("Shader linking failed: {}", String::from_utf8_lossy(&log));
            }
            let uniforms = NAMES
                .iter()
                .map(|name| {
                    let text = CString::new(*name).unwrap();
                    (*name, api::GetUniformLocation(id, text.as_ptr()))
                })
                .collect();
            Ok(Self { id, uniforms })
        }
    }
    fn float(&self, name: &str, value: f64) {
        unsafe {
            api::Uniform1f(self.uniforms[name], value as f32);
        }
    }
    fn int(&self, name: &str, value: i32) {
        unsafe {
            api::Uniform1i(self.uniforms[name], value);
        }
    }
    fn configure(&self, f: &VisualFrame, size: (i32, i32), ratio: f64, dt: f64) {
        unsafe {
            api::UseProgram(self.id);
            api::Uniform2f(self.uniforms["resolution"], size.0 as f32, size.1 as f32);
        }
        for (name, value) in [
            ("phase", f.time),
            ("energy", f.energy),
            ("bass", f.bass),
            ("treble", f.treble),
            ("phiPulse", f.phi_pulse),
            ("phiBloom", f.phi_bloom),
            ("phiTension", f.phi_tension),
            ("phiEvent", f.phi_event),
            ("phiVelocity", f.phi_velocity),
            ("phiImpulse", f.phi_impulse),
            ("aetherOnset", f.aether_onset),
            ("aetherBeatPhase", f.aether_beat_phase),
            ("aetherBeatConfidence", f.aether_beat_confidence),
            ("aetherDensity", f.aether_density),
            ("aetherSceneMorph", f.aether_scene_morph),
            ("aetherWorldTurn", f.aether_world_turn),
            ("aetherBeatPulse", f.aether_beat_pulse),
            ("pixelRatio", ratio),
            ("dt", dt),
        ] {
            self.float(name, value);
        }
        for (name, color) in [
            ("background", f.colors[0]),
            ("accent", f.colors[1]),
            ("cyan", f.colors[2]),
            ("bright", f.colors[3]),
            ("green", f.green),
            ("magenta", f.magenta),
        ] {
            unsafe {
                api::Uniform3fv(self.uniforms[name], 1, color.as_ptr());
            }
        }
    }
}

pub struct GlRenderer {
    context: gdk::GLContext,
    programs: Vec<Program>,
    vao: u32,
    textures: [u32; 4],
    swarm_buffers: [u32; 2],
    swarm_vaos: [u32; 2],
    swarm_index: usize,
    swarm_count: i32,
    history_revision: u64,
    wave_revision: u64,
    description: String,
    closed: bool,
}
impl GlRenderer {
    pub fn new(context: &gdk::GLContext) -> Result<Self> {
        if gdk::GLContext::current().as_ref() != Some(context) {
            bail!("OpenGL context is not current");
        }
        load_api()?;
        let description = gl_string(api::RENDERER);
        let lower = description.to_lowercase();
        if [
            "llvmpipe",
            "softpipe",
            "software rasterizer",
            "swrast",
            "swiftshader",
        ]
        .iter()
        .any(|name| lower.contains(name))
            && std::env::var("OMAMUSI_ALLOW_SOFTWARE_GL").as_deref() != Ok("1")
        {
            bail!("Software OpenGL ({description}); using software visuals");
        }
        let mut renderer = Self {
            context: context.clone(),
            programs: Vec::new(),
            vao: 0,
            textures: [0; 4],
            swarm_buffers: [0; 2],
            swarm_vaos: [0; 2],
            swarm_index: 0,
            swarm_count: SWARM_COUNT,
            history_revision: u64::MAX,
            wave_revision: u64::MAX,
            description,
            closed: false,
        };
        for (vertex, fragment, feedback) in [
            (
                QUAD_VERTEX,
                Some(include_str!("../../assets/shaders/quad.frag")),
                false,
            ),
            (
                include_str!("../../assets/shaders/particle.vert"),
                Some(include_str!("../../assets/shaders/particle.frag")),
                false,
            ),
            (
                QUAD_VERTEX,
                Some(include_str!("../../assets/shaders/event_horizon.frag")),
                false,
            ),
            (
                QUAD_VERTEX,
                Some(include_str!("../../assets/shaders/phi.frag")),
                false,
            ),
            (
                include_str!("../../assets/shaders/phi_particle.vert"),
                Some(include_str!("../../assets/shaders/phi_particle.frag")),
                false,
            ),
            (
                include_str!("../../assets/shaders/aether_swarm_update.vert"),
                None,
                true,
            ),
            (
                include_str!("../../assets/shaders/aether_swarm_render.vert"),
                Some(include_str!(
                    "../../assets/shaders/aether_swarm_render.frag"
                )),
                false,
            ),
            (QUAD_VERTEX, Some(IMAGE_FRAGMENT), false),
        ] {
            renderer
                .programs
                .push(Program::new(vertex, fragment, feedback)?);
        }
        unsafe {
            api::GenVertexArrays(1, &mut renderer.vao);
            api::BindVertexArray(renderer.vao);
            api::GenTextures(4, renderer.textures.as_mut_ptr());
            for texture in renderer.textures {
                api::BindTexture(api::TEXTURE_2D, texture);
                for (parameter, value) in [
                    (api::TEXTURE_MIN_FILTER, api::LINEAR),
                    (api::TEXTURE_MAG_FILTER, api::LINEAR),
                    (api::TEXTURE_WRAP_S, api::CLAMP_TO_EDGE),
                    (api::TEXTURE_WRAP_T, api::CLAMP_TO_EDGE),
                ] {
                    api::TexParameteri(api::TEXTURE_2D, parameter, value as i32);
                }
            }
            for (index, internal, w, h, format, kind) in [
                (0, api::R32F, 1216, 1, api::RED, api::FLOAT),
                (1, api::RGB8, 360, 96, api::RGB, api::UNSIGNED_BYTE),
                (2, api::RGBA8, 768, 256, api::RGBA, api::UNSIGNED_BYTE),
            ] {
                api::BindTexture(api::TEXTURE_2D, renderer.textures[index]);
                api::TexImage2D(
                    api::TEXTURE_2D,
                    0,
                    internal as i32,
                    w,
                    h,
                    0,
                    format,
                    kind,
                    std::ptr::null(),
                );
            }
            let mut seeds: Vec<f32> = include_bytes!("../../assets/swarm-seeds.bin")
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| f32::from_le_bytes(*p))
                .collect();
            // Higher particle rate (same small dot size): tile the baked
            // seeds up to SWARM_COUNT. Duplicated positions still diverge
            // because the update shader keys behavior off gl_VertexID.
            let needed = SWARM_COUNT as usize * 6;
            if seeds.len() < needed {
                let base = seeds.clone();
                while seeds.len() < needed {
                    let take = (needed - seeds.len()).min(base.len());
                    seeds.extend_from_slice(&base[..take]);
                }
            }
            seeds.truncate(needed);
            api::GenBuffers(2, renderer.swarm_buffers.as_mut_ptr());
            api::GenVertexArrays(2, renderer.swarm_vaos.as_mut_ptr());
            for (vao, buffer) in renderer.swarm_vaos.iter().zip(renderer.swarm_buffers) {
                api::BindVertexArray(*vao);
                api::BindBuffer(api::ARRAY_BUFFER, buffer);
                api::BufferData(
                    api::ARRAY_BUFFER,
                    (seeds.len() * 4) as isize,
                    seeds.as_ptr().cast(),
                    api::DYNAMIC_COPY,
                );
                api::EnableVertexAttribArray(0);
                api::VertexAttribPointer(0, 3, api::FLOAT, api::FALSE, 24, std::ptr::null());
                api::EnableVertexAttribArray(1);
                api::VertexAttribPointer(
                    1,
                    3,
                    api::FLOAT,
                    api::FALSE,
                    24,
                    12usize as *const c_void,
                );
            }
            api::BindVertexArray(0);
            api::BindBuffer(api::ARRAY_BUFFER, 0);
            api::Enable(api::PROGRAM_POINT_SIZE);
        }
        renderer.check_error()?;
        Ok(renderer)
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn context(&self) -> &gdk::GLContext {
        &self.context
    }
    fn require_current(&self) -> Result<()> {
        if self.closed {
            bail!("OpenGL renderer is closed");
        }
        if gdk::GLContext::current().as_ref() != Some(&self.context) {
            bail!("OpenGL resource context is not current");
        }
        Ok(())
    }
    fn check_error(&self) -> Result<()> {
        let error = unsafe { api::GetError() };
        if error != api::NO_ERROR {
            bail!("OpenGL error 0x{error:04x}");
        }
        Ok(())
    }
    pub fn draw(
        &mut self,
        scene: &mut Scene<'_>,
        width: i32,
        height: i32,
        ratio: f64,
        dt: f64,
    ) -> Result<()> {
        self.require_current()?;
        if width <= 0
            || height <= 0
            || width as i64 * height as i64 > 64_000_000
            || !ratio.is_finite()
            || ratio <= 0.0
        {
            bail!("Invalid OpenGL render dimensions");
        }
        let dt = if dt.is_finite() {
            dt.clamp(0.001, 0.05)
        } else {
            0.001
        };
        let f = scene.frame;
        unsafe {
            api::Viewport(0, 0, width, height);
            api::Disable(api::SCISSOR_TEST);
            api::Disable(api::CULL_FACE);
            api::ColorMask(api::TRUE, api::TRUE, api::TRUE, api::TRUE);
            api::DepthMask(api::TRUE);
            api::BlendEquation(api::FUNC_ADD);
            api::BindBuffer(api::PIXEL_UNPACK_BUFFER, 0);
            api::PixelStorei(api::UNPACK_ALIGNMENT, 1);
            api::Disable(api::DEPTH_TEST);
            api::Disable(api::BLEND);
            api::Enable(api::PROGRAM_POINT_SIZE);
            api::BindVertexArray(self.vao);
        }
        if matches!(f.mode, VisualMode::CoverArt | VisualMode::CoverGallery) {
            let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height)?;
            {
                let ctx = cairo::Context::new(&surface)?;
                ctx.scale(ratio, ratio);
                CpuRenderer::draw(&ctx, scene, width as f64 / ratio, height as f64 / ratio)?;
            }
            surface.flush();
            let bytes = surface.data().context("Cannot read cover surface")?;
            unsafe {
                api::ActiveTexture(api::TEXTURE3);
                api::BindTexture(api::TEXTURE_2D, self.textures[3]);
                api::TexImage2D(
                    api::TEXTURE_2D,
                    0,
                    api::RGBA8 as i32,
                    width,
                    height,
                    0,
                    api::BGRA,
                    api::UNSIGNED_INT_8_8_8_8_REV,
                    bytes.as_ptr().cast(),
                );
                api::UseProgram(self.programs[IMAGE].id);
                self.programs[IMAGE].int("imageData", 3);
                api::DrawArrays(api::TRIANGLES, 0, 3);
            }
        } else {
            let mut payload = [0.0f32; 1216];
            for (target, value) in payload[..96].iter_mut().zip(f.bands) {
                *target = value as f32;
            }
            for (target, value) in payload[96..192].iter_mut().zip(f.peaks) {
                *target = value as f32;
            }
            if f.mode == VisualMode::Waveform {
                for (target, value) in payload[192..].iter_mut().zip(&scene.wave.latest) {
                    *target = *value;
                }
            } else {
                payload[192..].copy_from_slice(&f.waveform);
            }
            unsafe {
                api::ActiveTexture(api::TEXTURE0);
                api::BindTexture(api::TEXTURE_2D, self.textures[0]);
                api::TexSubImage2D(
                    api::TEXTURE_2D,
                    0,
                    0,
                    0,
                    1216,
                    1,
                    api::RED,
                    api::FLOAT,
                    payload.as_ptr().cast(),
                );
                api::ActiveTexture(api::TEXTURE1);
                api::BindTexture(api::TEXTURE_2D, self.textures[1]);
            }
            if matches!(f.mode, VisualMode::Spectrogram | VisualMode::TimeTunnel)
                && self.history_revision != f.history_revision
            {
                unsafe {
                    api::TexSubImage2D(
                        api::TEXTURE_2D,
                        0,
                        0,
                        0,
                        360,
                        96,
                        api::RGB,
                        api::UNSIGNED_BYTE,
                        f.history.as_ptr().cast(),
                    );
                }
                self.history_revision = f.history_revision;
            }
            unsafe {
                api::ActiveTexture(api::TEXTURE2);
                api::BindTexture(api::TEXTURE_2D, self.textures[2]);
            }
            if f.mode == VisualMode::Waveform && self.wave_revision != scene.wave.revision {
                let pixels = scene.wave.rgba();
                unsafe {
                    api::TexSubImage2D(
                        api::TEXTURE_2D,
                        0,
                        0,
                        0,
                        768,
                        256,
                        api::RGBA,
                        api::UNSIGNED_BYTE,
                        pixels.as_ptr().cast(),
                    );
                }
                self.wave_revision = scene.wave.revision;
            }
            match f.mode {
                VisualMode::PhiCathedral => {
                    let program = &self.programs[PHI];
                    program.configure(f, (width, height), ratio, dt);
                    program.int("audioData", 0);
                    unsafe {
                        api::DrawArrays(api::TRIANGLES, 0, 3);
                    }
                    let program = &self.programs[PHI_PARTICLES];
                    program.configure(f, (width, height), ratio, dt);
                    program.int("audioData", 0);
                    unsafe {
                        api::Enable(api::BLEND);
                        api::BlendFunc(api::SRC_ALPHA, api::ONE);
                        api::DrawArrays(api::POINTS, 0, 6765);
                        api::Disable(api::BLEND);
                    }
                }
                VisualMode::EventHorizon => {
                    let program = &self.programs[HORIZON];
                    program.configure(f, (width, height), ratio, dt);
                    program.int("audioData", 0);
                    unsafe {
                        api::Uniform2fv(
                            program.uniforms["audioWaves[0]"],
                            5,
                            f.waves.as_ptr().cast(),
                        );
                        api::Uniform3fv(
                            program.uniforms["audioSpots[0]"],
                            4,
                            f.spots.as_ptr().cast(),
                        );
                        api::DrawArrays(api::TRIANGLES, 0, 3);
                    }
                }
                VisualMode::ParticleDance => {
                    unsafe {
                        api::ClearColor(f.colors[0][0], f.colors[0][1], f.colors[0][2], 1.0);
                        api::Clear(api::COLOR_BUFFER_BIT);
                    }
                    let target = 1 - self.swarm_index;
                    let program = &self.programs[SWARM_UPDATE];
                    program.configure(f, (width, height), ratio, dt);
                    unsafe {
                        api::Enable(api::RASTERIZER_DISCARD);
                        api::BindVertexArray(self.swarm_vaos[self.swarm_index]);
                        api::BindBufferBase(
                            api::TRANSFORM_FEEDBACK_BUFFER,
                            0,
                            self.swarm_buffers[target],
                        );
                        api::BeginTransformFeedback(api::POINTS);
                        api::DrawArrays(api::POINTS, 0, self.swarm_count);
                        api::EndTransformFeedback();
                        api::BindBufferBase(api::TRANSFORM_FEEDBACK_BUFFER, 0, 0);
                        api::Disable(api::RASTERIZER_DISCARD);
                    }
                    self.swarm_index = target;
                    let program = &self.programs[SWARM_RENDER];
                    program.configure(f, (width, height), ratio, dt);
                    program.int("audioData", 0);
                    program.float(
                        "canary",
                        if std::env::var("OMA_PARTICLE_CANARY").as_deref() == Ok("1") {
                            1.0
                        } else {
                            0.0
                        },
                    );
                    unsafe {
                        api::Enable(api::BLEND);
                        api::BlendFuncSeparate(
                            api::SRC_ALPHA,
                            api::ONE_MINUS_SRC_ALPHA,
                            api::ONE,
                            api::ONE_MINUS_SRC_ALPHA,
                        );
                        api::BindVertexArray(self.swarm_vaos[self.swarm_index]);
                        api::DrawArrays(api::POINTS, 0, self.swarm_count);
                        api::Disable(api::BLEND);
                    }
                }
                _ => {
                    let program = &self.programs[QUAD];
                    program.configure(f, (width, height), ratio, dt);
                    program.int("mode", f.mode as i32);
                    program.int("audioData", 0);
                    program.int("historyData", 1);
                    program.int("waveDensity", 2);
                    program.float("waveAlpha", scene.wave.latest_alpha);
                    unsafe {
                        api::DrawArrays(api::TRIANGLES, 0, 3);
                    }
                    if f.mode == VisualMode::Warp {
                        let program = &self.programs[PARTICLES];
                        program.configure(f, (width, height), ratio, dt);
                        program.int("audioData", 0);
                        unsafe {
                            api::Enable(api::BLEND);
                            api::BlendFunc(api::SRC_ALPHA, api::ONE);
                            api::DrawArraysInstanced(api::TRIANGLES, 0, 6, 4096);
                            api::Disable(api::BLEND);
                        }
                    }
                }
            }
        }
        unsafe {
            api::UseProgram(0);
            api::BindVertexArray(0);
            api::ActiveTexture(api::TEXTURE0);
        }
        self.check_error()
    }
    pub fn cleanup(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        self.require_current()?;
        unsafe {
            for program in &self.programs {
                api::DeleteProgram(program.id);
            }
            api::DeleteVertexArrays(1, &self.vao);
            api::DeleteVertexArrays(2, self.swarm_vaos.as_ptr());
            api::DeleteBuffers(2, self.swarm_buffers.as_ptr());
            api::DeleteTextures(4, self.textures.as_ptr());
        }
        self.programs.clear();
        self.vao = 0;
        self.swarm_vaos = [0; 2];
        self.swarm_buffers = [0; 2];
        self.textures = [0; 4];
        self.closed = true;
        Ok(())
    }
}
impl Drop for GlRenderer {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
#[path = "gl_tests.rs"]
mod tests;
