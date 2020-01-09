#version 450

layout(location = 0) in vec3 v_Normal;
layout(location = 1) in vec4 v_Position;

layout(location = 0) out vec4 o_Target;

void main() {
    o_Target = vec4(1.0, 0.0, 0.0, 1.0);
}
