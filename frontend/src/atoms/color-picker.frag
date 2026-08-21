#version 300 es
precision highp float;

uniform float u_hue;        // hue in degrees (0.0 to 360.0)
uniform float u_maxChroma;
uniform int u_colorSpace;   // 0 for oklch, 1 for hsl (interpreted as hsv for vertical axis)
in vec2 v_texCoord;
out vec4 outColor;

// convert Linear sRGB to Standard sRGB (gamma correction)
float srgbTransfer(float c) {
    return c <= 0.0031308 ? 12.92 * c : 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}

vec3 oklch_to_srgb(float L, float C, float hue_degrees) {
    float hueRad = radians(hue_degrees);
    float a = C * cos(hueRad);
    float b = C * sin(hueRad);

    // Oklab to LMS
    float l_ = L + 0.3963377774 * a + 0.2158037573 * b;
    float m_ = L - 0.1055613458 * a - 0.0638541728 * b;
    float s_ = L - 0.0894841775 * a - 1.2914855480 * b;

    // LMS non-linear to linear
    float l = l_ * l_ * l_;
    float m = m_ * m_ * m_;
    float s = s_ * s_ * s_;

    // LMS to Linear sRGB
    float rLinear =  4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    float gLinear = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    float bLinear = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    // Apply gamma and clamp to [0.0, 1.0]
    return vec3(
        srgbTransfer(clamp(rLinear, 0.0, 1.0)),
        srgbTransfer(clamp(gLinear, 0.0, 1.0)),
        srgbTransfer(clamp(bLinear, 0.0, 1.0))
    );
}

vec3 hsv_to_rgb(float h, float s, float v) {
    float c = v * s;
    float x = c * (1.0 - abs(mod(h / 60.0, 2.0) - 1.0));
    float m = v - c;

    vec3 rgb;
    if (h < 60.0) rgb = vec3(c, x, 0.0);
    else if (h < 120.0) rgb = vec3(x, c, 0.0);
    else if (h < 180.0) rgb = vec3(0.0, c, x);
    else if (h < 240.0) rgb = vec3(0.0, x, c);
    else if (h < 300.0) rgb = vec3(x, 0.0, c);
    else rgb = vec3(c, 0.0, x);

    return rgb + m;
}

void main() {
    vec3 rgb;
    if (u_colorSpace == 1) {
        float V = v_texCoord.y;
        float S = v_texCoord.x;
        rgb = hsv_to_rgb(u_hue, S, V);
    } else {
        float L = v_texCoord.y;
        float C = v_texCoord.x * u_maxChroma;
        rgb = oklch_to_srgb(L, C, u_hue);
    }
    outColor = vec4(rgb, 1.0);
}
