#version 450
layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 0) uniform CustomMaterial {
    vec4 Color;
};

// Naga GLSL does not support the sampler2D type, but only a texture2D + sampler combination
// See https://github.com/gfx-rs/naga/issues/1012
layout(set = 1, binding = 1) uniform texture2D CustomMaterial_texture;
layout(set = 1, binding = 2) uniform sampler CustomMaterial_sampler;


void main() {
    o_Target = Color * texture(sampler2D(CustomMaterial_texture,CustomMaterial_sampler), v_Uv);
}
