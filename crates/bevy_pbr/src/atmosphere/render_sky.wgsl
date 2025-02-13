#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{atmosphere, view, settings, atmosphere_transforms},
    functions::{
        sample_transmittance_lut, sample_transmittance_lut_segment,
        sample_sky_view_lut, direction_world_to_atmosphere,
        uv_to_ray_direction, uv_to_ndc, sample_aerial_view_lut,
        view_radius, sample_sun_illuminance, ndc_to_camera_dist,
        raymarch_atmosphere, max_atmosphere_distance
    },
};
#import bevy_render::view::View;

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

#ifdef MULTISAMPLED
@group(0) @binding(13) var depth_texture: texture_depth_multisampled_2d;
#else
@group(0) @binding(13) var depth_texture: texture_depth_2d;
#endif

struct RenderSkyOutput {
    @location(0) inscattering: vec4<f32>,
    @location(0) @second_blend_source transmittance: vec4<f32>,
}

@fragment
fn main(in: FullscreenVertexOutput) -> RenderSkyOutput {
    let depth = textureLoad(depth_texture, vec2<i32>(in.position.xy), 0);

    let ray_dir_ws = uv_to_ray_direction(in.uv).xyz;
    let r = view_radius();
    let mu = ray_dir_ws.y;
    let raymarch_split = 1.0;
    let raymarch_steps = 32.0;

    var transmittance: vec3<f32>;
    var inscattering: vec3<f32>;
    if depth == 0.0 {
        let ray_dir_as = direction_world_to_atmosphere(ray_dir_ws);
        transmittance = sample_transmittance_lut(r, mu);
        inscattering += sample_sky_view_lut(r, ray_dir_as);
        if (in.uv.x > raymarch_split) {
            let t_max = max_atmosphere_distance(r, mu);
            let result = raymarch_atmosphere(r, ray_dir_ws, t_max, raymarch_steps);
            inscattering = result.inscattering;
            transmittance = result.transmittance;
        }

        inscattering += sample_sun_illuminance(ray_dir_ws, transmittance);
    } else {
        let t = ndc_to_camera_dist(vec3(uv_to_ndc(in.uv), depth));
        inscattering = sample_aerial_view_lut(in.uv, t);
        transmittance = sample_transmittance_lut_segment(r, mu, t);

        if (in.uv.x > raymarch_split) {
            let result = raymarch_atmosphere(r, ray_dir_ws, t, raymarch_steps);
            inscattering = result.inscattering;
            transmittance = result.transmittance;
        }
    }
    return RenderSkyOutput(vec4(inscattering, 0.0), vec4(transmittance, 1.0));
}
