"""OpenGL shaders: a fullscreen vortex and one instanced warp-streak draw."""

import numpy as np
from OpenGL import GL
from OpenGL.GL.shaders import compileProgram, compileShader
from PySide6.QtCore import Qt, Signal
from PySide6.QtGui import QColor, QGuiApplication, QOffscreenSurface, QOpenGLContext, QSurfaceFormat
from PySide6.QtOpenGLWidgets import QOpenGLWidget


PARTICLE_COUNT = 4096

QUAD_VERTEX = """#version 330 core
out vec2 uv;
void main() {
    vec2 p = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    uv = p;
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
"""

QUAD_FRAGMENT = """#version 330 core
in vec2 uv;
out vec4 frag;
uniform int mode;
uniform vec2 resolution;
uniform vec3 background, accent, cyan, bright;
uniform float energy, phase, bass;
uniform sampler2D audioData, historyData;
float datum(int i) { return texelFetch(audioData, ivec2(i, 0), 0).r; }
void main() {
    vec3 color = background;
    if (mode == 0) {
        // Extreme rollercoaster: the vanishing point whips along a big
        // two-frequency flight path and the whole tunnel banks hard.
        vec2 flight = vec2(sin(phase*0.45)*0.16 + sin(phase*1.10+1.7)*0.07,
                           cos(phase*0.33)*0.13 + sin(phase*0.80+0.6)*0.05)
                      * (0.7+energy*0.9);
        vec2 warpCenter = vec2(0.57, 0.53) + flight;
        float bank = sin(phase*0.31)*0.65 + bass*0.18 + energy*0.10;
        float cb = cos(bank), sb = sin(bank);
        // Dive surges: the view zooms as the music pushes.
        float zoom = 1.0 + 0.12*sin(phase*0.9) + energy*0.15 + bass*0.08;
        vec2 p = (uv - warpCenter) * vec2(resolution.x / resolution.y, 1.0) / zoom;
        p = mat2(cb, -sb, sb, cb) * p;
        float radius = length(p);
        float depth = -log(max(radius, 0.035));
        float angle = atan(p.y, p.x+0.000001);
        float spiral = pow(0.5+0.5*sin(angle*5.0+depth*7.0-phase*3.0), 18.0);
        float rings = pow(0.5+0.5*sin(depth*18.0+phase*11.0), 22.0);
        float haze = pow(0.5+0.5*sin(angle*3.0+depth*5.0-phase*1.7), 4.0);
        float opening = smoothstep(0.035, 0.075, radius);
        // Subtle breathing rim instead of a hard ring.
        float rimRadius = 0.059 + bass*0.010 + energy*0.006 + sin(phase*2.3)*0.0035;
        float rim = exp(-abs(radius-rimRadius)*230.0);
        vec3 tint = mix(cyan, accent, 0.5+0.5*sin(angle+depth*1.2+phase*0.15));
        float wall = (spiral*0.17+rings*0.04+haze*0.025)*exp(-radius*1.2);
        color = mix(background*0.25, background, opening);
        color += tint*wall*opening*(0.65+energy*1.2+bass*0.25);
        color += mix(cyan,bright,0.5)*rim*(0.045+energy*0.075+bass*0.03);
    } else if (mode == 1) {
        int band = clamp(int(uv.x * 96.0), 0, 95);
        float level = datum(band), peak = datum(96 + band);
        float y = uv.y - 0.24;
        float inset = min(0.3, 2.0 / (resolution.x / 96.0));
        float edge = fract(uv.x * 96.0);
        float bar = step(inset, edge) * step(edge, 1.0 - inset);
        if (y >= 0.0 && y <= max(2.0 / resolution.y, level * 0.52)) {
            vec3 tint = mix(cyan, accent, clamp(y / 0.3, 0.0, 1.0));
            tint = mix(tint, bright, clamp((y - 0.3) / 0.22, 0.0, 1.0) * 0.55);
            color = mix(color, tint, bar * (0.35 + 0.5 * y / 0.52));
        }
        float peakLine = 1.0 - smoothstep(0.7, 1.5, abs(y - peak * 0.52 - 3.0/resolution.y) * resolution.y);
        color += accent * peakLine * bar * 0.65;
    } else if (mode == 2) {
        float x = clamp(uv.x * 1023.0, 0.0, 1023.0);
        int i = int(x);
        float sampleValue = mix(datum(192+i), datum(192+min(i+1,1023)), fract(x));
        float distance = abs(uv.y - (0.53 + sampleValue * 0.28)) * resolution.y;
        float glow = exp(-distance * 0.22) * 0.22;
        float line = exp(-distance * distance * 0.38);
        color += accent * (glow + line);
    } else {
        color = mix(background, texture(historyData, vec2(uv.x, 1.0-uv.y)).rgb, 0.85);
    }
    frag = vec4(color, 1.0);
}
"""

