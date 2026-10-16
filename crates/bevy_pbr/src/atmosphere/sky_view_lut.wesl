import package::{
    atmosphere::{
        bindings::{settings, view},
        functions::{
            get_view_position, raymarch_atmosphere,
            max_atmosphere_distance, direction_atmosphere_to_world,
            sky_view_lut_uv_to_zenith_azimuth, zenith_azimuth_to_ray_dir,
        },
    }
};

import bevy_render::{
    view::View,
    maths::HALF_PI,
};
import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

@group(0) @binding(13) var sky_view_lut_out: texture_storage_2d<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) idx: vec3<u32>, @builtin(workgroup_id) wg: vec3<u32>) {
    // Raymarched mode: the LUT only feeds slowly varying lighting (cloud ambient, ground and probe
    // irradiance, the sky behind thin cirrus), and its jitter is static, so alternate 16-row
    // bands of workgroups are updated on alternate frames (whole workgroups exit, so the work
    // halves). In lookup-texture mode it is the sky itself and is updated every frame.
    if (settings.rendering_method == 1u && (wg.y & 1u) != (view.frame_count & 1u)) {
        return;
    }
    let uv = vec2<f32>(idx.xy) / vec2<f32>(settings.sky_view_lut_size);

    let cam_pos = get_view_position();
    let r = length(cam_pos);
    var zenith_azimuth = sky_view_lut_uv_to_zenith_azimuth(r, uv);

    let ray_dir_as = zenith_azimuth_to_ray_dir(zenith_azimuth.x, zenith_azimuth.y);
    let ray_dir_ws = direction_atmosphere_to_world(ray_dir_as);

    let world_pos = cam_pos;
    let up = normalize(world_pos);
    let mu = dot(ray_dir_ws, up);
    let t_max = max_atmosphere_distance(r, mu);

    let result = raymarch_atmosphere(world_pos, ray_dir_ws, t_max, settings.sky_view_lut_samples, uv, true);

    textureStore(sky_view_lut_out, idx.xy, vec4(result.inscattering, 1.0));
}
