#import bevy_pbr::{
    atmosphere::{
        bindings::{
            atmosphere, settings, view, lights,
            transmittance_lut, sky_view_lut,
        },
        functions::{
            direction_world_to_atmosphere,
            sample_sky_view_lut,
            sample_transmittance_lut,
            get_view_position,
            max_atmosphere_distance,
            raymarch_atmosphere,
        },
    },
    utils::sample_cube_dir
}

@group(0) @binding(13) var output: texture_storage_2d_array<rgba16float, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(output);
    let slice_index = global_id.z;
    
    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y || slice_index >= 6u) {
        return;
    }
    
    // Calculate normalized UV coordinates for this pixel
    let uv = vec2<f32>(
        (f32(global_id.x) + 0.5) / f32(dimensions.x),
        (f32(global_id.y) + 0.5) / f32(dimensions.y)
    );

    var ray_dir_ws = sample_cube_dir(uv, slice_index);
    
    // invert the z direction to account for cubemaps being lefthanded
    ray_dir_ws.z = -ray_dir_ws.z;

    let world_pos = get_view_position();
    let r = length(world_pos);
    let up = normalize(world_pos);
    let mu = dot(ray_dir_ws, up);

    let ray_dir_as = direction_world_to_atmosphere(ray_dir_ws.xyz, up);
    var transmittance = sample_transmittance_lut(r, mu);
    var inscattering = sample_sky_view_lut(r, ray_dir_as);

    // Match `render_sky.wgsl` behavior: if raymarch mode is enabled, integrate numerically.
    // With CLOUDS_ENABLED this also includes volumetric clouds on the view ray.
    if (settings.rendering_method == 1u) {
        let t_max = max_atmosphere_distance(r, mu);
        let max_samples = settings.sky_max_samples;
        let result = raymarch_atmosphere(world_pos, ray_dir_ws, t_max, max_samples, uv, true);
        inscattering = result.inscattering;
        transmittance = result.transmittance;
    }

    // NOTE: We intentionally do NOT add the analytic sun-disk term here.
    // `sample_sun_radiance()` uses `fwidth()` for anti-aliasing, which is forbidden in compute stages.

    let color = vec4<f32>(inscattering, 1.0);

    textureStore(output, vec2<i32>(global_id.xy), i32(slice_index), color);
}
