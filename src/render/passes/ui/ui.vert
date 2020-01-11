#version 450

// vertex attributes
layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec4 a_Normal;

// instanced attributes (RectData)
layout (location = 2) in vec2 a_RectPosition;
layout (location = 3) in vec2 a_RectDimensions;
layout (location = 4) in vec4 a_RectColor;
layout (location = 5) in float a_RectZIndex;

layout(location = 0) out vec4 v_Color;

layout(set = 0, binding = 0) uniform Globals {
    mat4 u_ViewProj;
};

void main() {
    v_Color = a_RectColor;
    vec4 position = a_Pos * vec4(a_RectDimensions, 0.0, 1.0);
    position = position + vec4(a_RectPosition, -a_RectZIndex, 0.0);
    gl_Position = u_ViewProj * position;
}
