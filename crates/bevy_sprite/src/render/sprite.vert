#version 450

layout(location = 0) in vec3 position_os;
layout(location = 1) in vec3 normal_os;
layout(location = 2) in vec2 uv_vertex;

layout(location = 0) out vec2 v_uv;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};
layout(set = 2, binding = 1) uniform Sprite_size {
    vec2 size;
};

void main() {
    v_uv = uv_vertex;
    vec3 position = position_os * vec3(size, 1.0);
    gl_Position = ViewProj * Model * vec4(position, 1.0);
}