#version 450

layout(location = 0) in vec2 uv;

layout(location = 0) out vec4 o_Target;


void main() {
    o_Target = vec4(uv.x, uv.y, 0.0, 1.0);
}
