#version 300 es

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

out vec2 v_Uv;

layout(std140) uniform Camera {
    mat4 ViewProj;
};

layout(std140) uniform Transform {
    mat4 Model;
};

layout(std140) uniform Sprite_size {
    vec2 size;
};

void main() {
    v_Uv = Vertex_Uv;
    vec3 position = Vertex_Position * vec3(size, 1.0);
    gl_Position = ViewProj * Model * vec4(position, 1.0);
}
