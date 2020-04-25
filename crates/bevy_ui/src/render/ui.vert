#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout (location = 3) in vec2 I_Rect_Position;
layout (location = 4) in vec2 I_Rect_Size;
layout (location = 5) in vec4 I_Rect_Color;
layout (location = 6) in float I_Rect_ZIndex;

layout(location = 0) out vec4 v_Color;

layout(set = 0, binding = 0) uniform Camera2d {
    mat4 ViewProj;
};

void main() {
    v_Color = I_Rect_Color;
    vec3 position = Vertex_Position * vec3(I_Rect_Size, 0.0);
    position = position + vec4(I_Rect_Position + I_Rect_Size / 2.0, -I_Rect_ZIndex, 0.0);
    gl_Position = ViewProj * vec4(position, 1.0);
}
