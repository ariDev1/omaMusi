"""OpenGL shaders: a fullscreen vortex and one instanced warp-streak draw."""

import numpy as np
from OpenGL import GL
from OpenGL.GL.shaders import compileProgram, compileShader
from PySide6.QtCore import Qt, Signal
from PySide6.QtGui import QColor, QGuiApplication, QOffscreenSurface, QOpenGLContext, QSurfaceFormat
from PySide6.QtOpenGLWidgets import QOpenGLWidget
import math
import time


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


PHI_PARTICLE_COUNT = 6765
AETHER_SWARM_COUNT = 2584

PHI_FRAGMENT = """#version 330 core
in vec2 uv;
out vec4 frag;
uniform vec2 resolution;
uniform vec3 background, accent, cyan, bright, green, magenta;
uniform float energy, phase, bass, treble, phiPulse, phiBloom, phiTension, phiEvent, phiVelocity, phiImpulse, aetherOnset, aetherBeatPhase, aetherDensity, aetherSceneMorph, aetherWorldTurn, aetherBeatPulse;
uniform sampler2D audioData;
const float PHI = 1.618033988749895;
const float TAU = 6.283185307179586;
float datum(int i) { return texelFetch(audioData, ivec2(i, 0), 0).r; }
float sat(float x) { return clamp(x, 0.0, 1.0); }

void main() {
    vec2 screen = (uv - 0.5) * vec2(resolution.x / resolution.y, 1.0);
    vec3 ro = vec3(0.18*sin(phase/PHI + aetherWorldTurn), 0.14*cos(phase/(PHI*PHI) - aetherWorldTurn*0.7), phase*0.18*phiVelocity);
    vec3 rd = normalize(vec3(screen, 1.15));
    vec3 gold = vec3(1.0, 0.72, 0.24);
    vec3 color = background * 0.24;

    for (int j=0; j<3; ++j) {
        float fj = float(j);
        vec3 center = vec3(
            0.72*sin(phase*(0.13+fj*0.037)*pow(PHI,fj)),
            0.52*cos(phase*(0.11+fj*0.029)/pow(PHI,fj+1.0)),
            2.8 + fj*2.15 + 0.55*sin(phase*(0.17+fj*0.051))
        );
        float t = (center.z - ro.z) / max(rd.z, 0.08);
        vec3 hit = ro + rd*t;
        vec2 q = hit.xy - center.xy;
        float r = max(length(q), 0.002);
        float a = atan(q.y, q.x);
        float lr = log(r);
        float symmetry = (j==0 ? 5.0 : (j==1 ? 8.0 : 13.0));
        symmetry += phiTension * (j==2 ? 2.0 : 1.0);
        float sector = TAU / symmetry;
        float folded = abs(mod(a + sector*0.5, sector) - sector*0.5);
        float s1 = 0.5+0.5*sin(folded*5.0  - lr*8.0  + phase*(0.70+fj*0.09));
        float s2 = 0.5+0.5*sin(folded*8.0  + lr*13.0 - phase/PHI*(1.0+fj*0.13));
        float s3 = 0.5+0.5*sin(folded*13.0 - lr*21.0 + phase*PHI*(0.31+fj*0.07));
        float cathedral = pow(s1,8.0)*0.38 + pow(s2,10.0)*0.34 + pow(s3,12.0)*0.30;
        int band = clamp(int(fract(a/TAU + 0.5 + fj*0.17)*96.0),0,95);
        float audio = datum(band);
        float depthFade = 1.0 / (1.0 + 0.19*abs(t));
        float portal = exp(-abs(r-(0.17+0.025*fj+phiPulse*0.07))*32.0);
        vec3 c1 = mix(cyan, magenta, 0.5+0.5*sin(a*PHI + phase*(0.15+fj*0.03)));
        vec3 c2 = mix(green, accent, 0.5+0.5*cos(lr*8.0 - phase/PHI));
        vec3 psychedelic = mix(c1,c2,0.5+0.5*sin((a+lr)*13.0+phase*0.19));
        psychedelic = mix(psychedelic,gold,phiEvent*(0.20+0.30*portal));
        float glow = cathedral*(0.08+energy*0.42+audio*0.48)*depthFade;
        glow += portal*(0.035+phiPulse*0.13+phiEvent*0.22)*depthFade;
        color += psychedelic*glow;
    }

    vec2 c0 = vec2(0.34*sin(phase*0.23), 0.28*cos(phase*0.19));
    vec2 c1 = vec2(0.46*sin(phase*0.17*PHI+1.4), 0.31*cos(phase*0.21/PHI+0.7));
    vec2 c2 = vec2(0.29*sin(phase*0.27+3.1), 0.36*cos(phase*0.15+2.5));

    vec2 d0 = screen - c0;
    vec2 d1 = screen - c1;
    vec2 d2 = screen - c2;

    float r0 = max(length(d0), 0.001);
    float r1 = max(length(d1), 0.001);
    float r2 = max(length(d2), 0.001);

    float a0 = atan(d0.y, d0.x);
    float a1 = atan(d1.y, d1.x);
    float a2 = atan(d2.y, d2.x);

    float flock0 = exp(-r0*4.4) * exp(-abs(sin(a0*6.0  - phase*(0.7 + phiVelocity*0.22) - r0*14.0))*11.5);
    float flock1 = exp(-r1*4.0) * exp(-abs(sin(a1*7.0  + phase*(0.6 + phiVelocity*0.18) - r1*16.0))*10.0);
    float flock2 = exp(-r2*3.7) * exp(-abs(sin(a2*8.0  - phase*(0.8 + phiVelocity*0.20) - r2*18.0))*9.5);

    float swirl = (flock0 + flock1 + flock2) * (0.012 + treble*0.09 + phiImpulse*0.18);
    color += mix(cyan, bright, 0.28+0.50*treble) * swirl;

    float shock = exp(-abs(length(screen) - fract(phase*0.7*phiVelocity + aetherBeatPhase*0.35)*1.35)*28.0);
    color += gold * shock * (0.01 + phiImpulse*0.16 + phiEvent*0.10 + aetherOnset*0.14 + aetherBeatPulse*0.10);
    frag = vec4(color, 1.0);
}
"""

