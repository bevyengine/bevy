#version 450

layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec4 a_Normal;

layout(location = 0) out vec3 v_Normal;
layout(location = 1) out vec4 v_Position;

layout(set = 0, binding = 0) uniform Globals {
    mat4 u_ViewProj;
};

void main() {
    v_Normal = vec3(a_Normal.xyz);
    v_Position = vec4(a_Pos);
    gl_Position = u_ViewProj * v_Position;
}
