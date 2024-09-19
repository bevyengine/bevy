#import bevy_pbr::{
    mesh_view_types::{Lights, DirectionalLight},
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        functions::{
            sample_transmittance_lut, sample_atmosphere, rayleigh, henyey_greenstein,
            distance_to_top_atmosphere_boundary, distance_to_bottom_atmosphere_boundary,
        },
    }
}

#import bevy_render::view::View;

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var<uniform> settings: AtmosphereSettings;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(2) var<uniform> lights: Lights;

@group(0) @binding(3) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(4) var tranmittance_lut_sampler: sampler;

@group(0) @binding(5) var multiscattering_lut: texture_2d<f32>;
@group(0) @binding(6) var multiscattering_lut_sampler: sampler;

@group(0) @binding(7) var aerial_view_lut: texture_storage_3d<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1) //TODO: this approach makes it so closer slices get fewer samples. But we also expect those to have less scattering. So win/win?
fn main(@builtin(global_invocation_id) idx: vec2<u32>) {
    if any(idx > settings.aerial_view_lut_size.xy) { return; }
    let optical_depth: vec3<f32> = 0.0;

    let in_scattering = vec3(0.0);
    let clip_xy = (vec2<f32>(idx) + 0.5) / settings.aerial_view_lut_size.xy;
    let prev_world_z = 0;
    let direction_to_camera = view.world_from_clip * vec4(clip_xy, 1.0, 0.0); //TODO: check this

    let inscattered_illuminance = vec3(0.0);
    for (let slice_i = settings.aerial_view_lut_size.z - 1; z >= 0; slice_i--) { //reversed loop to make coords match reversed Z
        for (let step_i = 0u; step_i < settings.aerial_view_lut_samples; step_i++) {
            let clip_z = (f32(slice_i) + ((f32(step_i) + 0.5) / settings.aerial_view_lut_samples)) / settings.aerial_view_lut_size.z;
            let clip_pos = vec3(clip_xy, clip_z); //TODO: this is likely incorrect. NDC space, not clip; everything below handles clip space correctly though

            let world_pos = (view.world_from_clip * clip_pos).xyz;

            //see note on `depth_ndc_to_view_z`. Both of these are negative, so to get the (positive) difference we flip the order of subtraction
            let step_length = prev_world_z - world_pos.z;
            prev_world_z = world_pos.z;

            let view_height = world_pos.y;
            let local_atmosphere = sample_atmosphere(atmosphere, view_height);

            optical_depth += local_atmosphere.extinction * step_length; //TODO: units between step_length and atmosphere

            let transmittance_to_sample = exp(-optical_depth);

            let local_illuminance = vec3(0.0);
            for (let i = 0u; i < lights.n_directional_lights; i++) {
                let light = &lights.directional_lights[i];
                let mu_light = (*light).direction_to_light.y; //cosine of azimuth angle to light.

                let neg_LdotV = dot(direction_to_camera, (*light).direction_to_light);
                let rayleigh_phase = rayleigh(neg_LdotV);
                let mie_phase = henyey_greenstein(neg(LdotV));
                let phase = rayleigh_phase + mie_phase; //TODO: check this

                let ground_dist = distance_to_bottom_atmosphere_boundary(atmosphere, r, mu_light);
                let atmosphere_dist = distance_to_top_atmosphere_boundary(atmosphere, r, mu_light);
                let vis = step(atmosphere_dist, ground_dist); //TODO: need to check that the intersection tests return infinity on a miss
                let transmittance_to_light = sample_transmittance_lut(atmosphere, transmittance_lut, transmittance_lut_sampler, r, mu_light);
                let shadow_factor = transmittance_to_light * f32(vis);

                let psi_ms = sample_multiscattering_lut(atmosphere, multiscattering_lut, multiscattering_lut_sampler, r, mu_light)

                local_illuminance += (transmittance_to_sample * shadow_factor * phase + psi_ms) * (*light).color;
            }

            inscattered_illuminance += local_atmosphere.scattering * local_illuminance * step_length;
            let mean_transmittance = 0.33333333333 * (transmittance_to_sample.r + transmittance_to_sample.g + transmittance_to_sample.b);

            textureStore(aerial_view_lut, vec3(idx.xy, slice_i), vec4(inscattered_illuminance, mean_transmittance));
        }
    }
}


#define VIEW_PROJECTION_PERSPECTIVE 1 //TODO: specialize pipeline for this

/// Convert ndc depth to linear view z. 
/// Note: Depth values in front of the camera will be negative as -z is forward
fn depth_ndc_to_view_z(ndc_depth,:, f32) -> f32 {
#ifdef VIEW_PROJECTION_PERSPECTIVE
    return -perspective_camera_near() / ndc_depth;
#else ifdef VIEW_PROJECTION_ORTHOGRAPHIC
    return -(view.clip_from_view[3][2] - ndc_depth) / view.clip_from_view[2][2];
#else
    let view_pos = view.view_from_clip * vec4(0.0, 0.0, ndc_depth, 1.0);
    return view_pos.z / view_pos.w;
#endif
}