PHI_PARTICLE_VERTEX = """#version 330 core
uniform vec2 resolution;
uniform vec3 accent, cyan, bright, green, magenta;
uniform float energy, phase, bass, treble, phiPulse, phiBloom, phiTension, phiEvent, phiVelocity, phiImpulse, aetherOnset, aetherBeatPhase, aetherDensity, aetherSceneMorph, aetherWorldTurn, aetherBeatPulse, pixelRatio;
uniform sampler2D audioData;
out vec3 tint;
out float opacity;
const float PHI = 1.618033988749895;
const float INV_PHI = 0.618033988749895;
const float GOLDEN_ANGLE = 2.399963229728653;
const float COUNT = 6765.0;
float datum(int i) { return texelFetch(audioData, ivec2(i, 0), 0).r; }

void main() {
    float n = float(gl_VertexID) + 1.0;
    float seed = fract(n*INV_PHI);
    bool swarm = seed > 0.34;

    int bandIndex = int(mod(n*13.0,96.0));
    float band = datum(bandIndex);
    int waveIndex = int(mod(n*21.0,1024.0));
    float wave = datum(192+waveIndex);

    int cluster = int(mod(n,3.0));
    vec2 nucleus;
    if (cluster == 0) {
        nucleus = vec2(0.55*sin(phase*0.13), 0.42*cos(phase*0.11/PHI));
    } else if (cluster == 1) {
        nucleus = vec2(0.62*sin(phase*0.17*PHI+2.1), 0.36*cos(phase*0.09+1.3));
    } else {
        nucleus = vec2(0.48*sin(phase*0.07+4.0), 0.50*cos(phase*0.15/PHI+2.7));
    }

    float relZ;
    vec2 worldXY;

    if (!swarm) {
        float travel = fract(n*INV_PHI + phase*(0.045+0.060*phiVelocity)
                             + band*0.035 + phiImpulse*0.030);
        relZ = mix(0.55, 8.25, travel);

        float arm = mod(n, 21.0);
        float armAngle = arm * GOLDEN_ANGLE;
        float radial = (0.22 + sqrt(n/COUNT)*1.15)
                       * (0.78 + 0.16*sin(n*0.013 + phase/PHI));
        float angle = armAngle + relZ*0.22 + phase*0.10 + wave*(0.08+phiTension*0.14);

        vec2 radialVec = vec2(cos(angle), sin(angle));
        vec2 tangent = vec2(-radialVec.y, radialVec.x);
        worldXY = radialVec * radial
                + tangent * (0.05 + 0.08*sin(phase*0.7 + n*0.017));
        worldXY += nucleus * 0.20;
    } else {
        float lane = fract(seed*3.137 + phase*(0.11 + 0.22*phiVelocity)
                           + band*0.045 + phiImpulse*0.10);
        relZ = mix(0.30, 6.20, lane) + float(cluster) * 0.08;

        float orbit = seed*6.28318530718
                    + phase*(0.75 + band*0.20 + phiVelocity*0.15)
                    + aetherWorldTurn*(0.8 + float(cluster)*0.27)
                    + aetherBeatPulse*0.45;
        float spread = 0.12 + 0.62*fract(n*0.754877666) + phiPulse*0.12;
        vec2 radialVec = vec2(cos(orbit), sin(orbit));
        vec2 tangent = vec2(-radialVec.y, radialVec.x);

        float turn = sin(phase*1.9 + n*0.031 + wave*5.0);
        float flock = 0.16 + 0.26*treble + 0.18*phiImpulse + aetherDensity*0.18 + aetherOnset*0.20;

        worldXY = nucleus
                + radialVec * spread
                + tangent * (0.10 + 0.22*turn + flock*0.35)
                + vec2(sin(phase*0.9 + n*0.021),
                       cos(phase*1.1 + n*0.018))
                  * 0.05 * (0.4 + band);
    }

    vec2 camera = vec2(0.18*sin(phase/PHI),
                       0.14*cos(phase/(PHI*PHI)));
    vec2 projected = (worldXY - camera) / relZ;
    projected.x /= resolution.x / resolution.y;
    gl_Position = vec4(projected*1.72, 0.0, 1.0);

    float nearFactor = clamp(1.8/relZ, 0.35, 3.0);
    gl_PointSize = (swarm
                    ? (0.9 + band*2.5 + treble*1.8 + phiImpulse*2.0)
                    : (0.8 + band*3.1 + treble*1.2 + phiImpulse*1.0))
                   * pixelRatio * nearFactor;

    float pick = fract(n*INV_PHI);
    vec3 base = pick < 0.25 ? cyan
              : (pick < 0.5 ? accent
              : (pick < 0.75 ? green : magenta));
    vec3 gold = vec3(1.0,0.72,0.24);

    tint = mix(base, bright,
               band*0.20 + treble*0.05 + (swarm ? 0.12 : 0.0));
    tint = mix(tint, gold, phiEvent*(0.14+0.38*band));

    opacity = (swarm
               ? (0.14 + band*0.46 + treble*0.18 + phiImpulse*0.10)
               : (0.11 + band*0.52 + energy*0.16 + phiImpulse*0.05))
              * smoothstep(swarm ? 6.20 : 8.25,
                           swarm ? 0.30 : 0.55,
                           relZ);
}
"""

AETHER_SWARM_UPDATE_VERTEX = """#version 330 core
layout(location=0) in vec3 inPosition;
layout(location=1) in vec3 inVelocity;

uniform float dt;
uniform float phase;
uniform float energy;
uniform float bass;
uniform float treble;
uniform float aetherOnset;
uniform float aetherDensity;
uniform float aetherWorldTurn;
uniform float aetherBeatPulse;

out vec3 outPosition;
out vec3 outVelocity;

const float PHI = 1.618033988749895;
const float TAU = 6.283185307179586;

float hash1(float n) {
    return fract(sin(n*12.9898 + 78.233) * 43758.5453);
}

void main() {
    float id = float(gl_VertexID);
    float group = mod(id, 13.0);
    float seed = hash1(id + 1.0);

    vec3 p = inPosition;
    vec3 v = inVelocity;

    float g = group / 13.0;
    float heading = aetherWorldTurn*(0.70 + g*1.05) + g*TAU;
    float orbit = phase*(0.07 + 0.022*group) + heading;
    float phrase = 0.5 + 0.5*sin(phase*(0.12 + group*0.007) + group*0.43 + seed*TAU);

    vec3 center = vec3(
        1.05*sin(orbit + group*PHI),
        0.82*cos(orbit/PHI + group*0.37),
        1.95*sin(orbit*0.41 + group*0.73)
    );

    vec3 toCenter = center - p;
    float dist = max(length(toCenter), 0.001);
    vec3 radial = toCenter / dist;

    float shell = mix(0.38, 1.48, phrase);
    float shellError = dist - shell;

    vec3 axis = normalize(vec3(
        0.35 + 0.4*sin(group),
        0.55 + 0.3*cos(group*PHI),
        0.45 + 0.2*sin(group*0.7)
    ));
    vec3 tangent = normalize(cross(radial, axis) + vec3(0.0001));

    float cohesion = 0.26 + aetherDensity*0.34 + energy*0.12;
    float orbitForce = 0.24 + treble*0.22 + 0.08*sin(id*0.13 + phase);
    float repel = smoothstep(0.55, 0.10, dist) * (0.75 + bass*0.70);

    vec3 shellForce = -radial * shellError * (0.82 + 0.25*aetherDensity);

    vec3 curl = vec3(
        sin(p.y*1.9 + phase*0.21 + seed*TAU),
        cos(p.z*1.4 - phase*0.17 + seed*TAU),
        sin(p.x*1.6 + phase*0.13 - seed*TAU)
    ) * (0.06 + treble*0.09);

    vec3 drift = normalize(center + vec3(0.001)) * (0.02 + aetherBeatPulse*0.06);

    vec3 beatKick = tangent * (aetherBeatPulse*1.15 + aetherOnset*0.55)
                  + radial * (bass*aetherBeatPulse*0.40)
                  + normalize(vec3(
                        sin(group + phase*0.2),
                        cos(group*0.7 - phase*0.13),
                        sin(seed*TAU + phase*0.17)
                    )) * (aetherOnset*0.14);

    vec3 accel = radial*cohesion
               + shellForce
               + tangent*orbitForce
               - radial*repel
               + curl
               + drift
               + beatKick;

    v += accel * dt;
    v *= exp(-dt * 0.26);

    float speed = length(v);
    float maxSpeed = 0.72 + energy*1.10 + aetherOnset*0.85;
    if (speed > maxSpeed) {
        v *= maxSpeed / speed;
    }

    p += v * dt;

    float radius = length(p);
    if (radius > 4.9) {
        v -= normalize(p) * (radius - 4.9) * dt * 3.8;
    }

    outPosition = p;
    outVelocity = v;
}

"""

