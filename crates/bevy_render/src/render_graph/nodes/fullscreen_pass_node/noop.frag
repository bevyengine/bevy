#version 450

layout(location=0) in vec2 v_Uv;

layout(set = 0, binding = 0) uniform texture2D color_texture;
layout(set = 0, binding = 1) uniform sampler color_texture_sampler;

layout(location=0) out vec4 o_Target;

void main() {
    o_Target = texture(sampler2D(color_texture, color_texture_sampler), v_Uv);
}
