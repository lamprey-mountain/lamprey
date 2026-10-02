#version 300 es

precision highp float;
uniform float u_time;
uniform vec2 u_res;
out vec4 fragColor;

float rand(vec2 co) {
    return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
}

void main() {
    vec2 uv = gl_FragCoord.xy / u_res;
    float noise = rand(uv * u_time) * 0.1 + 0.05;
    float scanline = sin(uv.y * u_res.y * 0.5 + u_time * 10.0) * 0.005;
    float c = noise + scanline;
    fragColor = vec4(vec3(c), 1.0);
}
