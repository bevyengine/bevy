#version 450
layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 0) uniform CustomMaterial {
    vec4 Color;
};

layout(set = 1, binding = 1) uniform texture2D CustomMaterial_texture;
layout(set = 1, binding = 2) uniform sampler CustomMaterial_sampler;

// wgsl modules can be imported and used in glsl
// FIXME - this doesn't work any more ...
// #import bevy_pbr::pbr_functions as PbrFuncs

void main() {
    // o_Target = PbrFuncs::tone_mapping(Color * texture(sampler2D(CustomMaterial_texture,CustomMaterial_sampler), v_Uv));
    o_Target = Color * texture(sampler2D(CustomMaterial_texture,CustomMaterial_sampler), v_Uv);
}