AETHER_SWARM_RENDER_VERTEX = """#version 330 core
layout(location=0) in vec3 inPosition;
layout(location=1) in vec3 inVelocity;

uniform vec2 resolution;
uniform float phase;
uniform float energy;
uniform float bass;
uniform float treble;
uniform float aetherOnset;
uniform float aetherDensity;
uniform float aetherWorldTurn;
uniform float aetherBeatPulse;
uniform float pixelRatio;

out vec3 tint;
out float opacity;
out vec2 streakDir;
out float streakMix;

const float PHI = 1.618033988749895;

void main() {
    vec3 p = inPosition;
    vec3 vel = inVelocity;

    float yaw = 0.24*sin(aetherWorldTurn*0.31) + aetherWorldTurn*0.09;
    float cy = cos(yaw);
    float sy = sin(yaw);
    p.xz = mat2(cy,-sy,sy,cy) * p.xz;
    vel.xz = mat2(cy,-sy,sy,cy) * vel.xz;

    float pitch = 0.14*sin(aetherWorldTurn*0.19/PHI);
    float cp = cos(pitch);
    float sp = sin(pitch);
    p.yz = mat2(cp,-sp,sp,cp) * p.yz;
    vel.yz = mat2(cp,-sp,sp,cp) * vel.yz;

    float depth = p.z + 6.0;
    float safeDepth = max(0.50, depth);
    vec2 projected = p.xy / safeDepth;
    projected.x /= resolution.x / resolution.y;

    gl_Position = vec4(projected*2.45, 0.0, 1.0);

    float speed = length(vel);
    float nearFactor = clamp(2.9/safeDepth, 0.45, 4.6);
    gl_PointSize = (2.4 + speed*6.2 + treble*1.5 + aetherBeatPulse*2.6)
                   * pixelRatio * nearFactor;

    vec2 dir = vel.xy;
    float dirLen = length(dir);
    streakDir = dirLen > 0.0001 ? dir / dirLen : vec2(1.0, 0.0);
    streakMix = clamp(speed*1.8 + aetherBeatPulse*0.9 + aetherOnset*0.45, 0.0, 1.0);

    float id = float(gl_VertexID);
    float huePick = fract(id*0.61803398875);

    vec3 cyan = vec3(0.20,0.92,1.00);
    vec3 magenta = vec3(1.00,0.20,0.78);
    vec3 gold = vec3(1.00,0.72,0.24);
    vec3 violet = vec3(0.46,0.30,1.00);

    vec3 base = mix(cyan, magenta, smoothstep(0.18,0.82,huePick));
    base = mix(base, violet, 0.25 + 0.20*sin(id*0.07 + phase*0.2));
    base = mix(base, gold, aetherOnset*(0.22 + 0.20*huePick));

    float nearGlow = clamp(1.6/safeDepth, 0.0, 1.0);
    tint = base * (0.95 + energy*0.62 + speed*0.34 + nearGlow*0.32);
    opacity = clamp(0.38 + speed*0.42 + aetherDensity*0.24 + aetherBeatPulse*0.18,
                    0.0, 0.98);
}

"""

AETHER_SWARM_RENDER_FRAGMENT = """#version 330 core
in vec3 tint;
in float opacity;
in vec2 streakDir;
in float streakMix;
out vec4 frag;

void main() {
    vec2 p = gl_PointCoord*2.0 - 1.0;
    vec2 side = vec2(-streakDir.y, streakDir.x);

    float along = dot(p, streakDir);
    float across = dot(p, side);

    float body = exp(-(across*across*(16.0 + 12.0*streakMix)
                      + along*along*(7.0 - 4.5*streakMix)));
    float core = exp(-(across*across*(26.0 + 10.0*streakMix)
                      + along*along*(16.0 - 8.0*streakMix)));
    float tail = exp(-(across*across*(15.0 + 8.0*streakMix)
                      + max(0.0, -along)*(7.0 + 10.0*streakMix)))
                 * streakMix * 0.75;
    float halo = exp(-dot(p,p)*2.3) * 0.24;

    float alpha = (body*0.95 + core + tail + halo) * opacity;
    if (alpha < 0.02) discard;

    frag = vec4(tint, alpha);
}

"""

