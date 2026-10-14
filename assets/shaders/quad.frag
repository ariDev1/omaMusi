#version 330 core
in vec2 uv;
out vec4 frag;
uniform int mode;
uniform vec2 resolution;
uniform vec3 background, accent, cyan, bright;
uniform float energy, phase, bass;
uniform sampler2D audioData, historyData, waveDensity;
uniform float waveAlpha;
float datum(int i) { return texelFetch(audioData, ivec2(i, 0), 0).r; }
void main() {
    vec3 color = background;
    if (mode == 0) {
        // Extreme rollercoaster: the vanishing point whips along a big
        // two-frequency flight path and the whole tunnel banks hard.
        vec2 flight = vec2(sin(phase*0.45)*0.16 + sin(phase*1.10+1.7)*0.07,
                           cos(phase*0.33)*0.13 + sin(phase*0.80+0.6)*0.05)
                      * (0.7+energy*0.9);
        vec2 warpCenter = vec2(0.57, 0.53) + flight;
        float bank = sin(phase*0.31)*0.65 + bass*0.18 + energy*0.10;
        float cb = cos(bank), sb = sin(bank);
        // Dive surges: the view zooms as the music pushes.
        float zoom = 1.0 + 0.12*sin(phase*0.9) + energy*0.15 + bass*0.08;
        vec2 p = (uv - warpCenter) * vec2(resolution.x / resolution.y, 1.0) / zoom;
        p = mat2(cb, -sb, sb, cb) * p;
        float radius = length(p);
        float depth = -log(max(radius, 0.035));
        float angle = atan(p.y, p.x+0.000001);
        float spiral = pow(0.5+0.5*sin(angle*5.0+depth*7.0-phase*3.0), 18.0);
        float rings = pow(0.5+0.5*sin(depth*18.0+phase*11.0), 22.0);
        float haze = pow(0.5+0.5*sin(angle*3.0+depth*5.0-phase*1.7), 4.0);
        float opening = smoothstep(0.035, 0.075, radius);
        // Subtle breathing rim instead of a hard ring.
        float rimRadius = 0.059 + bass*0.010 + energy*0.006 + sin(phase*2.3)*0.0035;
        float rim = exp(-abs(radius-rimRadius)*230.0);
        vec3 tint = mix(cyan, accent, 0.5+0.5*sin(angle+depth*1.2+phase*0.15));
        float wall = (spiral*0.17+rings*0.04+haze*0.025)*exp(-radius*1.2);
        color = mix(background*0.25, background, opening);
        color += tint*wall*opening*(0.65+energy*1.2+bass*0.25);
        color += mix(cyan,bright,0.5)*rim*(0.045+energy*0.075+bass*0.03);
    } else if (mode == 1) {
        int band = clamp(int(uv.x * 96.0), 0, 95);
        float level = datum(band), peak = datum(96 + band);
        float y = uv.y - 0.24;
        float inset = min(0.3, 2.0 / (resolution.x / 96.0));
        float edge = fract(uv.x * 96.0);
        float bar = step(inset, edge) * step(edge, 1.0 - inset);
        if (y >= 0.0 && y <= max(2.0 / resolution.y, level * 0.52)) {
            vec3 tint = mix(cyan, accent, clamp(y / 0.3, 0.0, 1.0));
            tint = mix(tint, bright, clamp((y - 0.3) / 0.22, 0.0, 1.0) * 0.55);
            color = mix(color, tint, bar * (0.35 + 0.5 * y / 0.52));
        }
        float peakLine = 1.0 - smoothstep(0.7, 1.5, abs(y - peak * 0.52 - 3.0/resolution.y) * resolution.y);
        color += accent * peakLine * bar * 0.65;
    } else if (mode == 2) {
        float x = clamp(uv.x * 1023.0, 0.0, 1023.0);
        int i = int(x);
        float sampleValue = mix(datum(192+i), datum(192+min(i+1,1023)), fract(x));
        float distance = abs(uv.y - (0.53 + sampleValue * 0.28)) * resolution.y;
        vec4 heat = texture(waveDensity, vec2(uv.x, 1.0-uv.y));
        float line = exp(-distance * distance * 0.8) * waveAlpha;
        float alpha = line + heat.a * (1.0-line);
        // Qt's window composition consumes premultiplied color. Empty density
        // pixels remain truly transparent, rather than painting a dark panel.
        color = heat.rgb * heat.a * (1.0-line) + vec3(0.92, 0.99, 1.0) * line;
        frag = vec4(color, alpha);
        return;
    } else {
        color = mix(background, texture(historyData, vec2(uv.x, 1.0-uv.y)).rgb, 0.85);
    }
    frag = vec4(color, 1.0);
}
