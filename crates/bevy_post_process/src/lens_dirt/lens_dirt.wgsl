#define_import_path bevy_post_process::lens_dirt

struct LensDirtUniforms {
    intensity: f32,
    tint: vec3<f32>,
};

@group(1) @binding(0) var lens_dirt_texture: texture_2d<f32>;
@group(1) @binding(1) var lens_dirt_sampler: sampler;
@group(1) @binding(2) var<uniform> lens_dirt_uniforms: LensDirtUniforms;
