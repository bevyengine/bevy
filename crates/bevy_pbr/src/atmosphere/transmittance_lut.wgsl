#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{settings, atmosphere},
    functions::{AtmosphereSample, sample_atmosphere, get_local_r, max_atmosphere_distance},
    bruneton_functions::{transmittance_lut_uv_to_r_mu, distance_to_bottom_atmosphere_boundary, distance_to_top_atmosphere_boundary},
}


#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // map UV coordinates to view height (r) and zenith cos angle (mu)
    let r_mu = transmittance_lut_uv_to_r_mu(in.uv);

    // compute the optical depth from view height r to the top atmosphere boundary
    let optical_depth = compute_optical_depth_to_top_atmosphere_boundary(r_mu.x, r_mu.y, settings.transmittance_lut_samples);

    let transmittance = exp(-optical_depth);

    return vec4<f32>(transmittance, 1.0);
}

/// Compute the optical depth of the atmosphere from the ground to the top atmosphere boundary
/// at a given view height (r) and zenith cos angle (mu)
fn compute_optical_depth_to_top_atmosphere_boundary(r: f32, mu: f32, sample_count: u32) -> vec3<f32> {
    let t_max = max_atmosphere_distance(r, mu); //TODO: max? why not min?

    var optical_depth = vec3<f32>(0.0f);
    var prev_t = 0.0f;

    for (var i = 0u; i < sample_count; i++) {
    // SebH uses this for multiple scattering. It might not be needed here, but I've kept it to get results that are as close as possible to the original
        
    //TODO: check specific integration approach.
        let t_i = (t_max * f32(i) + 0.3f) / f32(sample_count); //TODO: should be 0.5f?
        let dt = t_i - prev_t;
        prev_t = t_i;

    // distance r from current sample point to planet center
        let r_i = get_local_r(r, mu, t_i);

        let atmosphere_sample = sample_atmosphere(r_i);
        let sample_optical_depth = atmosphere_sample.extinction * dt;

        optical_depth += sample_optical_depth;
    }

    return optical_depth;
}
