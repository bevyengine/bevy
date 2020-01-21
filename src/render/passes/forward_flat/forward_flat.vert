#version 450

layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec4 a_Normal;
layout(location = 2) in vec4 a_Uv;

layout(location = 0) out vec3 v_Normal;
layout(location = 1) out vec4 v_Position;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform Object {
    mat4 Model;
};

layout(set = 1, binding = 1) uniform Material {
    vec4 Albedo;
};

void main() {
    v_Normal = mat3(Model) * vec3(a_Normal.xyz);
    v_Position = Model * vec4(a_Pos);
    gl_Position = ViewProj * v_Position;
}
