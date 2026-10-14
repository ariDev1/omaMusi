#version 330 core
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

    // Same dance conductor as PHI_FRAGMENT: shared beat + director timing
    // so particles and background breathe, orbit and pulse together.
    float beatTurn = aetherBeatPhase * 6.28318530718;
    float beatKick = aetherBeatPulse * 0.6 + phiEvent * 0.4;
    float sceneSway = (aetherSceneMorph - 0.5);
    float dancePhase = phase + aetherWorldTurn * 1.5 + beatTurn * 0.05;

    int bandIndex = int(mod(n*13.0,96.0));
    float band = datum(bandIndex);
    int waveIndex = int(mod(n*21.0,1024.0));
    float wave = datum(192+waveIndex);

    // Nuclei sit exactly on the background flock centers (c0/c1/c2),
    // so particle clusters orbit the same anchors as the backdrop swirls.
    int cluster = int(mod(n,3.0));
    vec2 nucleus;
    if (cluster == 0) {
        nucleus = vec2(0.34*sin(dancePhase*0.23 + sceneSway*0.4), 0.28*cos(dancePhase*0.19) + beatKick*0.030);
    } else if (cluster == 1) {
        nucleus = vec2(0.46*sin(dancePhase*0.17*PHI+1.4 + sceneSway*0.3), 0.31*cos(dancePhase*0.21/PHI+0.7) + beatKick*0.030);
    } else {
        nucleus = vec2(0.29*sin(dancePhase*0.27+3.1), 0.36*cos(dancePhase*0.15+2.5 + sceneSway*0.35) + beatKick*0.030);
    }

    float relZ;
    vec2 worldXY;

    if (!swarm) {
        float travel = fract(n*INV_PHI + dancePhase*(0.045+0.060*phiVelocity)
                             + band*0.035 + phiImpulse*0.030
                             + aetherBeatPhase*0.08 + aetherBeatPulse*0.05 + sceneSway*0.03);
        relZ = mix(0.55, 8.25, travel);

        float arm = mod(n, 21.0);
        float armAngle = arm * GOLDEN_ANGLE;
        float radial = (0.22 + sqrt(n/COUNT)*1.15)
                       * (0.78 + 0.16*sin(n*0.013 + dancePhase/PHI));
        float angle = armAngle + relZ*0.22 + dancePhase*0.10 + beatTurn*0.03 + beatKick*0.20 + wave*(0.08+phiTension*0.14);

        vec2 radialVec = vec2(cos(angle), sin(angle));
        vec2 tangent = vec2(-radialVec.y, radialVec.x);
        worldXY = radialVec * radial
                + tangent * (0.05 + 0.08*sin(dancePhase*0.7 + beatTurn*0.1 + n*0.017));
        worldXY += nucleus * 0.20;
    } else {
        float lane = fract(seed*3.137 + dancePhase*(0.11 + 0.22*phiVelocity)
                           + band*0.045 + phiImpulse*0.10
                           + aetherBeatPhase*0.08 + aetherBeatPulse*0.05 + sceneSway*0.03);
        relZ = mix(0.30, 6.20, lane) + float(cluster) * 0.08;

        float orbit = seed*6.28318530718
                    + dancePhase*(0.70 + band*0.05 + phiVelocity*0.20)
                    + aetherWorldTurn*(0.8 + float(cluster)*0.27)
                    + beatTurn*0.15 + aetherBeatPulse*0.45 + beatKick*0.30;
        float spread = 0.12 + 0.62*fract(n*0.754877666) + phiPulse*0.12 + beatKick*0.10 + aetherBeatPulse*0.06;
        vec2 radialVec = vec2(cos(orbit), sin(orbit));
        vec2 tangent = vec2(-radialVec.y, radialVec.x);

        // Same spin as the background flock so swirls rotate together.
        float flockSpin = dancePhase*(0.7 + phiVelocity*0.22) + beatTurn*0.15 + beatKick*0.8;
        float turn = sin(flockSpin + n*0.031 + wave*2.0);
        float flock = 0.16 + 0.26*treble + 0.18*phiImpulse + aetherDensity*0.18 + aetherOnset*0.20;

        worldXY = nucleus
                + radialVec * spread
                + tangent * (0.10 + 0.22*turn + flock*0.35)
                + vec2(sin(dancePhase*0.9 + beatTurn*0.1 + n*0.021),
                       cos(dancePhase*1.1 + beatTurn*0.1 + n*0.018))
                  * 0.05 * (0.4 + band);
    }

    // Same camera as the background ray origin so parallax matches.
    vec2 camera = vec2(0.18*sin(phase/PHI + aetherWorldTurn + beatTurn*0.05),
                       0.14*cos(phase/(PHI*PHI) - aetherWorldTurn*0.7 + sceneSway*0.1) + beatKick*0.01);
    vec2 projected = (worldXY - camera) / relZ;
    projected.x /= resolution.x / resolution.y;
    gl_Position = vec4(projected*1.72, 0.0, 1.0);

    float nearFactor = clamp(1.8/relZ, 0.35, 3.0);
    gl_PointSize = (swarm
                    ? (0.9 + band*2.5 + treble*1.8 + phiImpulse*2.0 + aetherBeatPulse*1.2 + beatKick*0.6)
                    : (0.8 + band*3.1 + treble*1.2 + phiImpulse*1.0 + aetherBeatPulse*0.6))
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
               ? (0.14 + band*0.46 + treble*0.18 + phiImpulse*0.10 + aetherBeatPulse*0.12 + beatKick*0.08)
               : (0.11 + band*0.52 + energy*0.16 + phiImpulse*0.05 + aetherBeatPulse*0.08))
              * smoothstep(swarm ? 6.20 : 8.25,
                           swarm ? 0.30 : 0.55,
                           relZ);
}
