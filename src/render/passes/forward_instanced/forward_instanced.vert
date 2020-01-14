#version 450

// vertex attributes
layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec4 a_Normal;
layout(location = 2) in vec2 a_Uv;

// Instanced attributes
layout (location = 3) in vec3 a_instancePos;
layout (location = 4) in vec4 a_instanceColor;


layout(location = 0) out vec3 v_Normal;
layout(location = 1) out vec4 v_Position;
layout(location = 2) out vec4 v_Color;

layout(set = 0, binding = 0) uniform Globals {
    mat4 u_ViewProj;
    uvec4 u_NumLights;
};

void main() {
    v_Normal = vec3(a_Normal.xyz);
    v_Position = vec4(a_Pos) + vec4(a_instancePos, 1.0);
    v_Color = a_instanceColor;
    gl_Position = u_ViewProj * v_Position;
}
