import package::atmosphere::{
    bindings::{settings, atmosphere, medium_density_lut, medium_sampler},
    functions::{
        sample_density_lut, get_local_r, max_atmosphere_distance,
        MIDPOINT_RATIO, ABSORPTION_DENSITY, SCATTERING_DENSITY
    },
    bruneton_functions::transmittance_lut_uv_to_r_mu,
};

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

@group(0) @binding(13) var transmittance_lut_out: texture_storage_2d<rgba16float, write>;

@compute 
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
    let uv: vec2<f32> = (vec2<f32>(idx.xy) + 0.5) / vec2<f32>(settings.transmittance_lut_size);
    // map UV coordinates to view height (r) and zenith cos angle (mu)
    let r_mu = transmittance_lut_uv_to_r_mu(uv);

    // RGB optical depth → ballistic T; alpha = reducible Mie/haze OD (∫ g·σ_s dt).
    let optical_depth = ray_optical_depth(r_mu.x, r_mu.y, settings.transmittance_lut_samples);
    let transmittance = exp(-optical_depth.rgb);

    textureStore(transmittance_lut_out, idx.xy, vec4(transmittance, optical_depth.a));
}

/// Optical depth from (r, mu) to the top atmosphere boundary.
/// .rgb = absorption + scattering (all terms); .a = Σ g·σ_s for Mie/haze only.
fn ray_optical_depth(r: f32, mu: f32, sample_count: u32) -> vec4<f32> {
    let t_max = max_atmosphere_distance(r, mu);
    var optical_depth = vec4<f32>(0.0f);
    // Quadratically spaced segments, sampled at their midpoints: the dense near-surface layers
    // dominate grazing paths, and uniform steps (first sample ~0.3 * t_max / n away) skip them,
    // overestimating low-sun transmittance by up to 2x (blue, 3 deg elevation).
    for (var i = 0u; i < sample_count; i++) {
        let u0 = f32(i) / f32(sample_count);
        let u1 = f32(i + 1u) / f32(sample_count);
        let t0 = t_max * u0 * u0;
        let t1 = t_max * u1 * u1;
        let t_i = 0.5 * (t0 + t1);
        let dt = t1 - t0;

        let r_i = get_local_r(r, mu, t_i);

        let normalized_altitude = (r_i - atmosphere.inner_radius) / (atmosphere.outer_radius - atmosphere.inner_radius);
        let uv_abs = vec2(1.0 - normalized_altitude, ABSORPTION_DENSITY);
        let uv_sca = vec2(1.0 - normalized_altitude, SCATTERING_DENSITY);
        let absorption = textureSampleLevel(medium_density_lut, medium_sampler, uv_abs, 0.0).xyz;
        let scattering_s = textureSampleLevel(medium_density_lut, medium_sampler, uv_sca, 0.0);
        let extinction = absorption + scattering_s.xyz;
        optical_depth += vec4(extinction * dt, scattering_s.a * dt);
    }

    return optical_depth;
}
