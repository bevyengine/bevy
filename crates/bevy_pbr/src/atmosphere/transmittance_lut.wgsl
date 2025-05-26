#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{settings, atmosphere},
    functions::{AtmosphereSample, sample_atmosphere, get_local_r, max_atmosphere_distance, MIDPOINT_RATIO},
    bruneton_functions::{transmittance_lut_uv_to_r_mu, distance_to_bottom_atmosphere_boundary, distance_to_top_atmosphere_boundary},
}


#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(13) var transmittance_lut_out: texture_storage_2d<rgba16float, write>;

@compute 
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
    let uv: vec2<f32> = (vec2<f32>(idx.xy) + 0.5) / vec2<f32>(settings.transmittance_lut_size);
    // map UV coordinates to view height (r) and zenith cos angle (mu)
    let r_mu = transmittance_lut_uv_to_r_mu(uv);

    // compute the optical depth from view height r to the top atmosphere boundary
    let optical_depth = ray_optical_depth(r_mu.x, r_mu.y, settings.transmittance_lut_samples);
    let transmittance = exp(-optical_depth);

    textureStore(transmittance_lut_out, idx.xy, vec4(transmittance, 1.0));
}

/// Compute the optical depth of the atmosphere from the ground to the top atmosphere boundary
/// at a given view height (r) and zenith cos angle (mu)
fn ray_optical_depth(r: f32, mu: f32, sample_count: u32) -> vec3<f32> {
    let t_max = max_atmosphere_distance(r, mu);
    var optical_depth = vec3<f32>(0.0f);
    var prev_t = 0.0f;

    for (var i = 0u; i < sample_count; i++) {
        let t_i = t_max * (f32(i) + MIDPOINT_RATIO) / f32(sample_count);
        let dt = t_i - prev_t;
        prev_t = t_i;

        let r_i = get_local_r(r, mu, t_i);

        let atmosphere_sample = sample_atmosphere(r_i);
        let sample_optical_depth = atmosphere_sample.extinction * dt;

        optical_depth += sample_optical_depth;
    }

    return optical_depth;
}
