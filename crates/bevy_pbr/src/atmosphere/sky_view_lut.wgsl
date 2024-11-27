#import bevy_pbr::{
    mesh_view_types::Lights,
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, view, settings},
        functions::{
            sample_atmosphere, get_local_up, AtmosphereSample,
            sample_local_inscattering, get_local_r, view_radius,
            sky_view_lut_unsquash_ray_dir, direction_view_to_world,
            max_atmosphere_distance, direction_atmosphere_to_world,
        },
    }
}

#import bevy_render::view::View;
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(13) var sky_view_lut_out: texture_storage_2d_array<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
    let uv = (vec2<f32>(idx.xy) + vec2(0.5)) / f32(settings.sky_view_lut_size);

    let r = view_radius(); //TODO: paper says to center the sky view on the planet ground

    let ray_dir_as_squashed = cubemap_coords_to_ray_dir(uv, idx.z);
    let ray_dir_as = correct_sampling_dir(r, sky_view_lut_unsquash_ray_dir(ray_dir_as_squashed));
    let ray_dir = direction_view_to_world(ray_dir_as);

    let mu = ray_dir.y;

    let t_max = max_atmosphere_distance(r, mu);
    let dt = t_max / f32(settings.sky_view_lut_samples);

    var total_inscattering = vec3(0.0);
    var optical_depth = vec3(0.0);
    for (var step_i: u32 = 0u; step_i < settings.sky_view_lut_samples; step_i++) {
        let t_i = dt * (f32(step_i) + 0.5); //todo: 0.3???;
        let local_r = get_local_r(r, mu, t_i);
        let local_up = get_local_up(r, t_i, ray_dir);

        let local_atmosphere = sample_atmosphere(local_r);
        optical_depth += local_atmosphere.extinction * dt;
        let transmittance_to_sample = exp(-optical_depth);

        var local_inscattering = sample_local_inscattering(local_atmosphere, transmittance_to_sample, ray_dir, local_r, local_up);
        total_inscattering += local_inscattering * dt;
    }

    textureStore(sky_view_lut_out, idx.xy, idx.z, vec4(total_inscattering, 1.0));
}

//approximates sampling direction from angle to horizon at the current radius
fn correct_sampling_dir(r: f32, ray_dir_as: vec3<f32>) -> vec3<f32> {
    let altitude_ratio = atmosphere.bottom_radius / r;
    let neg_mu_horizon = sqrt(1 - altitude_ratio * altitude_ratio);
    return normalize(ray_dir_as - vec3(0.0, neg_mu_horizon, 0.0));
}

fn cubemap_coords_to_ray_dir(uv: vec2<f32>, face_index: u32) -> vec3<f32> {
    let quotient: u32 = face_index / 2u;
    let remainder: u32 = face_index % 2u;
    let sign: f32 = 1.0 - 2.0 * f32(remainder);
    var ray_dir = vec3(0.0);
    let uv1_1 = uv * 2 - 1;
    switch quotient {
        case 0u: { // x axis 
            ray_dir = vec3(sign, -uv1_1.y, -sign * uv1_1.x);
        }
        case 1u: { // y axis
            ray_dir = vec3(uv1_1.x, sign, sign * uv1_1.y);
        }
        case 2u: { // z axis
            ray_dir = vec3(sign * uv1_1.x, -uv1_1.y, sign);
        }
        default: {
            ray_dir = vec3(0.0, 1.0, 0.0);
        }
    }
    return normalize(ray_dir);
}
