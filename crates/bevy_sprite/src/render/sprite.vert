#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform Camera2d {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform Quad {
    vec2 Quad_Position;
    vec2 Quad_Size;
    float Quad_ZIndex;
};

void main() {
    v_Uv = Vertex_Uv;
    vec3 position = Vertex_Position * vec3(Quad_Size, 0.0);
    position = position + vec3(Quad_Position, Quad_ZIndex);
    gl_Position = ViewProj * vec4(position, 1.0);
}