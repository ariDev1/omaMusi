#version 330 core
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
    // Shared dance conductor: identical math in PHI_FRAGMENT and
    // PHI_PARTICLE_VERTEX so background and particles move as one.
    // Beat + director terms (beatPhase/beatPulse/worldTurn/sceneMorph)
    // ride on top of the ambient phase instead of replacing it.
    float beatTurn = aetherBeatPhase * TAU;
    float beatKick = aetherBeatPulse * 0.6 + phiEvent * 0.4;
    float sceneSway = (aetherSceneMorph - 0.5);
    float dancePhase = phase + aetherWorldTurn * 1.5 + beatTurn * 0.05;
    vec3 ro = vec3(0.18*sin(phase/PHI + aetherWorldTurn + beatTurn*0.05), 0.14*cos(phase/(PHI*PHI) - aetherWorldTurn*0.7 + sceneSway*0.1) + beatKick*0.01, phase*0.18*phiVelocity);
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

    vec2 c0 = vec2(0.34*sin(dancePhase*0.23 + sceneSway*0.4), 0.28*cos(dancePhase*0.19) + beatKick*0.030);
    vec2 c1 = vec2(0.46*sin(dancePhase*0.17*PHI+1.4 + sceneSway*0.3), 0.31*cos(dancePhase*0.21/PHI+0.7) + beatKick*0.030);
    vec2 c2 = vec2(0.29*sin(dancePhase*0.27+3.1), 0.36*cos(dancePhase*0.15+2.5 + sceneSway*0.35) + beatKick*0.030);

    vec2 d0 = screen - c0;
    vec2 d1 = screen - c1;
    vec2 d2 = screen - c2;

    float r0 = max(length(d0), 0.001);
    float r1 = max(length(d1), 0.001);
    float r2 = max(length(d2), 0.001);

    float a0 = atan(d0.y, d0.x);
    float a1 = atan(d1.y, d1.x);
    float a2 = atan(d2.y, d2.x);

    float flockSpin = dancePhase*(0.7 + phiVelocity*0.22) + beatTurn*0.15 + beatKick*0.8;
    float flock0 = exp(-r0*4.4) * exp(-abs(sin(a0*6.0  - flockSpin - r0*14.0))*11.5);
    float flock1 = exp(-r1*4.0) * exp(-abs(sin(a1*7.0  + flockSpin*0.85 - r1*16.0))*10.0);
    float flock2 = exp(-r2*3.7) * exp(-abs(sin(a2*8.0  - flockSpin*1.1 - r2*18.0))*9.5);

    float swirl = (flock0 + flock1 + flock2) * (0.012 + treble*0.09 + phiImpulse*0.18);
    color += mix(cyan, bright, 0.28+0.50*treble) * swirl;

    float shock = exp(-abs(length(screen) - fract(dancePhase*0.7*phiVelocity + aetherBeatPhase*0.35 + sceneSway*0.10)*1.35)*28.0);
    color += gold * shock * (0.01 + phiImpulse*0.16 + phiEvent*0.10 + aetherOnset*0.14 + aetherBeatPulse*0.10);
    frag = vec4(color, 1.0);
}
