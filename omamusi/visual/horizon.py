"""Reference-composed accretion disk and gravitationally bent disk image."""

import numpy as np


def wave_payload(bursts):
    """Upload real transient ages; negative ages mark unused wave slots."""
    result = np.zeros((5, 2), dtype=np.float32)
    result[:, 0] = -1.0
    for i, (age, strength) in enumerate(bursts[-5:]):
        result[i] = age, strength
    return result


REFERENCE_EVENT_HORIZON_FRAGMENT = """#version 330 core
in vec2 uv;
out vec4 frag;
uniform vec2 resolution;
uniform float phase, energy, bass, treble, phiPulse, phiEvent;
uniform float aetherOnset, aetherBeatPulse, aetherWorldTurn;
uniform sampler2D audioData;
uniform vec2 audioWaves[5];
const float TAU = 6.28318530718;

float hash(vec2 p) { return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453); }
float noise(vec2 p) {
    vec2 i=floor(p), f=fract(p); f=f*f*(3.0-2.0*f);
    return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),
               mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x),f.y);
}
mat2 rotate(float a) { return mat2(cos(a),-sin(a),sin(a),cos(a)); }

// Texture uses angle as the travel coordinate and radius as the strand
// coordinate: filaments circulate around the disk, rather than radiate.
float strands(float radius, float angle) {
    float travel=angle-phase*0.16-aetherWorldTurn*0.12;
    // Periodic angular coordinates avoid an atan seam through the ring.
    vec2 orbit=vec2(cos(travel),sin(travel));
    float warp=noise(orbit*3.0+vec2(radius*9.0,radius*4.0));
    float fine=0.5+0.5*sin(radius*150.0+warp*6.0+sin(travel*7.0)*0.7);
    float broad=0.5+0.5*sin(radius*53.0+warp*4.0);
    float knots=noise(orbit*14.0+vec2(radius*32.0,radius*13.0));
    return 0.52+0.20*fine*fine+0.15*broad+0.23*knots;
}
vec3 thermal(float heat) {
    vec3 copper=vec3(0.95,0.32,0.10);
    vec3 cream=vec3(1.0,0.79,0.48);
    return mix(copper,cream,smoothstep(0.12,0.75,heat));
}

void main() {
    float aspect=resolution.x/resolution.y;
    vec2 screen=(uv-0.5)*vec2(aspect,1.0);
    // Stable close-up framing. Slow drift does not turn the disk face-on
    // or change the reference's silhouette on every musical transient.
    vec2 center=vec2(aspect*0.32,0.14);
    center+=vec2(sin(phase*0.018)*0.015,cos(phase*0.014)*0.012);
    vec2 p=rotate(0.31+0.012*sin(phase*0.025))*(screen-center);
    float radius=0.49;
    float r=length(p);
    float angle=atan(p.y,p.x);
    float hit=1.0-exp(-(aetherOnset+aetherBeatPulse+phiEvent)*0.65);
    float exposure=1.0+energy*0.12+hit*0.10;

    // Cool empty space and a faint reflected-light veil around the hole.
    vec3 color=vec3(0.004,0.011,0.013);
    color+=vec3(0.020,0.032,0.035)*exp(-abs(r-radius)*3.0)*0.6;
    float shadow=1.0-smoothstep(radius-0.002,radius+0.003,r);
    color*=1.0-shadow*0.97;

    // Back-side disk bent into a broad continuous upper/lower horseshoe.
    // One envelope, with concentric orbital strands, replaces detached arcs.
    float offset=r-radius;
    float lensGate=smoothstep(-0.015,0.018,offset);
    float lensBody=exp(-pow((offset-0.105)/0.105,2.0))*lensGate;
    float lensHeat=exp(-max(offset,0.0)*6.0);
    float lensTexture=strands(r,angle);
    float lensAsymmetry=0.74+0.26*cos(angle+0.2);
    color+=thermal(lensHeat)*lensBody*lensTexture*lensAsymmetry*3.1*exposure;
    // A single fine photon edge follows the same black-hole boundary.
    float photon=exp(-abs(offset-0.010)/0.0028);
    color+=vec3(1.0,0.91,0.72)*photon*0.85;
    color+=vec3(1.0,0.70,0.37)*exp(-abs(offset-0.060)/0.065)*lensGate*0.24;

    // Each detected bass transient launches one wave. Real elapsed ages
    // prevent looping rings or false emissions when playback is quiet.
    float waveLight=0.0;
    for (int i=0;i<5;++i) {
        float age=audioWaves[i].x;
        if (age<0.0 || age>=1.8) continue;
        float waveR=radius+0.065+age*0.38;
        float delta=r-waveR;
        float front=exp(-pow(delta/0.008,2.0));
        float wake=exp(-abs(delta)/0.025)*0.25;
        float fade=smoothstep(0.0,0.07,age)*(1.0-smoothstep(0.45,1.8,age));
        float arcs=0.30+0.70*pow(0.5+0.5*cos(angle*3.0-age*1.7+float(i)),2.0);
        waveLight+=(front+wake)*fade*arcs*audioWaves[i].y;
    }
    color+=vec3(0.80,0.53,0.28)*waveLight*0.26*lensGate;

    // Foreground disk crosses the lower part of the black silhouette.
    // Its bright crest is diagonal on screen; circular coordinates supply
    // the texture, not straight lightning or outward radial spokes.
    float y=p.y+0.19;
    vec2 disk=vec2(p.x,y/0.22);
    float diskR=length(disk);
    float diskA=atan(disk.y,disk.x);
    float diskHeat=exp(-abs(diskR-0.62)*1.5);
    float thickness=0.021+diskHeat*0.027;
    float crest=exp(-pow(y/thickness,2.0));
    float skirt=exp(-abs(y)/(thickness*2.1));
    float texture=strands(diskR,diskA);
    float diskFade=exp(-max(abs(p.x)-0.65,0.0)*0.55);
    float light=(crest*3.8+skirt*1.15)*texture*diskFade*exposure;
    color+=thermal(diskHeat)*light;
    color+=vec3(1.0,0.93,0.77)*crest*diskHeat*2.0;
    // Broad, localized bloom at the junction and luminous disk crest.
    color+=vec3(1.0,0.76,0.45)*exp(-abs(y)/0.09)*diskFade*0.25;
    color+=vec3(1.0,0.79,0.52)*exp(-length(p-vec2(-radius,-0.19))*10.0)*0.7;

    // Sparse spectrum-lit streaks orbit on disk lanes and the lensed band.
    // The stronger notes select brighter lanes; transients reveal their
    // short moving heads and fading tails, rather than brightening all light.
    float diskStreaks=0.0;
    float lensStreaks=0.0;
    for (int lane=0;lane<7;++lane) {
        float n=float(lane);
        int band=18+lane*11;
        float level=texelFetch(audioData,ivec2(band,0),0).r;
        float strength=level*(0.12+hit*0.88);
        float orbit=phase*(0.24+n*0.013)+n*2.39996+aetherWorldTurn*0.12;
        float diskHead=exp(-(1.0-cos(diskA-orbit))*110.0);
        float diskTail=exp(-(1.0-cos(diskA-orbit+0.13))*35.0)*0.32;
        float diskLane=exp(-pow((diskR-(0.55+n*0.11))/0.010,2.0));
        diskStreaks+=(diskHead+diskTail)*diskLane*strength;
        float lensHead=exp(-(1.0-cos(angle-orbit))*120.0);
        float lensTail=exp(-(1.0-cos(angle-orbit+0.16))*38.0)*0.30;
        float lensLane=exp(-pow((offset-(0.035+n*0.021))/0.006,2.0));
        lensStreaks+=(lensHead+lensTail)*lensLane*strength;
    }
    color+=vec3(1.0,0.83,0.57)*(diskStreaks*skirt*diskFade+ lensStreaks*lensGate)*0.85;

    color=vec3(1.0)-exp(-max(color,vec3(0.0))*1.12);
    frag=vec4(color,1.0);
}
"""
