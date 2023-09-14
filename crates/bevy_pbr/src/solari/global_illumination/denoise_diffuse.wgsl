#import bevy_solari::scene_bindings
#import bevy_solari::global_illumination::view_bindings view, depth_buffer, normals_buffer, motion_vectors, previous_depth_buffer, previous_normals_buffer, diffuse_denoiser_temporal_history, diffuse_raw, diffuse_denoised_temporal, diffuse_denoised_spatiotemporal
#import bevy_solari::utils depth_to_world_position

@compute @workgroup_size(8, 8, 1)
fn denoise_diffuse_temporal(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let screen_size = vec2<u32>(view.viewport.zw);
    if any(global_id.xy >= screen_size) { return; }

    let motion_vector = textureLoad(motion_vectors, global_id.xy, 0i).rg;
    let uv = (vec2<f32>(global_id.xy) + 0.5) / view.viewport.zw;
    let history_uv = uv - motion_vector;
    let history_id = vec2<i32>(history_uv * view.viewport.zw);

    let history = textureLoad(diffuse_denoiser_temporal_history, history_id, 0i);
    let irradiance = textureLoad(diffuse_raw, global_id.xy).rgb;

    var history_samples = history.a;

    if any(history_id < vec2(0i)) || any(history_id >= vec2<i32>(screen_size)) {
        history_samples = 0.0;
    }

    let previous_depth = textureLoad(previous_depth_buffer, history_id, 0i);
    let current_depth = textureLoad(depth_buffer, global_id.xy, 0i);
    // TODO: Is it ok to use depth_to_world_position(), which uses the current view, for previous_position?
    let previous_position = depth_to_world_position(previous_depth, history_uv);
    let current_position = depth_to_world_position(current_depth, uv);
    let previous_normal = normalize(textureLoad(previous_normals_buffer, history_id, 0i).xyz * 2.0 - vec3(1.0));
    let current_normal = normalize(textureLoad(normals_buffer, global_id.xy, 0i).xyz * 2.0 - vec3(1.0));

    let plane_distance = abs(dot(previous_position - current_position, current_normal));
    if plane_distance >= 0.5 {
        history_samples = 0.0;
    }

    if dot(current_normal, previous_normal) < 0.95 {
        history_samples = 0.0;
    }

    history_samples = min(history_samples + 1.0, 32.0);

    let blended_irradiance = mix(history.rgb, irradiance, 1.0 / history_samples);

    textureStore(diffuse_denoised_temporal, global_id.xy, vec4(blended_irradiance, history_samples));
}

@compute @workgroup_size(8, 8, 1)
fn denoise_diffuse_spatial(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let screen_size = vec2<u32>(view.viewport.zw);
    if any(global_id.xy >= screen_size) { return; }

    let irradiance = textureLoad(diffuse_denoised_temporal, global_id.xy);
    // TODO

    textureStore(diffuse_denoised_spatiotemporal, global_id.xy, irradiance);
}
