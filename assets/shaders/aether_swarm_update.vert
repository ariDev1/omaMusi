#version 330 core
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
uniform float aetherBeatPhase;
uniform float aetherBeatConfidence;
uniform float phiImpulse;
uniform float phiEvent;
uniform float phiVelocity;

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

    // Continuous beat groove: even between triggers the shell breathes
    // with the beat cycle; strength follows beat confidence so ambient
    // music stays calm and locked grooves visibly pump.
    float beatCycle = sin(aetherBeatPhase*TAU + g*TAU);
    float groove = (0.25 + 0.75*aetherBeatConfidence);
    float dance = aetherBeatPulse*1.4 + aetherOnset*1.2 + phiImpulse*1.6 + phiEvent*1.8;
    // Saturating drive: loud passages compress toward 1 instead of stacking
    // linearly into instant white-out. Quiet detail stays, hits stay bounded.
    float drive = 1.0 - exp(-dance*0.9);

    vec3 center = vec3(
        1.05*sin(orbit + group*PHI),
        0.82*cos(orbit/PHI + group*0.37),
        1.95*sin(orbit*0.41 + group*0.73)
    );
    // Hit bounce: whole flock attractor jumps on transients instead of
    // only drifting on the slow orbit.
    center += vec3(
        sin(group*2.1 + phase*0.4),
        cos(group*1.7 - phase*0.33),
        sin(group*0.9 + phase*0.27)
    ) * (drive*0.35 + beatCycle*groove*0.08);

    vec3 toCenter = center - p;
    float dist = max(length(toCenter), 0.001);
    vec3 radial = toCenter / dist;

    // Shell structures persist, but breathe with the music instead of
    // acting as a rigid cage that fights every transient.
    float shell = mix(0.38, 1.48, phrase) * (1.0 + drive*0.38 + beatCycle*groove*0.06);
    float shellError = dist - shell;

    vec3 axis = normalize(vec3(
        0.35 + 0.4*sin(group),
        0.55 + 0.3*cos(group*PHI),
        0.45 + 0.2*sin(group*0.7)
    ));
    vec3 tangent = normalize(cross(radial, axis) + vec3(0.0001));

    float cohesion = 0.26 + aetherDensity*0.34 + energy*0.12;
    float orbitForce = (0.24 + treble*0.22 + 0.08*sin(id*0.13 + phase))
                     * (1.0 + phiVelocity*0.35 + drive*0.7);
    float repel = smoothstep(0.55, 0.10, dist) * (0.75 + bass*0.70 + drive*0.8);

    vec3 shellForce = -radial * shellError * (0.82 + 0.25*aetherDensity);

    vec3 curl = vec3(
        sin(p.y*1.9 + phase*0.21 + seed*TAU),
        cos(p.z*1.4 - phase*0.17 + seed*TAU),
        sin(p.x*1.6 + phase*0.13 - seed*TAU)
    ) * (0.15 + treble*0.50 + drive*0.90);

    vec3 drift = normalize(center + vec3(0.001)) * (0.05 + drive*0.45);

    // Kicks scaled ~4x up: previously tangent*1.15*dt moved velocity by
    // ~0.02/frame (invisible next to orbital speeds of ~0.4).
    // Explode: outward blast from the origin on hits so the whole flock
    // pumps outward, then the shell spring pulls it back (pump, not drift).
    vec3 dirOut = p / max(length(p), 0.35);
    float boom = drive*5.00 + beatCycle*groove*0.45;
    float perParticle = 0.45 + 1.10*fract(seed*7.31);
    vec3 blast = dirOut * boom * perParticle
               + tangent * (drive*3.40 + beatCycle*groove*0.55) * perParticle
               + radial * (bass*drive*1.60 + drive*2.20 + beatCycle*groove*0.45);
    vec3 beatKick = blast
                  + normalize(vec3(
                        sin(group + phase*0.2),
                        cos(group*0.7 - phase*0.13),
                        sin(seed*TAU + phase*0.17)
                    )) * drive*1.10;

    vec3 accel = vec3(0.0);
    vec3 flockAccel = radial*cohesion
               + shellForce
               + tangent*orbitForce
               - radial*repel
               + curl
               + drift
               + beatKick;

    // A broad ring anchors the scene. Stable membership keeps its gold
    // particles separate from the blue/violet surrounding swarm.
    float torusMask = step(hash1(id*0.37 + 5.0), 0.30);
    float uT = seed*TAU + phase*(0.38*(0.7 + drive*0.6 + phiVelocity*0.20)) + beatCycle*groove*0.10;
    float vT = fract(seed*7.77)*TAU*3.0 + phase*(1.10 + drive*0.80) + g*2.20;
    float ringR = 1.90*(1.0 + drive*0.14 + beatCycle*groove*0.05);
    float tubeR = 0.16*(1.0 + drive*0.45);
    float cuT = cos(uT), suT = sin(uT), cvT = cos(vT), svT = sin(vT);
    vec3 torusLocal = vec3((ringR + tubeR*cvT)*cuT, (ringR + tubeR*cvT)*suT, tubeR*svT);
    float tiltT = 0.42 + 0.16*sin(phase*0.09 + 1.0);
    float cTilt = cos(tiltT), sTilt = sin(tiltT);
    vec3 torusTilt = vec3(torusLocal.x, torusLocal.y*cTilt - torusLocal.z*sTilt,
                          torusLocal.y*sTilt + torusLocal.z*cTilt);
    float yawT = aetherWorldTurn*0.6 + phase*0.06;
    float cYaw = cos(yawT), sYaw = sin(yawT);
    // Restrained drift leaves the ring near the center and makes its
    // music-driven bend readable instead of wandering out of view.
    vec3 torusCenter = vec3(0.15*sin(phase*0.11 + aetherWorldTurn*0.40),
                            0.12*cos(phase*0.083 + 1.2) + drive*0.15*sin(phase*0.9),
                            0.12*sin(phase*0.067 + 2.1) + drive*0.20*sin(phase*1.1));
    vec3 torusTarget = vec3(torusTilt.x*cYaw - torusTilt.y*sYaw,
                            torusTilt.x*sYaw + torusTilt.y*cYaw,
                            torusTilt.z)
                     + torusCenter;
    vec3 torusTanLocal = vec3(-suT, cuT, 0.15*sin(vT*2.0 + phase*1.3));
    vec3 torusTanTilt = vec3(torusTanLocal.x, torusTanLocal.y*cTilt - torusTanLocal.z*sTilt,
                             torusTanLocal.y*sTilt + torusTanLocal.z*cTilt);
    vec3 torusTangent = normalize(vec3(torusTanTilt.x*cYaw - torusTanTilt.y*sYaw,
                                       torusTanTilt.x*sYaw + torusTanTilt.y*cYaw,
                                       torusTanTilt.z) + vec3(0.0001));
    vec3 torusAccel = (torusTarget - p)*3.8 + torusTangent*(1.10 + drive*1.20 + phiVelocity*0.40);
    accel = mix(flockAccel, torusAccel + shellForce*0.25 + curl*0.5, torusMask);

    v += accel * dt;
    // Snappier tracking: old 0.26 damping made velocity integrate for
    // seconds (floaty trails). ~1.4 follows kicks within a beat while
    // still smoothing jitter; extra damping on quiet passages.
    // Loosen damping during blasts so explosions actually fly.
    float blastEase = clamp(drive*0.9, 0.0, 0.9);
    v *= exp(-dt * ((1.35 + (1.0 - clamp(dance, 0.0, 1.5))*0.55) * (1.0 - blastEase*0.55)));

    float speed = length(v);
    float maxSpeed = 1.60 + energy*1.20 + drive*2.20;
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

