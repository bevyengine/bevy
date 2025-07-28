enable dual_source_blending;

#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{atmosphere, view, atmosphere_transforms},
    functions::{
        sample_transmittance_lut, sample_transmittance_lut_segment,
        sample_sky_view_lut, direction_world_to_atmosphere,
        uv_to_ray_direction, uv_to_ndc, sample_aerial_view_lut,
        view_radius, sample_sun_radiance, ndc_to_camera_dist
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
#ifdef DUAL_SOURCE_BLENDING
    @location(0) @blend_src(0) inscattering: vec4<f32>,
    @location(0) @blend_src(1) transmittance: vec4<f32>,
#else
    @location(0) inscattering: vec4<f32>,
#endif
}

@fragment
fn main(in: FullscreenVertexOutput) -> RenderSkyOutput {
    let depth = textureLoad(depth_texture, vec2<i32>(in.position.xy), 0);

    let ray_dir_ws = uv_to_ray_direction(in.uv);
    let r = view_radius();
    let mu = ray_dir_ws.y;

    var transmittance: vec3<f32>;
    var inscattering: vec3<f32>;

    let sun_radiance = sample_sun_radiance(ray_dir_ws.xyz);

    if depth == 0.0 {
        let ray_dir_as = direction_world_to_atmosphere(ray_dir_ws.xyz);
        transmittance = sample_transmittance_lut(r, mu);
        inscattering += sample_sky_view_lut(r, ray_dir_as);
        inscattering += sun_radiance * transmittance * view.exposure;
    } else {
        let t = ndc_to_camera_dist(vec3(uv_to_ndc(in.uv), depth));
        inscattering = sample_aerial_view_lut(in.uv, t);
        transmittance = sample_transmittance_lut_segment(r, mu, t);
    }
#ifdef DUAL_SOURCE_BLENDING
    return RenderSkyOutput(vec4(inscattering, 0.0), vec4(transmittance, 1.0));
#else
    let mean_transmittance = (transmittance.r + transmittance.g + transmittance.b) / 3.0;
    return RenderSkyOutput(vec4(inscattering, mean_transmittance));
#endif
    
}
