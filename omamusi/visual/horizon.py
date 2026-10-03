"""Reference-composed accretion disk and gravitationally bent disk image."""

import numpy as np


class HorizonHotspots:
    """Bounded transient events with identities that survive slot changes."""

    LIFETIME = 4.2

    def __init__(self):
        self.reset()

    def reset(self):
        self.spots = []
        self.cooldown = 0.0
        self.previous_onset = 0.0
        self.serial = 0

    def update(self, dt, metrics, active):
        self.spots = [[age + dt, strength, seed] for age, strength, seed in self.spots
                      if age + dt < self.LIFETIME]
        self.cooldown = max(0.0, self.cooldown - dt)
        transient = max(metrics.mid_transient, metrics.high_transient * 0.7,
                        metrics.low_transient * 0.6)
        onset_rise = max(0.0, metrics.onset - self.previous_onset)
        self.previous_onset = metrics.onset
        if active and self.cooldown == 0 and (transient > 0.10 or onset_rise > 0.10):
            strength = min(1.0, max(transient * 2.0, onset_rise * 2.0))
            seed = (self.serial * 0.61803398875 + 0.23) % 1.0
            self.serial += 1
            self.spots.append([0.0, strength, seed])
            self.spots = self.spots[-4:]
            self.cooldown = 0.24

    def payload(self):
        result = np.zeros((4, 3), dtype=np.float32)
        result[:, 0] = -1.0
        for i, spot in enumerate(self.spots):
            result[i] = spot
        return result


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
uniform vec3 audioSpots[4];
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
    // Only the near-facing disk surface is seen directly. Its far/return
    // half is represented by the lensed horseshoe, not a second visible
    // ellipse above the crest. The upper edge feathers over a few pixels.
    float nearSurface=1.0-smoothstep(0.0,0.018,y);
    skirt*=nearSurface;
    crest*=1.0-smoothstep(0.0,0.032,y);
    // Unwrap the visible surface's filaments. Sampling full elliptical
    // radius here drew closed oval ends (the unwanted returning stream).
    // Angle still advects the light orbitally, while lanes run along the
    // near disk face and continue off-screen instead of turning back.
    float surfaceLane=0.62-y/0.22+0.055*log(1.0+abs(p.x)*2.0);
    float texture=strands(surfaceLane,diskA);
    float diskFade=exp(-max(abs(p.x)-0.65,0.0)*0.55);
    float light=(crest*3.8+skirt*1.15)*texture*diskFade*exposure;
    color+=thermal(diskHeat)*light;
    color+=vec3(1.0,0.93,0.77)*crest*diskHeat*2.0;
    // Broad, localized bloom at the junction and luminous disk crest.
    color+=vec3(1.0,0.76,0.45)*exp(-abs(y)/0.09)*diskFade*0.25
          *(0.15+0.85*nearSurface);
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

    // Transient-born hot spots keep stable identities as old events expire.
    // The foreground image disappears behind the shadow; delayed images
    // of the same orbit emerge on the upper and lower lensed disk bands.
    vec3 spotLight=vec3(0.0);
    for (int i=0;i<4;++i) {
        float age=audioSpots[i].x;
        if (age<0.0 || age>=4.2) continue;
        float strength=audioSpots[i].y;
        float seed=audioSpots[i].z;
        float speed=1.35+seed*0.40;
        float orbit=seed*TAU+age*speed;
        float orbitR=0.58+seed*0.21;
        float fade=smoothstep(0.0,0.09,age)*(1.0-smoothstep(1.8,4.2,age));
        vec2 spot=vec2(cos(orbit)*orbitR,sin(orbit)*0.12-0.19);
        float foreground=1.0-smoothstep(-0.12,0.20,sin(orbit));
        float visibility=mix(1.0-shadow,1.0,foreground);
        float core=exp(-dot(p-spot,p-spot)/(0.009*0.009));
        float halo=exp(-dot(p-spot,p-spot)/(0.025*0.025))*0.20;
        float tail=0.0;
        for (int t=1;t<=5;++t) {
            float lag=float(t)*0.06;
            vec2 trail=vec2(cos(orbit-lag)*orbitR,sin(orbit-lag)*0.12-0.19);
            vec2 d=p-trail;
            float width=0.009+float(t)*0.001;
            tail+=exp(-dot(d,d)/(width*width))*exp(-float(t)*0.45)*0.32;
        }
        spotLight+=(vec3(1.0,0.91,0.70)*core+vec3(1.0,0.46,0.14)*(halo+tail))
                  *fade*strength*visibility*1.25;

        for (int echo=0;echo<2;++echo) {
            float delay=echo==0 ? 0.26 : 0.62;
            float echoAge=age-delay;
            if (echoAge<=0.0) continue;
            float pastOrbit=seed*TAU+echoAge*speed;
            float behind=smoothstep(-0.15,0.35,sin(pastOrbit));
            float echoFade=smoothstep(0.0,0.12,echoAge)
                          *(1.0-smoothstep(1.8,4.2,age));
            float upperAngle=atan(abs(sin(pastOrbit))*0.9+0.18,cos(pastOrbit));
            float upper=exp(-(1.0-cos(angle-upperAngle))*190.0);
            float lower=exp(-(1.0-cos(angle+upperAngle))*190.0)*0.65;
            float arcTail=exp(-(1.0-cos(angle-upperAngle+0.15))*55.0)*0.22
                         +exp(-(1.0-cos(angle+upperAngle+0.15))*55.0)*0.14;
            float lane=0.040+seed*0.09+float(echo)*0.023;
            float band=exp(-pow((offset-lane)/0.010,2.0));
            float attenuation=echo==0 ? 0.75 : 0.28;
            spotLight+=vec3(1.0,0.72,0.38)*(upper+lower+arcTail)*band
                      *behind*echoFade*strength*attenuation*lensGate;
        }
    }
    color+=min(spotLight,vec3(1.4));

    color=vec3(1.0)-exp(-max(color,vec3(0.0))*1.12);
    frag=vec4(color,1.0);
}
"""
