#version 450

// vertex attributes
layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec4 a_Normal;
layout(location = 2) in vec2 a_Uv;

// instanced attributes (RectData)
layout (location = 3) in vec2 a_RectPosition;
layout (location = 4) in vec2 a_RectSize;
layout (location = 5) in vec4 a_RectColor;
layout (location = 6) in float a_RectZIndex;

layout(location = 0) out vec4 v_Color;

layout(set = 0, binding = 0) uniform Camera2d {
    mat4 ViewProj;
};

void main() {
    v_Color = a_RectColor;
    vec4 position = a_Pos * vec4(a_RectSize, 0.0, 1.0);
    position = position + vec4(a_RectPosition + a_RectSize / 2.0, -a_RectZIndex, 0.0);
    gl_Position = ViewProj * position;
}
