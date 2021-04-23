#version 450

layout(location=0) out vec2 v_Uv;

void main() {
    float x = float(((uint(gl_VertexIndex) + 2u) / 3u)%2u);
    float y = float(((uint(gl_VertexIndex) + 1u) / 3u)%2u);

    gl_Position = vec4(-1.0f + x*2.0f, -1.0f+y*2.0f, 0.0f, 1.0f);
    v_Uv = vec2(x, 1.0-y);
}
