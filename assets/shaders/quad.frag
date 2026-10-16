#version 330 core
in vec2 uv;
out vec4 frag;
uniform int mode;
uniform vec2 resolution;
uniform vec3 background, accent, cyan, bright;
uniform float energy, phase, bass, treble;
uniform sampler2D audioData, historyData, waveDensity;
uniform float waveAlpha;
uniform float aetherOnset, aetherDensity, aetherBeatPulse;
float datum(int i) { return texelFetch(audioData, ivec2(i, 0), 0).r; }
float segdist(vec2 p, vec2 a, vec2 b) {
    vec2 pa = p-a, ba = b-a;
    float h = clamp(dot(pa,ba)/max(dot(ba,ba),0.00001),0.0,1.0);
    return length(pa-ba*h);
}
vec2 webNode(int i) {
    float fi=float(i);
    float z=0.5+0.5*sin(fi*1.61803398875+phase*0.11);
    float a=fi*2.39996322973+phase*(0.05+z*0.035);
    vec2 drift=vec2(sin(fi*0.73+phase*0.17),cos(fi*1.13-phase*0.13))*vec2(0.12,0.10);
    float spread=0.24+z*0.32+aetherDensity*0.07;
    return vec2(cos(a)*spread,sin(a)*spread*0.72)+drift;
}
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
    } else if (mode == 9) {
        vec2 p=(uv-0.5)*vec2(resolution.x/resolution.y,1.0);
        vec3 web=background*0.28;
        float pulse=aetherBeatPulse*0.85+aetherOnset*0.30;
        for(int i=0;i<18;i++){
            vec2 a=webNode(i);
            float level=datum((i*11+7)%96);
            float d=length(p-a);
            float halo=exp(-d*(55.0-18.0*level));
            vec3 tint=mix(cyan,accent,0.5+0.5*sin(float(i)*0.73+phase*0.08));
            web+=tint*halo*(0.12+level*0.62+pulse*0.35);
            web+=bright*exp(-d*180.0)*(0.18+level*0.72);
            for(int k=1;k<=2;k++){
                int j=(i+(k==1?3:7))%18;
                vec2 b=webNode(j);
                float ld=segdist(p,a,b);
                float reach=length(a-b);
                float gate=1.0-smoothstep(0.34+0.12*aetherDensity,0.58+0.12*aetherDensity,reach);
                float filament=exp(-ld*(165.0-35.0*energy))*gate;
                web+=tint*filament*(0.035+0.13*level+0.09*pulse);
            }
        }
        color=web;
    } else if (mode == 10) {
        vec2 center=vec2(0.5+sin(phase*0.23)*0.045,0.5+cos(phase*0.19+1.2)*0.040);
        vec2 p=(uv-center)*vec2(resolution.x/resolution.y,1.0);
        float r=length(p);
        float angle=atan(p.y,p.x);
        float travel=fract(r*6.8-phase*(0.42+energy*0.24));
        float ring=pow(max(0.0,1.0-abs(travel-0.5)*2.0),16.0);
        float spoke=pow(max(0.0,0.5+0.5*cos(angle*12.0+phase*0.65-r*18.0)),18.0);
        float historyX=clamp(1.0-r*1.28,0.0,1.0);
        float historyY=fract(angle/6.28318530718+0.5);
        vec3 memory=texture(historyData,vec2(historyX,historyY)).rgb;
        float memoryPower=max(memory.r,max(memory.g,memory.b));
        vec3 tint=mix(cyan,accent,0.5+0.5*sin(angle*2.0+r*8.0+phase*0.12));
        float beat=exp(-abs(travel-0.53)*28.0)*aetherBeatPulse;
        float fade=smoothstep(0.78,0.03,r);
        color=background*0.24;
        color+=memory*(0.42+memoryPower*0.85)*fade;
        color+=tint*ring*(0.12+memoryPower*0.85+energy*0.22)*fade;
        color+=mix(accent,bright,0.55)*beat*(0.22+bass*0.45)*fade;
        color+=tint*spoke*ring*(0.03+treble*0.08)*fade;
        float core=exp(-r*r*260.0);
        color+=mix(cyan,bright,0.72)*core*(0.65+aetherOnset*0.5);
    } else {
        color = mix(background, texture(historyData, vec2(uv.x, 1.0-uv.y)).rgb, 0.85);
    }
    frag = vec4(color, 1.0);
}
