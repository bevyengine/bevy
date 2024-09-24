#import bevy_pbr::{
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        functions::{distance_to_top_atmosphere_boundary, sample_atmosphere, sample_local_inscattering}
    }
    mesh_view_types::Lights
}
#import bevy_render::view::View;
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var<uniform> settings: AtmosphereSettings;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(3) var<uniform> lights: Lights;
@group(0) @binding(4) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(5) var tranmittance_lut_sampler: sampler;
@group(0) @binding(6) var multiscattering_lut: texture_2d<f32>;
@group(0) @binding(7) var multiscattering_lut_sampler: sampler;

fn magically_get_view_direction(in: FullscreenVertexOutput) -> vec3<f32> { //TODO: HOW
    return vec3(0.0);
}

fn magically_get_view_position() -> vec3<f32> {
    return vec3(0.0);
}

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec3<f32> {
    let view_dir = magically_get_view_direction(in);
    let view_pos = magically_get_view_position();

    let atmosphere_dist = distance_to_top_atmosphere_boundary(atmosphere, r, mu);
    let step_length = atmosphere_dist / f32(settings.sky_view_lut_samples);

    let inscattered_illuminance = vec3(0.0);
    for (let step_i = 0u; step_i < settings.sky_view_lut_samples; step_i++) {
        let pos = view_pos + step_length * view_dir;
        let r = pos.y;

        let local_atmosphere = sample_atmosphere(atmosphere, r);
        let local_illuminance = sample_local_inscattering(
            atmosphere, lights, transmittance_lut, transmittance_lut_sampler,
            multiscattering_lut, multiscattering_lut_sampler, r, view_dir
        );

        inscattered_illuminance += local_atmosphere.scattering * local_illuminance * step_length;
    }

    return inscattered_illuminance;
}
