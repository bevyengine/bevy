#version 450

layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 2) uniform texture2D SpriteSheet_texture;
layout(set = 1, binding = 3) uniform sampler SpriteSheet_texture_sampler;

void main() {
    o_Target = texture(
        sampler2D(SpriteSheet_texture, SpriteSheet_texture_sampler),
        v_Uv);
}
