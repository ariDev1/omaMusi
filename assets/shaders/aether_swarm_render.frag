#version 330 core
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

    frag = vec4(tint, clamp(alpha, 0.0, 0.85));
}

