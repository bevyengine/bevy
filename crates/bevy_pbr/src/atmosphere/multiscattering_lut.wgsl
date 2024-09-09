#import bevy_pbr::atmosphere::types::Atmosphere;

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var multiscattering_lut: texture_storage_2d<rgba16float, write>;

fn s2_sequence(n: u32) -> vec2<f32> {
//    const phi_2 = vec2(1.3247179572447460259609088, 1.7548776662466927600495087);
//    fract(0.5 + phi_2 * n);
    return vec2(0.0, 0.0);
}

//Lambert equal-area projection. 
fn map_to_sphere(uv: vec2<f32>) -> vec3<f32> {
    return vec3(0.0, 0.0, 0.0); //TODO
}

const SPHERE_SAMPLES: u32 = 64u;
const STEPS: u32 = 20u;

@compute 
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    //for (let sphere_sample_index: u32 = 0u; sphere_sample_index < SPHERE_SAMPLES; sphere_sample_index++) {
    //    let dir = map_to_sphere(s2_sequence(sphere_sample_index));
    //
    //    for (let step_index: u32 = 0u; step_index < STEPS; step_index++) {
    //    }
    //}
}
