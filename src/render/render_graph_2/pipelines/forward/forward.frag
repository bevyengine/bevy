#version 450

layout(location = 0) in vec4 v_Position;
layout(location = 1) in vec3 v_Normal;
layout(location = 2) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 1, binding = 1) uniform StandardMaterial {
    vec4 Albedo;
};

void main() {
    // multiply the light by material color
    o_Target = vec4(1.0, 0.0, 0.0, 1.0);
    o_Target = Albedo;
}