PARTICLE_VERTEX = """#version 330 core
uniform float phase, energy, bass, pixelRatio;
uniform vec2 resolution;
uniform vec3 accent, cyan, bright, green, magenta;
uniform sampler2D audioData;
out vec3 tint;
out float opacity;
out vec2 streakUV;
const float TAU = 6.28318530718;
float hash(float x) { return fract(sin(x*127.1+311.7)*43758.5453); }
float datum(int i) { return texelFetch(audioData, ivec2(i, 0), 0).r; }
void main() {
    // Fixed star seeds; audio waves bend the streams, time flies the curves.
    float id = float(gl_InstanceID)+1.0;
    float seedAngle = hash(id)*TAU;
    float bandPos = hash(id+7.0)*96.0;
    int bandIdx = int(bandPos);
    float band = mix(datum(bandIdx), datum(min(bandIdx+1, 95)), fract(bandPos));
    float wavePos = hash(id+13.0)*1024.0;
    int waveIdx = int(wavePos);
    float wave = mix(datum(192+waveIdx), datum(192+min(waveIdx+1, 1023)), fract(wavePos));
    // Same extreme flight path as the tunnel walls, banking hard.
    vec2 flight = vec2(sin(phase*0.45)*0.16 + sin(phase*1.10+1.7)*0.07,
                       cos(phase*0.33)*0.13 + sin(phase*0.80+0.6)*0.05)
                  * (0.7+energy*0.9);
    float bank = sin(phase*0.31)*0.65 + bass*0.18 + energy*0.10;
    float wobble = wave*0.45 + sin(phase*2.2+id*1.7)*0.06*(0.3+band*1.4);
    float angle = seedAngle + phase*0.12 + bank + wobble;
    float radius = 1.6+hash(id+31.0)*7.0;
    float surge = 1.0 + 0.35*sin(phase*0.9) + energy*0.6;
    float speed = (8.0 + band*10.0 + energy*3.5 + bass*2.0) * surge;
    float z = mod(hash(id+71.0)*28.0-phase*speed,28.0)+0.7;
    float tailZ = z+0.7+energy*2.1+bass*0.8+band*1.6;
    float aspect = resolution.x / resolution.y;
    vec2 radial = vec2(cos(angle)*aspect, sin(angle))*radius;
    // Whip the whole streak field through the bends; dives stretch it.
    vec2 curve = vec2(sin(phase*0.65), cos(phase*0.52))*resolution.y*0.22*(0.5+energy*1.0);
    float scale = resolution.y*0.72*(1.0+0.12*sin(phase*0.9)+energy*0.15+bass*0.08);
    vec2 center = resolution*(vec2(0.57,0.53)+flight);
    vec2 head = center+radial*scale/z + curve*(1.0-z/28.0);
    vec2 tail = center+radial*scale/tailZ + curve*(1.0-tailZ/28.0);
    vec2 direction = normalize(head-tail);
    vec2 normal = vec2(-direction.y,direction.x);
    vec2 corners[6] = vec2[6](vec2(0,-1),vec2(1,-1),vec2(1,1),
                             vec2(0,-1),vec2(1,1),vec2(0,1));
    vec2 corner = corners[gl_VertexID];
    float width = (0.9+hash(id+101.0)*1.1+energy*0.8+band*1.7)*pixelRatio;
    vec2 screen = mix(tail,head,corner.x)+normal*corner.y*width;
    gl_Position = vec4(screen/resolution*2.0-1.0,0.0,1.0);
    streakUV = corner;
    // Saturated per-stream palette: teal, amber, green, magenta.
    // Only a whisper of bright so colors never wash out to white.
    float pick = hash(id+141.0);
    vec3 palette = cyan;
    if (pick < 0.30) palette = cyan;
    else if (pick < 0.55) palette = accent;
    else if (pick < 0.75) palette = green;
    else palette = magenta;
    tint = mix(palette, bright, 0.12+hash(id+161.0)*0.15);
    tint = mix(tint, bright, clamp(band*0.8, 0.0, 1.0)*0.25);
    opacity = smoothstep(0.7,2.0,z)*(1.0-smoothstep(25.0,28.7,z))
              *(0.16+energy*0.34+band*0.30)*mix(0.4,1.0,hash(id+191.0));
    opacity *= smoothstep(0.037,0.07,length((head-center)/resolution.y));
}
"""

