// A shader that creates water ripples by overlaying 4 normal maps on top of one
// another.
//
// This is used in the `ssr` example. It only supports deferred rendering.

#import bevy_pbr::{
    pbr_deferred_functions::deferred_output,
    pbr_fragment::pbr_input_from_standard_material,
    prepass_io::{VertexOutput, FragmentOutput},
}
#import bevy_render::globals::Globals

// Parameters to the water shader.
struct WaterSettings {
    // How much to displace each octave each frame, in the u and v directions.
    // Two octaves are packed into each `vec4`.
    octave_vectors: array<vec4<f32>, 2>,
    // How wide the waves are in each octave.
    octave_scales: vec4<f32>,
    // How high the waves are in each octave.
    octave_strengths: vec4<f32>,
}

@group(0) @binding(1) var<uniform> globals: Globals;

@group(2) @binding(100) var water_normals_texture: texture_2d<f32>;
@group(2) @binding(101) var water_normals_sampler: sampler;
@group(2) @binding(102) var<uniform> water_settings: WaterSettings;

// Samples a single octave of noise and returns the resulting normal.
fn sample_noise_octave(uv: vec2<f32>, strength: f32) -> vec3<f32> {
    let N = textureSample(water_normals_texture, water_normals_sampler, uv).rbg * 2.0 - 1.0;
    // This isn't slerp, but it's good enough.
    return normalize(mix(vec3(0.0, 1.0, 0.0), N, strength)); 
}

// Samples all four octaves of noise and returns the resulting normal.
fn sample_noise(uv: vec2<f32>, time: f32) -> vec3<f32> {
    let uv0 = uv * water_settings.octave_scales[0] + water_settings.octave_vectors[0].xy * time;
    let uv1 = uv * water_settings.octave_scales[1] + water_settings.octave_vectors[0].zw * time;
    let uv2 = uv * water_settings.octave_scales[2] + water_settings.octave_vectors[1].xy * time;
    let uv3 = uv * water_settings.octave_scales[3] + water_settings.octave_vectors[1].zw * time;
    return normalize(
        sample_noise_octave(uv0, water_settings.octave_strengths[0]) +
        sample_noise_octave(uv1, water_settings.octave_strengths[1]) +
        sample_noise_octave(uv2, water_settings.octave_strengths[2]) +
        sample_noise_octave(uv3, water_settings.octave_strengths[3])
    );
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    // Create the PBR input.
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    // Bump the normal.
    pbr_input.N = sample_noise(in.uv, globals.time);
    // Send the rest to the deferred shader.
    return deferred_output(in, pbr_input);
}
