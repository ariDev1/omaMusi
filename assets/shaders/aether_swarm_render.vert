#version 330 core
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
uniform float aetherBeatPhase;
uniform float aetherBeatConfidence;
uniform float phiImpulse;
uniform float phiEvent;
uniform float pixelRatio;
uniform sampler2D audioData;
uniform float canary;

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

    // Ease the framing back slightly during energetic passages so the
    // ring can bend without crowding the edges. Dots keep their size.
    float framing = 2.45/(1.0 + 0.20*clamp(energy, 0.0, 1.0));
    gl_Position = vec4(projected*framing, 0.0, 1.0);

    float speed = length(vel);
    // Per-particle spectrum band: neighbours dance to different parts
    // of the music instead of all pulsing on the global smoothed level.
    int bandIdx = int(mod(float(gl_VertexID)*13.0, 96.0));
    float band = texelFetch(audioData, ivec2(bandIdx, 0), 0).r;
    // Beat-wave (locked groove) + free shimmer (always moving): with no
    // BPM lock beatPhase freezes, so a pure beat wave would freeze too.
    float beatWave = 0.5 + 0.5*sin(aetherBeatPhase*6.28318530718 + float(gl_VertexID)*0.021);
    float shimmer = 0.5 + 0.5*sin(phase*2.2 + float(gl_VertexID)*0.11 + float(gl_VertexID)*0.013);
    float wave = clamp(beatWave*(0.25 + 0.75*aetherBeatConfidence) + shimmer*0.55, 0.0, 1.4);
    float dance = aetherBeatPulse + aetherOnset + phiImpulse + phiEvent;
    // Same saturating drive as the physics: loud music compresses toward
    // 1 instead of stacking linearly into big white blobs.
    float drive = 1.0 - exp(-dance*0.9);
    // Music moves the shape, not the dot size. Small points keep packed
    // areas detailed instead of turning into large overlapping blobs.
    gl_PointSize = 2.6*pixelRatio;

    vec2 dir = vel.xy;
    float dirLen = length(dir);
    streakDir = dirLen > 0.0001 ? dir / dirLen : vec2(1.0, 0.0);
    streakMix = clamp(speed*1.4 + aetherBeatPulse*0.9 + aetherOnset*0.45 + phiImpulse*0.6 + phiEvent*0.5 + band*0.5, 0.0, 1.0);

    float id = float(gl_VertexID);
    float torusSeed = id*0.37 + 5.0;
    float torusMask = step(fract(sin(torusSeed*12.9898 + 78.233)*43758.5453), 0.30);
    float groupShade = mod(id,13.0)/13.0;
    vec3 surroundingColor = mix(vec3(0.12,0.56,0.94), vec3(0.42,0.22,0.78), groupShade);
    vec3 base = mix(surroundingColor, vec3(1.00,0.68,0.20), torusMask);

    float nearGlow = clamp(1.6/safeDepth, 0.0, 1.0);
    // Bounded light output adds depth and a gentle musical shimmer while
    // the palette stays coherent and particle size stays fixed.
    float brightness = clamp(0.72 + energy*0.08 + min(speed,2.0)*0.06
                             + nearGlow*0.10 + drive*0.12 + wave*0.03, 0.65, 0.96);
    tint = base * brightness;
    opacity = clamp(0.10 + min(speed,2.0)*0.04 + drive*0.04, 0.10, 0.24);
    // Debug canary (OMA_PARTICLE_CANARY=1): unmistakable giant red dots.
    // Proves the live shader is on screen. No-op when canary is 0.
    gl_PointSize = max(gl_PointSize, canary*26.0*pixelRatio);
    tint = mix(tint, vec3(1.0, 0.08, 0.15), canary*0.95);
    opacity = max(opacity, canary*0.9);
}

