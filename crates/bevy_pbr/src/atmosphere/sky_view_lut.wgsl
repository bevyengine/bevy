#import bevy_pbr::{
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, settings},
        functions::{
            view_radius, max_atmosphere_distance, direction_atmosphere_to_world,
            sky_view_lut_uv_to_zenith_azimuth, zenith_azimuth_to_ray_dir,
            raymarch_atmosphere, get_view_position, view_radius_constant
        },
    }
}

#import bevy_render::{
    view::View,
    maths::HALF_PI,
}
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(13) var sky_view_lut_out: texture_storage_2d<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
    let uv = vec2<f32>(idx.xy) / vec2<f32>(settings.sky_view_lut_size);

    // let world_pos = vec3<f32>(0.0, 0.0, 0.0); // get_view_position();
    let r = view_radius_constant();
    let world_pos = vec3<f32>(0.0, r, 0.0);
    let up = normalize(world_pos);
    var zenith_azimuth = sky_view_lut_uv_to_zenith_azimuth(r, uv);

    let ray_dir_as = zenith_azimuth_to_ray_dir(zenith_azimuth.x, zenith_azimuth.y);
    let ray_dir_ws = direction_atmosphere_to_world(ray_dir_as);

    let mu = dot(ray_dir_ws, up);
    let t_max = max_atmosphere_distance(r, mu);

    let sample_count = mix(1.0, f32(settings.sky_view_lut_samples), clamp(t_max * 0.01, 0.0, 1.0));
    let result = raymarch_atmosphere(world_pos, ray_dir_ws, t_max, sample_count, uv);

    textureStore(sky_view_lut_out, idx.xy, vec4(result.inscattering, 1.0));
}