EVENT_HORIZON_FRAGMENT = """#version 330 core
in vec2 uv;
out vec4 frag;

uniform vec2 resolution;
uniform float phase;
uniform float energy;
uniform float bass;
uniform float treble;
uniform float phiPulse;
uniform float phiBloom;
uniform float phiTension;
uniform float phiEvent;
uniform float phiVelocity;
uniform float phiImpulse;
uniform float aetherOnset;
uniform float aetherBeatPhase;
uniform float aetherDensity;
uniform float aetherSceneMorph;
uniform float aetherWorldTurn;
uniform float aetherBeatPulse;

const float TAU = 6.283185307179586;
const float PHI = 1.618033988749895;

float hash1(float n) {
    return fract(sin(n * 127.1 + 311.7) * 43758.5453123);
}

float hash2(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float noise2(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float a = hash2(i);
    float b = hash2(i + vec2(1.0, 0.0));
    float c = hash2(i + vec2(0.0, 1.0));
    float d = hash2(i + vec2(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

float fbm(vec2 p) {
    float value = 0.0;
    float amp = 0.55;
    mat2 m = mat2(0.80, -0.60, 0.60, 0.80);
    for (int i = 0; i < 5; ++i) {
        value += amp * noise2(p);
        p = m * p * 2.04 + vec2(3.7, -2.1);
        amp *= 0.48;
    }
    return value;
}

mat2 rot(float a) {
    float c = cos(a);
    float s = sin(a);
    return mat2(c, -s, s, c);
}

float ring(float r, float center, float width) {
    return exp(-abs(r - center) / max(width, 1e-4));
}

float annulus(float r, float innerR, float outerR, float soft) {
    return smoothstep(innerR - soft, innerR + soft, r)
         * (1.0 - smoothstep(outerR - soft, outerR + soft, r));
}

float starLayer(vec2 p, float scale, float threshold) {
    vec2 cell = floor(p * scale);
    vec2 local = fract(p * scale) - 0.5;
    float h = hash2(cell);
    float star = smoothstep(threshold, 1.0, h);
    float d = length(local);
    return star * exp(-d * d * mix(90.0, 230.0, h));
}

void main() {
    float musicDrive = clamp(
        energy * 0.36 + bass * 0.24 + treble * 0.10 + aetherDensity * 0.20 + aetherBeatPulse * 0.20,
        0.0, 1.0
    );

    float phraseSurge = smoothstep(0.56, 0.94, aetherSceneMorph)
                      * (0.35 + 0.65 * (0.5 + 0.5 * sin(phase * 0.032 + aetherWorldTurn)));
    float beatSurge = clamp(aetherBeatPulse * 0.90 + aetherOnset * 0.72 + phiEvent * 0.28, 0.0, 1.6);
    float gravityPull = 0.028 * phraseSurge + 0.020 * beatSurge;
    float edgeFeeling = clamp(0.55 + phraseSurge * 0.25 + beatSurge * 0.18 + bass * 0.12, 0.0, 1.2);

    float fly = phase * 0.029 + aetherWorldTurn * 0.060;
    float approach = 0.5 + 0.5 * sin(fly * 0.73 - 0.6 + aetherSceneMorph * 0.8);
    float perspectiveDrift = 0.5 + 0.5 * sin(fly * 0.57 + 1.4);
    vec2 cameraLoop = vec2(
        0.090 * sin(fly) + 0.040 * sin(phase * 0.090 + aetherSceneMorph * 1.6),
        0.058 * sin(fly * 0.68 + 0.95) + 0.032 * phraseSurge * sin(phase * 0.19)
    );
    vec2 cameraKick = vec2(
        0.020 * sin(phase * 0.88 + phiEvent * 4.0),
        0.018 * cos(phase * 0.74 + phiImpulse * 6.0)
    ) * (0.20 + aetherOnset * 0.85 + aetherBeatPulse * 0.48);
    float cameraRoll = 0.082 * sin(fly * 0.59)
                     + 0.028 * sin(phase * 0.10 + aetherSceneMorph * 1.4)
                     + 0.022 * phraseSurge * sin(phase * 0.06);
    float edgeZoom = mix(1.10, 1.48, edgeFeeling);
    float cameraZoom = (mix(0.92, 1.34, approach) + edgeZoom * 0.16) - gravityPull * 0.20;
    float inclination = mix(0.16, 0.995, 0.5 + 0.5 * sin(fly * 0.84 + 0.55 + aetherSceneMorph * 0.95));
    float foreshorten = mix(0.11, 0.995, inclination);

    vec2 screen = uv - (vec2(0.5) + cameraLoop + cameraKick);
    screen.x *= resolution.x / resolution.y;
    screen = rot(cameraRoll) * screen;
    screen /= cameraZoom;
    screen.x *= mix(0.89, 1.12, perspectiveDrift);
    screen.y += gravityPull * (0.10 + 0.18 * (0.5 + 0.5 * cos(aetherBeatPhase * TAU)));

    // Strong off-axis placement keeps the object cinematic instead of symbolic.
    vec2 horizonCenter = vec2(
        0.16 + 0.050 * sin(fly * 0.40 + 0.4),
       -0.016 + 0.026 * cos(fly * 0.58 - 0.2)
    );
    vec2 p = screen - horizonCenter;
    p = rot(0.065 * sin(fly * 0.32) + 0.035 * phraseSurge) * p;

    float antiEyeBias = clamp(0.74 + 0.40 * cos(atan(p.y, p.x) - 0.08) + 0.18 * sin(p.x * 2.8 - phase * 0.15), 0.0, 1.6);

    float shadowRadius = 0.170 + bass * 0.046 + phiPulse * 0.020 + phraseSurge * 0.010;
    float r = max(length(p), 0.0005);
    vec2 radialDir = p / r;
    float a = atan(p.y, p.x);

    float starBendMask = smoothstep(shadowRadius * 0.70, shadowRadius * 3.40, r);
    float lensWarp = (0.074 + 0.045 * musicDrive + 0.024 * phraseSurge) / (r * r + 0.018);
    lensWarp *= starBendMask;
    vec2 lensedBackground = p + radialDir * lensWarp;

    float starsFine = starLayer(lensedBackground + vec2(phase * 0.0015, 0.0), 88.0, 0.987);
    float starsCoarse = starLayer(lensedBackground * 0.60 - vec2(0.0, phase * 0.0011), 52.0, 0.981);
    float starsHalo = ring(r, shadowRadius * 1.56, 0.20) * (starsFine * 0.60 + starsCoarse * 0.40);
    float stars = (starsFine * 0.66 + starsCoarse * 0.30 + starsHalo * 0.60)
                * (0.72 + 0.28 * sin(phase * 0.22 + hash2(floor(lensedBackground * 64.0)) * TAU));

    vec2 discP = p;
    discP.y /= foreshorten;
    discP.x *= clamp(1.0 + discP.y * 0.15 * cos(fly), 0.74, 1.30);

    float discR = length(discP);
    float discA = atan(discP.y, discP.x);

    float discInner = shadowRadius * 1.46;
    float discOuter = 1.38;
    float directDisc = annulus(discR, discInner, discOuter, 0.040);

    float discHalfThickness = 0.022 + 0.060 * foreshorten + bass * 0.010 + phraseSurge * 0.010;
    float discPlane = mix(0.14, 1.0, exp(-abs(p.y) / discHalfThickness));
    float tangentialBelt = exp(-abs(p.y) / (discHalfThickness * 2.80));
    directDisc *= discPlane * tangentialBelt;

    float centerOcclusion = 1.0 - smoothstep(shadowRadius - 0.014, shadowRadius + 0.040, abs(discP.x));
    directDisc *= max(0.0, 1.0 - centerOcclusion * smoothstep(shadowRadius * 0.80, shadowRadius * 1.05, discR));

    float rotation = phase * (0.58 + phiVelocity * 0.22) + aetherWorldTurn * 0.32;
    vec2 plasmaCoord = vec2(discA * 4.0 + rotation, log(max(discR, discInner)) * 6.6 - phase * 0.22);

    float turbulence = fbm(plasmaCoord * vec2(1.0, 1.18));
    float turbulenceFine = fbm(plasmaCoord * vec2(3.1, 3.0) + 9.4);
    float magmaFlow = fbm(plasmaCoord * vec2(0.54, 0.38) - vec2(phase * 0.024, 0.0));
    float tangentialFlow = 0.5 + 0.5 * sin(discA * 34.0 - phase * (2.05 + musicDrive * 1.14) + turbulence * 5.2 + log(max(discR, discInner)) * 5.4);
    float shearFlow = 0.5 + 0.5 * sin(discA * 52.0 - phase * (3.30 + musicDrive * 1.30) + turbulenceFine * 6.4);
    float radialBands = 0.5 + 0.5 * sin(log(max(discR, discInner)) * 21.5 - phase * 0.68 + turbulence * 2.5);
    float stressLines = pow(max(0.0, sin(discA * 72.0 - phase * 4.0 + turbulenceFine * 8.8)), 14.0) * (0.24 + 1.02 * beatSurge);
    float streamShear = pow(max(0.0, sin(discA * 18.0 - phase * 1.56 + turbulence * 3.2)), 4.8);
    float wrapFlow = pow(max(0.0, sin(discA * 8.6 - phase * 0.98 + turbulence * 1.9)), 3.2);
    float discLaneCoherence = smoothstep(0.40, 0.86, 0.58 * tangentialFlow + 0.42 * shearFlow);
    float discLaneContrast = 0.5 + 0.5 * sin(discA * 12.0 - phase * 1.10 + turbulence * 2.2);

    float discCore = directDisc;
    float plasmaBands = mix(tangentialFlow, radialBands, 0.10);
    float fineFilaments = smoothstep(0.50, 0.93, 0.66 * shearFlow + 0.34 * turbulenceFine);
    float hotKnots = pow(max(0.0, sin(discA * 18.0 - rotation * 2.2 + turbulenceFine * 4.4)), 10.0);
    float radialHeat = 1.0 - smoothstep(discInner, discOuter, discR);
    radialHeat = pow(clamp(radialHeat, 0.0, 1.0), 0.62);

    float observerSide = 0.5 + 0.5 * cos(discA - 0.35 - cameraRoll);
    float asymmetry = pow(observerSide, 1.82);
    float doppler = mix(0.22, 3.05, asymmetry);
    float heat = discCore
               * (0.22 + 0.34 * plasmaBands + 0.24 * fineFilaments + 0.22 * hotKnots + 0.18 * magmaFlow + 0.18 * streamShear + 0.18 * wrapFlow + 0.16 * discLaneCoherence + 0.12 * discLaneContrast)
               * (0.42 + 1.42 * radialHeat)
               * doppler;

    float photonRadius = shadowRadius * 1.42;
    float upperWarp = 0.012 * sin(a * 2.8 + phase * 0.05) + 0.013 * fbm(vec2(a * 2.7, phase * 0.05 + r * 7.2));
    float lowerWarp = 0.010 * sin(a * 3.8 - phase * 0.04) + 0.011 * fbm(vec2(a * 2.3 + 5.0, phase * 0.04 + r * 6.2));

    float topArcRadius = photonRadius + 0.070 + 0.120 * (1.0 - foreshorten) + upperWarp;
    float bottomArcRadius = photonRadius + 0.038 + 0.074 * (1.0 - foreshorten) + lowerWarp;

    float topAngular = smoothstep(-0.04, 0.38, sin(a)) * (0.18 + 0.82 * smoothstep(-0.90, 0.66, cos(a)));
    float bottomAngular = smoothstep(-0.12, 0.30, -sin(a)) * (0.08 + 0.92 * smoothstep(-0.995, 0.34, cos(a)));

    float upperLens = ring(r, topArcRadius, 0.021 + 0.018 * (1.0 - foreshorten)) * topAngular;
    float upperInner = ring(r, photonRadius + 0.020 + upperWarp * 0.35, 0.0092) * smoothstep(-0.02, 0.92, sin(a));
    float lowerLens = ring(r, bottomArcRadius, 0.018) * bottomAngular * 0.78;

    float lensFlow = 0.58 + 0.42 * sin(a * 24.0 - phase * (1.92 + musicDrive * 0.92) + fbm(vec2(a * 3.8, r * 18.0 + phase * 0.12)) * 4.4);
    float bentContinuity = 0.52 + 0.48 * tangentialFlow;
    upperLens *= (0.70 + 0.62 * lensFlow) * bentContinuity;
    upperInner *= (0.74 + 0.52 * lensFlow) * bentContinuity;
    lowerLens *= (0.68 + 0.50 * (1.0 - lensFlow)) * bentContinuity;

    float heroBandTop = ring(r, photonRadius + 0.096 + upperWarp * 0.9, 0.036 + 0.024 * (1.0 - foreshorten))
                      * smoothstep(0.06, 0.998, sin(a))
                      * smoothstep(-0.84, 0.70, cos(a));
    float heroBandBottom = ring(r, photonRadius + 0.050 + lowerWarp * 0.8, 0.027)
                         * smoothstep(0.00, 0.998, -sin(a))
                         * smoothstep(-0.995, 0.36, cos(a));
    float heroFlow = 0.52 + 0.48 * sin(a * 26.0 - phase * 2.02 + turbulenceFine * 5.2);
    heroBandTop *= (0.78 + 0.70 * heroFlow) * bentContinuity;
    heroBandBottom *= (0.68 + 0.58 * (1.0 - heroFlow)) * bentContinuity;

    float ringBreak = 0.24 + 0.76 * fbm(vec2(a * 5.5, phase * 0.05 + r * 10.0));
    float ringSide = smoothstep(-0.06, 0.92, cos(a - 0.05));
    float photonRing = ring(r, photonRadius + upperWarp * 0.12, 0.0032 + 0.0026 * treble) * ringBreak * ringSide * (0.14 + 0.86 * lensFlow);

    // Tidal shear sheets replace the eye-like spoke reading.
    float tidalShear = exp(-abs(p.y - (0.14 * sin(p.x * 1.9 + phase * 0.38) - 0.01)) * (9.4 + phraseSurge * 2.6));
    float tidalShearB = exp(-abs(p.y - (-0.19 * sin(p.x * 1.55 - phase * 0.36) + 0.14)) * (6.8 + beatSurge * 2.1));
    float tidalShearC = exp(-abs(p.y - (0.28 * sin(p.x * 0.92 + phase * 0.20) + 0.34)) * (4.4 + edgeFeeling * 1.1));
    float leftSurge = exp(-abs(p.y - (-0.12 * sin((p.x + 0.95) * 2.4 - phase * 0.42) - 0.02)) * (6.0 + aetherOnset * 3.0))
                    * smoothstep(0.55, -0.82, p.x);
    float leftWake = exp(-abs(p.y - (0.08 * sin((p.x + 0.60) * 3.1 + phase * 0.56) + 0.10)) * (7.0 + treble * 2.4))
                   * smoothstep(0.42, -0.90, p.x);
    float streamA = tidalShear;
    float streamB = tidalShearB;
    float streamC = tidalShearC;
    float energyStream = max(streamA * 1.00, max(streamB * 0.92, streamC * 0.74));
    float streamAsymmetry = 0.34 + 0.66 * smoothstep(-0.04, 0.96, p.x + 0.26);
    energyStream *= streamAsymmetry;
    energyStream += leftSurge * 0.52 + leftWake * 0.42;

    // Add layered foreground plasma sheets for depth.
    float foregroundSheet = exp(-abs(screen.y - (0.06 * sin(screen.x * 2.2 - phase * 0.25) - 0.22)) * 6.5)
                          * exp(-abs(screen.x + 0.20) * 0.75);
    float foregroundSheetB = exp(-abs(screen.y - (-0.08 * sin(screen.x * 1.7 + phase * 0.18) + 0.28)) * 5.4)
                           * exp(-abs(screen.x - 0.48) * 0.90);
    float foregroundDustVeil = exp(-abs(screen.y - (0.04 * sin(screen.x * 2.9 + phase * 0.48) - 0.02)) * 4.2)
                             * exp(-abs(screen.x + 0.05) * 0.45);
    float foregroundDustStreak = exp(-abs(screen.y - (-0.16 * sin(screen.x * 1.2 - phase * 0.22) + 0.18)) * 3.8)
                               * exp(-abs(screen.x - 0.62) * 0.82);

    float shockRadius = shadowRadius + 0.12 + fract(aetherBeatPhase + phase * 0.024 + aetherOnset * 0.14) * 0.64;
    float shock = ring(r, shockRadius, 0.010 + aetherOnset * 0.010) * (aetherBeatPulse * 0.24 + aetherOnset * 0.22 + phiEvent * 0.10);

    float shadowMask = smoothstep(shadowRadius + 0.010, shadowRadius - 0.008, r);
    // Give the silhouette a subtle not-perfect edge.
    float edgeFray = 0.010 * fbm(vec2(a * 6.4 + phase * 0.02, r * 24.0));
    float shadowFray = smoothstep(shadowRadius + 0.014 + edgeFray, shadowRadius - 0.010 + edgeFray, r);
    float lensEdgeComplexity = ring(r, shadowRadius + 0.020 + edgeFray * 0.45, 0.012)
                             * (0.44 + 0.56 * sin(a * 10.0 + phase * 0.26 + turbulence * 1.8));
    float innerGlow = exp(-max(r - shadowRadius, 0.0) * 10.2) * smoothstep(shadowRadius, shadowRadius + 0.15, r);

    float glimpseGate = smoothstep(0.76, 0.98, aetherSceneMorph * 0.60 + phiEvent * 0.24 + (0.5 + 0.5 * sin(phase * 0.07)) * 0.30);
    float glimpse = glimpseGate * shadowMask * exp(-abs(sin(a * 8.0 + phase * 0.20) - sin(r * 58.0 - aetherWorldTurn)) * 8.2);

    vec3 dustColor = vec3(0.0);
    float dust = 0.0;
    float dustFront = 0.0;
    for (int i = 0; i < 34; ++i) {
        float fi = float(i);
        float seed = fi * PHI;
        float depth = mix(0.08, 1.0, hash1(seed + 13.0));
        float lane = hash1(seed + 4.0);
        float orbit = phase * (0.018 + lane * 0.032) / depth + TAU * hash1(seed + 7.0);
        float rr = mix(photonRadius + 0.15, 1.52, fract(1.0 - phase * (0.018 + lane * 0.030) / depth + lane));
        vec2 pos = horizonCenter + vec2(cos(orbit), sin(orbit)) * rr;
        pos.y *= 0.56 + 0.26 * sin(seed);
        vec2 delta = screen - pos;
        float streak = exp(-(abs(delta.x) * mix(4.0, 10.0, depth) + abs(delta.y) * mix(14.0, 38.0, depth)));
        float spark = exp(-length(delta) * mix(14.0, 42.0, depth));
        dust += (spark + streak * 0.65) * mix(0.14, 0.032, depth);
        if (depth < 0.34) {
            dustFront += spark + streak * 0.5;
        }
        dustColor += mix(vec3(0.88, 0.15, 0.016), vec3(1.00, 0.72, 0.12), lane) * (spark + streak * 0.4);
    }

    vec3 voidBlack = vec3(0.0010, 0.0005, 0.0002);
    vec3 smoke = vec3(0.014, 0.0032, 0.0010);
    vec3 ember = vec3(0.34, 0.034, 0.0030);
    vec3 orange = vec3(1.00, 0.28, 0.020);
    vec3 hot = vec3(1.00, 0.66, 0.12);
    vec3 whiteHot = vec3(1.00, 0.96, 0.86);

    float backgroundHalo = exp(-abs(r - photonRadius * 1.26) * 3.5);
    vec3 color = mix(voidBlack, smoke, 0.034 + backgroundHalo * 0.024);
    color += vec3(0.68, 0.78, 0.92) * stars * (0.24 + 0.12 * treble);

    color += ember * discCore * (0.10 + 0.08 * bass);
    color += orange * heat * (0.82 + energy * 0.26) * antiEyeBias;
    color += hot * heat * radialHeat * (0.42 + treble * 0.16 + aetherDensity * 0.12) * antiEyeBias;
    color += whiteHot * heat * hotKnots * (0.18 + aetherOnset * 0.24) * antiEyeBias;

    color += hot * stressLines * discCore * (0.18 + 0.38 * beatSurge);
    color += orange * phraseSurge * discCore * (0.14 + 0.20 * magmaFlow);
    color += hot * wrapFlow * discCore * (0.12 + 0.20 * phraseSurge);
    color += hot * discLaneCoherence * discCore * (0.10 + 0.16 * treble);

    color += orange * upperLens * (0.48 + bass * 0.12) * antiEyeBias;
    color += hot * upperLens * lensFlow * (0.24 + aetherDensity * 0.10) * antiEyeBias;
    color += whiteHot * upperInner * (0.26 + aetherBeatPulse * 0.16) * antiEyeBias;
    color += hot * lowerLens * (0.26 + bass * 0.10);
    color += whiteHot * lensEdgeComplexity * (0.05 + beatSurge * 0.05);

    color += orange * heroBandTop * (0.96 + bass * 0.22 + phraseSurge * 0.40) * antiEyeBias;
    color += hot * heroBandTop * (0.46 + heroFlow * 0.22) * antiEyeBias;
    color += hot * heroBandBottom * (0.40 + bass * 0.15 + beatSurge * 0.22);
    color += whiteHot * heroBandTop * (0.20 + aetherOnset * 0.18) * antiEyeBias;
    color += whiteHot * photonRing * (0.04 + treble * 0.04 + aetherOnset * 0.06);

    color += mix(orange, hot, 0.55) * energyStream * (0.34 + beatSurge * 0.38 + phraseSurge * 0.24);
    color += whiteHot * energyStream * stressLines * 0.08;
    color += hot * leftSurge * (0.18 + 0.22 * aetherOnset);
    color += orange * leftWake * (0.12 + 0.18 * treble);

    color += orange * foregroundSheet * (0.20 + 0.26 * phraseSurge + 0.18 * beatSurge);
    color += hot * foregroundSheetB * (0.16 + 0.20 * aetherOnset);
    color += hot * foregroundDustVeil * (0.08 + 0.14 * beatSurge);
    color += whiteHot * foregroundDustStreak * (0.04 + 0.10 * treble);
    color += whiteHot * dustFront * 0.08;

    color += orange * innerGlow * (0.048 + energy * 0.030);
    color += hot * shock;
    color += whiteHot * glimpse * (0.040 + phiEvent * 0.060);
    color += dustColor * (0.048 + treble * 0.040);
    color += hot * dust * (0.026 + aetherOnset * 0.036);

    color *= 1.0 - shadowFray * 0.992;

    float vignette = smoothstep(1.76, 0.16, length(screen));
    color *= vignette;

    float exposure = 1.22 + musicDrive * 0.28 + phraseSurge * 0.16 + aetherOnset * 0.14;
    color = vec3(1.0) - exp(-color * exposure);
    color = pow(max(color, vec3(0.0)), vec3(0.93));

    float grain = hash2(gl_FragCoord.xy + vec2(floor(phase * 8.0), 23.0)) - 0.5;
    color += grain * (0.0046 + treble * 0.0016);

    frag = vec4(max(color, vec3(0.0)), 1.0);
}
"""

