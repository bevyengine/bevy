#define_import_path bevy_core_pipeline::tonemapping_lut_bindings

@group(0) @binding(#TONEMAPPING_LUT_TEXTURE_BINDING_INDEX) var dt_lut_texture: texture_3d<f32>;
@group(0) @binding(#TONEMAPPING_LUT_SAMPLER_BINDING_INDEX) var dt_lut_sampler: sampler;

