#version 330 core
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
