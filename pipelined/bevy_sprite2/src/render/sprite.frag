#version 450

layout(location = 0) in vec2 v_Uv;
layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 0) uniform texture2D sprite_texture;
layout(set = 1, binding = 1) uniform sampler sprite_sampler;

void main() {
    o_Target = texture(sampler2D(sprite_texture, sprite_sampler), v_Uv);
}
