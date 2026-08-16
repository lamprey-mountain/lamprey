#version 300 es

in vec2 a_position;
out vec2 v_texCoord;

void main() {
    // Map position from [0, 1] space to clip space [-1, 1]
    gl_Position = vec4(a_position * 2.0 - 1.0, 0.0, 1.0);
    v_texCoord = a_position;
}
