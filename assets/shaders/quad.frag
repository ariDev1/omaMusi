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

        const vec3 C_CYAN    = vec3(0.02,0.70,1.00);
        const vec3 C_BLUE    = vec3(0.05,0.18,1.00);
        const vec3 C_VIOLET  = vec3(0.42,0.06,1.00);
        const vec3 C_MAGENTA = vec3(1.00,0.02,0.68);
        const vec3 C_PINK    = vec3(1.00,0.10,0.38);
        const vec3 C_ORANGE  = vec3(1.00,0.30,0.02);
        const vec3 C_GOLD    = vec3(1.00,0.72,0.10);
        const vec3 C_WHITE   = vec3(1.00,0.92,0.78);

        vec3 web=background*0.025;
        float pulse=clamp(aetherBeatPulse*0.95+aetherOnset*0.50,0.0,1.0);
        vec2 hubs[4];

        for(int i=0;i<4;i++){
            float fi=float(i);
            float a=phase*0.18+fi*1.57079632679+sin(phase*0.09+fi*0.9)*0.30;
            float orbit=0.11+0.05*fi+aetherDensity*0.04;
            hubs[i]=vec2(cos(a)*orbit,sin(a)*orbit*0.72);
            float hd=length(p-hubs[i]);

            vec3 outer=mix(C_VIOLET,C_MAGENTA,0.25+0.20*fi);
            vec3 mid=mix(C_MAGENTA,C_ORANGE,0.20+0.18*fi);
            web+=outer*exp(-hd*8.0)*(0.40+0.34*pulse);
            web+=mid*exp(-hd*22.0)*(0.32+0.30*pulse);
            web+=C_GOLD*exp(-hd*48.0)*(0.22+0.26*pulse);
            web+=C_WHITE*exp(-hd*100.0)*(0.16+0.22*pulse);
        }

        for(int i=0;i<60;i++){
            vec2 a=webNode(i);
            float level=datum((i*7+13)%96);
            int hub=i%4;
            vec2 h=hubs[hub];

            float chroma=0.5+0.5*sin(float(i)*0.37+phase*0.10+level*2.4);
            vec3 cool=mix(C_CYAN,C_VIOLET,chroma);
            vec3 hot=mix(C_MAGENTA,C_ORANGE,clamp(level*0.85+pulse*0.18,0.0,1.0));
            vec3 tint=mix(cool,hot,clamp(0.18+level*0.58+pulse*0.20,0.0,1.0));

            float ad=length(p-a);
            web+=tint*exp(-ad*(20.0-5.0*level))*(0.12+0.30*level+0.12*pulse);
            web+=mix(tint,C_WHITE,0.62)*exp(-ad*100.0)*(0.06+0.30*level);

            float hd=segdist(p,a,h);
            float reach=length(a-h);
            float trunk=exp(-hd*(78.0-12.0*energy))*(1.0-smoothstep(0.10,0.62+0.14*aetherDensity,reach));
            web+=tint*trunk*(0.24+0.38*level+0.20*pulse);
            web+=mix(tint,C_WHITE,0.52)*pow(trunk,1.7)*(0.10+0.15*pulse);

            float t=fract(phase*(0.22+0.05*level)+float(i)*0.071);
            vec2 pkt=mix(h,a,t);
            float pd=length(p-pkt);
            web+=mix(C_GOLD,C_WHITE,0.55)*exp(-pd*95.0)*(0.20+0.28*pulse);

            for(int k=0;k<2;k++){
                int j=(i+(k==0?5:17))%60;
                vec2 b=webNode(j);
                float reach2=length(a-b);
                float gate=1.0-smoothstep(0.08,0.46+0.24*aetherDensity,reach2);
                if(gate>0.001){
                    float ld=segdist(p,a,b);
                    float filament=exp(-ld*(102.0-14.0*energy))*gate;
                    vec3 fc=mix(tint,mix(C_MAGENTA,C_ORANGE,0.35),0.18+0.28*level);
                    web+=fc*filament*(0.18+0.36*level+0.16*pulse);
                    web+=mix(fc,C_WHITE,0.56)*pow(filament,1.8)*(0.06+0.08*pulse);

                    float tp=fract(phase*(0.14+0.03*float(k))+float(i)*0.043+float(k)*0.27);
                    vec2 mover=mix(a,b,tp);
                    float md=length(p-mover);
                    web+=mix(C_CYAN,C_WHITE,0.65)*exp(-md*125.0)*(0.10+0.18*pulse);
                }
            }
        }

        float vignette=smoothstep(1.25,0.18,length(p));
        color=web*vignette;
    } else if (mode == 11) {
        // Layered refractive lattice: fixed work per pixel keeps comparisons
        // repeatable while audio changes only color and motion.
        vec2 p = (uv-0.5)*vec2(resolution.x/resolution.y, 1.0);
        vec3 sum = vec3(0.0);
        float beat = 0.15 + bass*0.5 + aetherBeatPulse*0.35;
        for (int i=0; i<28; ++i) {
            float depth = float(i)/28.0;
            float scale = 1.0 + depth*7.0;
            float angle = phase*0.09 + depth*3.8;
            mat2 turn = mat2(cos(angle), -sin(angle), sin(angle), cos(angle));
            vec2 q = turn*p*scale;
            q += vec2(sin(phase*0.17+depth*8.0), cos(phase*0.13-depth*9.0))*0.34;
            vec2 grid = abs(fract(q)-0.5);
            float wire = exp(-min(grid.x,grid.y)*scale*18.0);
            float node = exp(-length(grid)*scale*15.0);
            float band = datum((i*13)%96);
            vec3 hue = 0.5+0.5*cos(vec3(0.0,2.1,4.2)+depth*7.0+phase*0.13);
            sum += mix(cyan, hue, 0.72)*(wire*0.022+node*0.07)*(0.5+band*0.8+beat);
        }
        float core = exp(-dot(p,p)*12.0);
        color = background*0.07 + sum + mix(accent,bright,0.65)*core*(0.14+energy*0.25);
        color *= 1.0-smoothstep(0.55,1.1,length(p))*0.7;
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
