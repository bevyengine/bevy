#import bevy_pbr::{
    mesh_view_types::{Lights, DirectionalLight},
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, settings},
        functions::{multiscattering_lut_uv_to_r_mu, sample_transmittance_lut},
        bruneton_functions::{
            distance_to_top_atmosphere_boundary, distance_to_bottom_atmosphere_boundary,
        }
    }
}


const PHI_2: vec2<f32> = vec2(1.3247179572447460259609088, 1.7548776662466927600495087);

@group(0) @binding(12) var multiscattering_lut_out: texture_storage_2d<rgba16float, write>;

fn s2_sequence(n: u32) -> vec2<f32> {
    return fract(0.5 + f32(n) * PHI_2);
}

//Lambert equal-area projection. 
fn map_to_hemisphere(uv: vec2<f32>) -> vec2<f32> {
    //NOTE: must make sure to map to the a hemisphere centered on +-Z,
    //since the integral is symmetric about the x axis
    return vec2(0.0, 0.0); //TODO
}

@compute 
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let uv: vec2<f32> = (vec2<f32>(global_id.xy) + 0.5) / vec2<f32>(settings.multiscattering_lut_size);

    //See Multiscattering LUT paramatrization
    //let r_mu = multiscattering_lut_uv_to_r_mu(uv);

    //single directional light is oriented exactly along the x axis, 
    //with an zenith angle corresponding to mu
    //let direction_to_light = normalize(vec3(1.0, r_mu.y, 0.0));

    /*for (var dir_i: u32= 0u; dir_i < settings.multiscattering_lut_dirs; dir_i++) {
        let phi_theta = map_to_hemisphere(s2_sequence(dir_i));
        let mu = phi_theta.y; // cos(zenith_angle) = dot(vec3::up, dir);

        let atmosphere_dist = min(top_atmosphere_dist, bottom_atmosphere_dist);

        sample_multiscattering_dir(atmosphere, r_mu, atmosphere_dist);
    }*/
}


fn sample_multiscattering_dir(atmosphere: Atmosphere, r: f32, mu: f32, dir: vec2<f32>, atmosphere_dist: f32) {
    for (var step_i: u32 = 0u; step_i < settings.multiscattering_lut_samples; step_i++) {
    }
}