PARTICLE_FRAGMENT = """#version 330 core
in vec3 tint;
in float opacity;
in vec2 streakUV;
out vec4 frag;
void main() {
    float light = exp(-streakUV.y*streakUV.y*4.5);
    light *= pow(streakUV.x,0.75)*smoothstep(0.0,0.07,1.0-streakUV.x);
    frag = vec4(tint, light*opacity);
}
"""


def gl_format():
    fmt = QSurfaceFormat()
    fmt.setVersion(3, 3)
    fmt.setProfile(QSurfaceFormat.OpenGLContextProfile.CoreProfile)
    fmt.setSwapInterval(1)
    fmt.setDepthBufferSize(24)
    return fmt


def hardware_available():
    """Reject software GL as well as unsupported platforms before creating a widget."""
    if QGuiApplication.platformName() in ("offscreen", "minimal"):
        return False, "headless platform"
    context = QOpenGLContext()
    context.setFormat(gl_format())
    if not context.create():
        return False, "OpenGL context unavailable"
    surface = QOffscreenSurface()
    surface.setFormat(context.format())
    surface.create()
    if not context.makeCurrent(surface):
        return False, "OpenGL surface unavailable"
    try:
        renderer = (GL.glGetString(GL.GL_RENDERER) or b"unknown").decode(errors="replace")
        if any(word in renderer.lower() for word in ("llvmpipe", "softpipe", "swrast", "software")):
            return False, f"software OpenGL: {renderer}"
        fmt = context.format()
        if context.isOpenGLES() or (fmt.majorVersion(), fmt.minorVersion()) < (3, 3):
            return False, "desktop OpenGL 3.3 unavailable"
        return True, renderer
    finally:
        context.doneCurrent()


