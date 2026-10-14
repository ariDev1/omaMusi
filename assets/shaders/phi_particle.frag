#version 330 core
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
