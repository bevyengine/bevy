#import bevy_pbr::{
    mesh_view_types::{Lights, DirectionalLight},
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, settings, view, lights, aerial_view_lut_out},
        functions::{
            sample_transmittance_lut, sample_atmosphere, rayleigh, henyey_greenstein,
            sample_multiscattering_lut, AtmosphereSample, sample_local_inscattering,
            get_local_r, get_local_up, view_radius, uv_to_ndc, position_ndc_to_world, depth_ndc_to_view_z,
            max_atmosphere_distance
        },
    }
}


@group(0) @binding(13) var aerial_view_lut_out: texture_storage_3d<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1) //TODO: this approach makes it so closer slices get fewer samples. But we also expect those to have less scattering. So win/win?
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
    if any(idx.xy > settings.aerial_view_lut_size.xy) { return; }

    let uv = (vec2<f32>(idx.xy) + 0.5) / vec2<f32>(settings.aerial_view_lut_size.xy);
    let ray_dir = uv_to_ray_direction_ws(uv);
    let r = view_radius();
    let mu = ray_dir.y;
    let t_max = max_atmosphere_distance(r, mu);

    var prev_t = 0.0;
    var total_inscattering = vec3(0.0);
    var optical_depth = vec3(0.0);

    // The aerial view LUT is in NDC space, so it uses bevy's reverse z convention. Since
    // we write multiple slices from each thread, we need to iterate in order near->far, which 
    // is why the indices are reversed.
    for (var slice_i: i32 = i32(settings.aerial_view_lut_size.z - 1); slice_i >= 0; slice_i--) {
        var sum_transmittance = 0.0;
        for (var step_i: i32 = i32(settings.aerial_view_lut_samples - 1); step_i >= 0; step_i--) {
            let sample_depth = depth_at_sample(slice_i, step_i);
            //view_dir.w is the cosine of the angle between the view vector and the camera forward vector, used to correct the step length.            
            let t_i = -depth_ndc_to_view_z(sample_depth) / ray_dir.w * settings.scene_units_to_km;
            let dt = (t_i - prev_t);
            prev_t = t_i;

            let local_r = get_local_r(r, mu, t_i);
            if local_r > atmosphere.top_radius { break; }
            let local_up = get_local_up(r, t_i, ray_dir.xyz);

            let local_atmosphere = sample_atmosphere(local_r);
            optical_depth += local_atmosphere.extinction * dt; 

            // use beer's law to get transmittance from optical density
            let transmittance_to_sample = exp(-optical_depth);

            // evaluate one segment of the integral
            var local_inscattering = sample_local_inscattering(local_atmosphere, transmittance_to_sample, ray_dir.xyz, local_r, local_up);
            total_inscattering += local_inscattering * dt;
            sum_transmittance += transmittance_to_sample.r + transmittance_to_sample.g + transmittance_to_sample.b;
        }
        //We only have one channel to store transmittance, so we store the mean 
        let mean_transmittance = sum_transmittance / (f32(settings.aerial_view_lut_samples) * 3.0);
        textureStore(aerial_view_lut_out, vec3(vec2<i32>(idx.xy), slice_i), vec4(total_inscattering, mean_transmittance));
    }
}

// linearly interpolates from 0..1 on the domain of slice_i, using step_i as a substep index
fn depth_at_sample(slice_i: i32, step_i: i32) -> f32 {
    return (f32(slice_i) + ((f32(step_i) + 0.5) / f32(settings.aerial_view_lut_samples))) / f32(settings.aerial_view_lut_size.z);
}

//Modified from skybox.wgsl. For this pass we don't need to apply a separate sky transform or consider camera viewport.
//w component is the cosine of the view direction with the view forward vector, to correct step distance at the edges of the viewport
fn uv_to_ray_direction_ws(uv: vec2<f32>) -> vec4<f32> {
    // Using world positions of the fragment and camera to calculate a ray direction
    // breaks down at large translations. This code only needs to know the ray direction.
    // The ray direction is along the direction from the camera to the fragment position.
    // In view space, the camera is at the origin, so the view space ray direction is
    // along the direction of the fragment position - (0,0,0) which is just the
    // fragment position.
    // Use the position on the near clipping plane to avoid -inf world position
    // because the far plane of an infinite reverse projection is at infinity.
    let view_position_homogeneous = view.view_from_clip * vec4(
        uv_to_ndc(uv),
        1.0,
        1.0,
    );

    let view_ray_direction = view_position_homogeneous.xyz / view_position_homogeneous.w;
    // Transforming the view space ray direction by the inverse view matrix, transforms the
    // direction to world space. Note that the w element is set to 0.0, as this is a
    // vector direction, not a position, That causes the matrix multiplication to ignore
    // the translations from the view matrix.
    let ray_direction = (view.world_from_view * vec4(view_ray_direction, 0.0)).xyz;

    return vec4(normalize(ray_direction), -view_ray_direction.z);
}