class GpuCanvas(QOpenGLWidget):
    failed = Signal(str)

    def __init__(self, owner):
        super().__init__(owner)
        self.owner = owner
        self.setFormat(gl_format())
        self.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
        self.ready = False
        self.quad = self.particles = self.vao = 0
        self.textures = []
        self.uniforms = {}
        self.history_revision = -1
        self.audio_payload = np.zeros(1216, dtype=np.float32)

    def initializeGL(self):
        self.history_revision = -1
        try:
            self.quad = compileProgram(compileShader(QUAD_VERTEX, GL.GL_VERTEX_SHADER),
                                       compileShader(QUAD_FRAGMENT, GL.GL_FRAGMENT_SHADER))
            self.particles = compileProgram(compileShader(PARTICLE_VERTEX, GL.GL_VERTEX_SHADER),
                                            compileShader(PARTICLE_FRAGMENT, GL.GL_FRAGMENT_SHADER))
            self.vao = int(GL.glGenVertexArrays(1))
            GL.glBindVertexArray(self.vao)
            self.textures = [int(x) for x in GL.glGenTextures(2)]
            for texture in self.textures:
                GL.glBindTexture(GL.GL_TEXTURE_2D, texture)
                GL.glTexParameteri(GL.GL_TEXTURE_2D, GL.GL_TEXTURE_MIN_FILTER, GL.GL_LINEAR)
                GL.glTexParameteri(GL.GL_TEXTURE_2D, GL.GL_TEXTURE_MAG_FILTER, GL.GL_LINEAR)
                GL.glTexParameteri(GL.GL_TEXTURE_2D, GL.GL_TEXTURE_WRAP_S, GL.GL_CLAMP_TO_EDGE)
                GL.glTexParameteri(GL.GL_TEXTURE_2D, GL.GL_TEXTURE_WRAP_T, GL.GL_CLAMP_TO_EDGE)
            GL.glBindTexture(GL.GL_TEXTURE_2D, self.textures[0])
            GL.glTexImage2D(GL.GL_TEXTURE_2D, 0, GL.GL_R32F, 1216, 1, 0, GL.GL_RED, GL.GL_FLOAT, None)
            GL.glBindTexture(GL.GL_TEXTURE_2D, self.textures[1])
            GL.glTexImage2D(GL.GL_TEXTURE_2D, 0, GL.GL_RGB8, 360, 96, 0, GL.GL_RGB, GL.GL_UNSIGNED_BYTE, None)
            for program in (self.quad, self.particles):
                self.uniforms[program] = {name: GL.glGetUniformLocation(program, name) for name in
                    ("mode", "resolution", "background", "accent", "cyan", "bright", "green", "magenta", "energy",
                     "phase", "bass", "pixelRatio", "audioData", "historyData")}
            self.context().aboutToBeDestroyed.connect(self.cleanup)
            self.ready = True
        except Exception as error:
            self.failed.emit(f"OpenGL initialization: {error}")

    def common_uniforms(self, program, width, height):
        state = self.owner
        u = self.uniforms[program]
        GL.glUseProgram(program)
        GL.glUniform2f(u["resolution"], width, height)
        GL.glUniform1f(u["energy"], state.energy)
        GL.glUniform1f(u["phase"], state.time)
        GL.glUniform1f(u["bass"], state.bass)
        for name, key in (("background", "background"), ("accent", "accent"),
                          ("cyan", "cyan"), ("bright", "bright_foreground"),
                          ("green", "green"), ("magenta", "magenta")):
            color = QColor(state.colors.get(key, "#ffffff"))
            location = u[name]
            if location is not None and location != -1:
                GL.glUniform3f(location, color.redF(), color.greenF(), color.blueF())

    def paintGL(self):
        if not self.ready:
            return
        try:
            state = self.owner
            ratio = self.devicePixelRatioF()
            width, height = int(self.width()*ratio), int(self.height()*ratio)
            GL.glViewport(0, 0, width, height)
            GL.glDisable(GL.GL_SCISSOR_TEST)
            GL.glDisable(GL.GL_CULL_FACE)
            GL.glColorMask(True, True, True, True)
            GL.glDepthMask(True)
            GL.glBlendEquation(GL.GL_FUNC_ADD)
            GL.glBindBuffer(GL.GL_PIXEL_UNPACK_BUFFER, 0)
            GL.glDisable(GL.GL_DEPTH_TEST)
            GL.glDisable(GL.GL_BLEND)
            GL.glBindVertexArray(self.vao)
            self.audio_payload[:96] = state.bands
            self.audio_payload[96:192] = state.peaks
            if state.mode in (0, 2):
                self.audio_payload[192:] = state.waveform()
            GL.glActiveTexture(GL.GL_TEXTURE0)
            GL.glBindTexture(GL.GL_TEXTURE_2D, self.textures[0])
            GL.glTexSubImage2D(GL.GL_TEXTURE_2D, 0, 0, 0, 1216, 1, GL.GL_RED, GL.GL_FLOAT, self.audio_payload)
            GL.glActiveTexture(GL.GL_TEXTURE1)
            GL.glBindTexture(GL.GL_TEXTURE_2D, self.textures[1])
            if state.mode == 3 and self.history_revision != state.history_revision:
                GL.glTexSubImage2D(GL.GL_TEXTURE_2D, 0, 0, 0, 360, 96, GL.GL_RGB,
                                   GL.GL_UNSIGNED_BYTE, np.ascontiguousarray(state.history))
                self.history_revision = state.history_revision
            self.common_uniforms(self.quad, width, height)
            u = self.uniforms[self.quad]
            GL.glUniform1i(u["mode"], state.mode)
            GL.glUniform1i(u["audioData"], 0)
            GL.glUniform1i(u["historyData"], 1)
            GL.glDrawArrays(GL.GL_TRIANGLES, 0, 3)
            if state.mode == 0:
                self.common_uniforms(self.particles, width, height)
                u = self.uniforms[self.particles]
                GL.glUniform1i(u["audioData"], 0)
                GL.glUniform1f(u["pixelRatio"], ratio)
                GL.glEnable(GL.GL_BLEND)
                GL.glBlendFunc(GL.GL_SRC_ALPHA, GL.GL_ONE)
                GL.glDrawArraysInstanced(GL.GL_TRIANGLES, 0, 6, PARTICLE_COUNT)
                GL.glDisable(GL.GL_BLEND)
            GL.glUseProgram(0)
            GL.glBindVertexArray(0)
        except Exception as error:
            self.ready = False
            self.failed.emit(f"OpenGL rendering: {error}")

    def cleanup(self):
        if not self.context() or not self.context().isValid():
            return
        self.makeCurrent()
        if self.textures:
            GL.glDeleteTextures(self.textures)
            self.textures = []
        if self.vao:
            GL.glDeleteVertexArrays(1, [self.vao])
            self.vao = 0
        for program in (self.quad, self.particles):
            if program:
                GL.glDeleteProgram(program)
        self.quad = self.particles = 0
        self.ready = False
        self.doneCurrent()
