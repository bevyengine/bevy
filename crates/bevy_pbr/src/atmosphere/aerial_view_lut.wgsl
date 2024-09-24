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
@group(0) @binding(3) var<uniform> lights: Lights;

@group(0) @binding(4) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(5) var tranmittance_lut_sampler: sampler;

@group(0) @binding(6) var multiscattering_lut: texture_2d<f32>;
@group(0) @binding(7) var multiscattering_lut_sampler: sampler;

@group(0) @binding(8) var aerial_view_lut: texture_storage_3d<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1) //TODO: this approach makes it so closer slices get fewer samples. But we also expect those to have less scattering. So win/win?
fn main(@builtin(global_invocation_id) idx: vec2<u32>) {
    if any(idx > settings.aerial_view_lut_size.xy) { return; }
    let optical_depth: vec3<f32> = 0.0;

    let in_scattering = vec3(0.0);
    let clip_xy = (vec2<f32>(idx) + 0.5) / settings.aerial_view_lut_size.xy;
    let prev_world_z = 0;
    let view_dir = view.world_from_clip * vec4(clip_xy, 1.0, 0.0); //TODO: check this

    let inscattered_illuminance = vec3(0.0);
    for (let slice_i = settings.aerial_view_lut_size.z - 1; z >= 0; slice_i--) { //reversed loop to make coords match reversed Z
        for (let step_i = 0u; step_i < settings.aerial_view_lut_samples; step_i++) {
            let clip_z = (f32(slice_i) + ((f32(step_i) + 0.5) / settings.aerial_view_lut_samples)) / settings.aerial_view_lut_size.z;
            let clip_pos = vec3(clip_xy, clip_z); //TODO: this is likely incorrect. NDC space, not clip; everything below handles clip space correctly though

            let world_pos = (view.world_from_clip * clip_pos).xyz;

            //TODO: see note on `depth_ndc_to_view_z`. Both of these are negative, so to get the (positive) difference we flip the subtraction
            let step_length = prev_world_z - world_pos.z; //TODO: completely wrong. Needs to be ziew space or at least world space depth
            prev_world_z = world_pos.z;

            let r = world_pos.y;
            let local_atmosphere = sample_atmosphere(atmosphere, view_height);

            optical_depth += local_atmosphere.extinction * step_length; //TODO: units between step_length and atmosphere

            let transmittance_to_sample = exp(-optical_depth);

            let local_illuminance = sample_local_inscattering(
                atmosphere, lights, transmittance_lut, transmittance_lut_sampler,
                multiscattering_lut, multiscattering_lut_sampler, r, view_dir
            );
            inscattered_illuminance += local_atmosphere.scattering * local_illuminance * step_length;
            let mean_transmittance = 0.33333333333 * (transmittance_to_sample.r + transmittance_to_sample.g + transmittance_to_sample.b);

            textureStore(aerial_view_lut, vec3(idx.xy, slice_i), vec4(inscattered_illuminance, mean_transmittance));
        }
    }
}