PHI_PARTICLE_FRAGMENT = """#version 330 core
in vec3 tint;
in float opacity;
out vec4 frag;
void main() {
    vec2 p = gl_PointCoord*2.0-1.0;
    float r2 = dot(p,p);
    if (r2 > 1.0) discard;
    float core = exp(-r2*5.0);
    float halo = exp(-r2*1.6)*0.30;
    frag = vec4(tint, (core+halo)*opacity);
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
        self.quad = self.particles = self.phi = self.phi_particles = self.event_horizon = self.vao = 0
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
            self.event_horizon = compileProgram(compileShader(QUAD_VERTEX, GL.GL_VERTEX_SHADER),
                                               compileShader(EVENT_HORIZON_FRAGMENT, GL.GL_FRAGMENT_SHADER))
            self.phi = compileProgram(compileShader(QUAD_VERTEX, GL.GL_VERTEX_SHADER),
                                      compileShader(PHI_FRAGMENT, GL.GL_FRAGMENT_SHADER))
            self.phi_particles = compileProgram(compileShader(PHI_PARTICLE_VERTEX, GL.GL_VERTEX_SHADER),
                                                compileShader(PHI_PARTICLE_FRAGMENT, GL.GL_FRAGMENT_SHADER))
            self._init_aether_swarm()
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
            for program in (self.quad, self.particles, self.phi, self.phi_particles, self.event_horizon):
                self.uniforms[program] = {name: GL.glGetUniformLocation(program, name) for name in
                    ("mode", "resolution", "background", "accent", "cyan", "bright", "green", "magenta", "energy",
                     "phase", "bass", "treble", "phiPulse", "phiBloom", "phiTension", "phiEvent",
                     "phiVelocity", "phiImpulse", "aetherOnset", "aetherBeatPhase",
                     "aetherDensity", "aetherSceneMorph", "aetherWorldTurn", "aetherBeatPulse",
                     "pixelRatio", "audioData", "historyData")}
            self.context().aboutToBeDestroyed.connect(self.cleanup)
            self.ready = True
        except Exception as error:
            self.failed.emit(f"OpenGL initialization: {error}")


    def _link_transform_feedback_program(self, vertex_source):
        shader = compileShader(vertex_source, GL.GL_VERTEX_SHADER)
        program = GL.glCreateProgram()
        GL.glAttachShader(program, shader)
        varying_name_0 = GL.ctypes.create_string_buffer(b"outPosition")
        varying_name_1 = GL.ctypes.create_string_buffer(b"outVelocity")
        varying_ptrs = (GL.ctypes.POINTER(GL.ctypes.c_char) * 2)(
            GL.ctypes.cast(varying_name_0, GL.ctypes.POINTER(GL.ctypes.c_char)),
            GL.ctypes.cast(varying_name_1, GL.ctypes.POINTER(GL.ctypes.c_char)),
        )
        varying_names = GL.ctypes.cast(
            varying_ptrs,
            GL.ctypes.POINTER(GL.ctypes.POINTER(GL.ctypes.c_char)),
        )
        GL.glTransformFeedbackVaryings(
            program,
            2,
            varying_names,
            GL.GL_INTERLEAVED_ATTRIBS,
        )
        GL.glLinkProgram(program)
        linked = GL.glGetProgramiv(program, GL.GL_LINK_STATUS)
        if not linked:
            log = GL.glGetProgramInfoLog(program)
            raise RuntimeError(f"Aether swarm transform-feedback link failed: {log}")
        GL.glDetachShader(program, shader)
        GL.glDeleteShader(shader)
        return int(program)

    def _init_aether_swarm(self):
        self.swarm_update = self._link_transform_feedback_program(
            AETHER_SWARM_UPDATE_VERTEX
        )
        self.swarm_render = compileProgram(
            compileShader(AETHER_SWARM_RENDER_VERTEX, GL.GL_VERTEX_SHADER),
            compileShader(AETHER_SWARM_RENDER_FRAGMENT, GL.GL_FRAGMENT_SHADER),
        )

        rng = np.random.default_rng(2584)
        state = np.zeros((AETHER_SWARM_COUNT, 6), dtype=np.float32)

        group = np.arange(AETHER_SWARM_COUNT, dtype=np.float32) % 13.0
        angle = group * (math.tau / 13.0)
        radius = rng.uniform(0.25, 2.1, AETHER_SWARM_COUNT).astype(np.float32)

        state[:, 0] = np.cos(angle) * radius + rng.normal(0, 0.16, AETHER_SWARM_COUNT)
        state[:, 1] = np.sin(angle) * radius + rng.normal(0, 0.16, AETHER_SWARM_COUNT)
        state[:, 2] = rng.uniform(-2.2, 2.2, AETHER_SWARM_COUNT)

        tangent_x = -np.sin(angle)
        tangent_y = np.cos(angle)
        speed = rng.uniform(0.08, 0.24, AETHER_SWARM_COUNT)
        state[:, 3] = tangent_x * speed
        state[:, 4] = tangent_y * speed
        state[:, 5] = rng.normal(0, 0.025, AETHER_SWARM_COUNT)

        self.swarm_vbos = [int(x) for x in GL.glGenBuffers(2)]
        self.swarm_vaos = [int(x) for x in GL.glGenVertexArrays(2)]

        stride = 6 * 4
        for vao, vbo in zip(self.swarm_vaos, self.swarm_vbos):
            GL.glBindVertexArray(vao)
            GL.glBindBuffer(GL.GL_ARRAY_BUFFER, vbo)
            GL.glBufferData(GL.GL_ARRAY_BUFFER, state.nbytes, state, GL.GL_DYNAMIC_COPY)
            GL.glEnableVertexAttribArray(0)
            GL.glVertexAttribPointer(0, 3, GL.GL_FLOAT, False, stride, GL.ctypes.c_void_p(0))
            GL.glEnableVertexAttribArray(1)
            GL.glVertexAttribPointer(1, 3, GL.GL_FLOAT, False, stride, GL.ctypes.c_void_p(12))

        GL.glBindVertexArray(0)
        GL.glBindBuffer(GL.GL_ARRAY_BUFFER, 0)

        self.swarm_index = 0
        self.swarm_clock = time.monotonic()
        self.swarm_update_uniforms = {
            name: GL.glGetUniformLocation(self.swarm_update, name)
            for name in (
                "dt", "phase", "energy", "bass", "treble",
                "aetherOnset", "aetherDensity", "aetherWorldTurn", "aetherBeatPulse",
            )
        }
        self.swarm_render_uniforms = {
            name: GL.glGetUniformLocation(self.swarm_render, name)
            for name in (
                "resolution", "phase", "energy", "bass", "treble",
                "aetherOnset", "aetherDensity", "aetherWorldTurn",
                "aetherBeatPulse", "pixelRatio",
            )
        }

    def _update_aether_swarm(self):
        state = self.owner
        now = time.monotonic()
        dt = min(0.05, max(0.001, now - self.swarm_clock))
        self.swarm_clock = now

        source = self.swarm_index
        target = 1 - source

        GL.glUseProgram(self.swarm_update)
        for name, value in (
            ("dt", dt),
            ("phase", state.time),
            ("energy", state.energy),
            ("bass", state.bass),
            ("treble", state.treble),
            ("aetherOnset", state.aether_onset),
            ("aetherDensity", state.aether_density),
            ("aetherWorldTurn", state.aether_world_turn),
            ("aetherBeatPulse", state.aether_beat_pulse),
        ):
            loc = self.swarm_update_uniforms[name]
            if loc != -1:
                GL.glUniform1f(loc, value)

        GL.glEnable(GL.GL_RASTERIZER_DISCARD)
        GL.glBindVertexArray(self.swarm_vaos[source])
        GL.glBindBufferBase(GL.GL_TRANSFORM_FEEDBACK_BUFFER, 0, self.swarm_vbos[target])
        GL.glBeginTransformFeedback(GL.GL_POINTS)
        GL.glDrawArrays(GL.GL_POINTS, 0, AETHER_SWARM_COUNT)
        GL.glEndTransformFeedback()
        GL.glBindBufferBase(GL.GL_TRANSFORM_FEEDBACK_BUFFER, 0, 0)
        GL.glDisable(GL.GL_RASTERIZER_DISCARD)

        self.swarm_index = target

    def _draw_aether_swarm(self, width, height, ratio):
        state = self.owner
        GL.glUseProgram(self.swarm_render)

        u = self.swarm_render_uniforms
        GL.glUniform2f(u["resolution"], width, height)
        GL.glUniform1f(u["pixelRatio"], ratio)

        for name, value in (
            ("phase", state.time),
            ("energy", state.energy),
            ("bass", state.bass),
            ("treble", state.treble),
            ("aetherOnset", state.aether_onset),
            ("aetherDensity", state.aether_density),
            ("aetherWorldTurn", state.aether_world_turn),
            ("aetherBeatPulse", state.aether_beat_pulse),
        ):
            if u[name] != -1:
                GL.glUniform1f(u[name], value)

        GL.glEnable(GL.GL_BLEND)
        GL.glBlendFunc(GL.GL_SRC_ALPHA, GL.GL_ONE)
        GL.glBindVertexArray(self.swarm_vaos[self.swarm_index])
        GL.glDrawArrays(GL.GL_POINTS, 0, AETHER_SWARM_COUNT)
        GL.glBindVertexArray(0)
        GL.glDisable(GL.GL_BLEND)

    def common_uniforms(self, program, width, height):
        state = self.owner
        u = self.uniforms[program]
        GL.glUseProgram(program)
        GL.glUniform2f(u["resolution"], width, height)
        for name, value in (
                ("energy", state.energy), ("phase", state.time), ("bass", state.bass),
                ("treble", state.treble), ("phiPulse", state.phi_pulse),
                ("phiBloom", state.phi_bloom), ("phiTension", state.phi_tension),
                ("phiEvent", state.phi_event), ("phiVelocity", state.phi_velocity),
                ("phiImpulse", state.phi_impulse),
                ("aetherOnset", state.aether_onset),
                ("aetherBeatPhase", state.aether_beat_phase),
                ("aetherDensity", state.aether_density),
                ("aetherSceneMorph", state.aether_scene_morph),
                ("aetherWorldTurn", state.aether_world_turn),
                ("aetherBeatPulse", state.aether_beat_pulse)):
            location = u[name]
            if location is not None and location != -1:
                GL.glUniform1f(location, value)
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
            if state.mode in (0, 2, 4):
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
            if state.mode == 4:
                self.common_uniforms(self.phi, width, height)
                u = self.uniforms[self.phi]
                GL.glUniform1i(u["audioData"], 0)
                GL.glDrawArrays(GL.GL_TRIANGLES, 0, 3)

                self.common_uniforms(self.phi_particles, width, height)
                u = self.uniforms[self.phi_particles]
                GL.glUniform1i(u["audioData"], 0)
                GL.glUniform1f(u["pixelRatio"], ratio)
                GL.glEnable(GL.GL_BLEND)
                GL.glBlendFunc(GL.GL_SRC_ALPHA, GL.GL_ONE)
                GL.glDrawArrays(GL.GL_POINTS, 0, PHI_PARTICLE_COUNT)
                GL.glDisable(GL.GL_BLEND)

                self._update_aether_swarm()
                self._draw_aether_swarm(width, height, ratio)
            elif state.mode == 5:
                GL.glUseProgram(self.event_horizon)
                self.common_uniforms(self.event_horizon, width, height)
                GL.glDrawArrays(GL.GL_TRIANGLES, 0, 3)
            else:
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
        # Aether persistent swarm resources.
        if getattr(self, "swarm_vbos", None):
            GL.glDeleteBuffers(len(self.swarm_vbos), self.swarm_vbos)
            self.swarm_vbos = []
        if getattr(self, "swarm_vaos", None):
            GL.glDeleteVertexArrays(len(self.swarm_vaos), self.swarm_vaos)
            self.swarm_vaos = []
        for name in ("swarm_update", "swarm_render"):
            program = getattr(self, name, None)
            if program:
                GL.glDeleteProgram(program)
                setattr(self, name, None)
        if not self.context() or not self.context().isValid():
            return
        self.makeCurrent()
        if self.textures:
            GL.glDeleteTextures(self.textures)
            self.textures = []
        if self.vao:
            GL.glDeleteVertexArrays(1, [self.vao])
            self.vao = 0
        for program in (self.quad, self.particles, self.phi, self.phi_particles, self.event_horizon):
            if program:
                GL.glDeleteProgram(program)
        self.quad = self.particles = self.phi = self.phi_particles = self.event_horizon = 0
        self.ready = False
        self.doneCurrent()
