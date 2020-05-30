#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform UiCamera {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform Rect {
    vec2 Rect_Position;
    vec2 Rect_Size;
    float Rect_ZIndex;
};

void main() {
    v_Uv = Vertex_Uv;
    vec3 position = Vertex_Position * vec3(Rect_Size, 0.0);
    position = position + vec3(Rect_Position, -Rect_ZIndex);
    gl_Position = ViewProj * vec4(position, 1.0);
}