#version 450

layout(location=0) out vec2 v_Uv;

void main() {
    // Setup a single triangle
    float x = float((gl_VertexIndex & 1) << 2);
    float y = float((gl_VertexIndex & 2) << 1);
    v_Uv = vec2(x * 0.5, 1.0-y * 0.5);
    gl_Position = vec4(x - 1.0, y - 1.0, 0, 1);
}
