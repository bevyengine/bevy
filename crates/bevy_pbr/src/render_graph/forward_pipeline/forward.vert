#version 450

layout(location = 0) in vec3 position_os;
layout(location = 1) in vec3 normal_os;
layout(location = 2) in vec2 uv_vertex;

layout(location = 0) out vec3 position_world;
layout(location = 1) out vec3 normal_world;
layout(location = 2) out vec2 v_uv;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    normal_world = (Model * vec4(normal_os, 1.0)).xyz;
    normal_world = mat3(Model) * normal_os;
    position_world = (Model * vec4(position_os, 1.0)).xyz;
    v_uv = uv_vertex;
    gl_Position = ViewProj * vec4(position_os, 1.0);
}
