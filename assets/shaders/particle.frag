#version 330 core
in vec3 tint;
in float opacity;
in vec2 streakUV;
out vec4 frag;
void main() {
    float light = exp(-streakUV.y*streakUV.y*4.5);
    light *= pow(streakUV.x,0.75)*smoothstep(0.0,0.07,1.0-streakUV.x);
    frag = vec4(tint, light*opacity);
}
