#import bevy_solari::scene_bindings map_ray_hit, uniforms
#import bevy_solari::global_illumination::view_bindings view, previous_depth_buffer, depth_buffer, motion_vectors, screen_probes_history, screen_probes, screen_probes_confidence_history, screen_probes_confidence, noise, FIRST_RADIANCE_CASCADE_INTERVAL
#import bevy_solari::world_cache::query query_world_cache
#import bevy_solari::utils trace_ray, depth_to_world_position
#import bevy_pbr::utils octahedral_decode

var<push_constant> cascade: u32;

@compute @workgroup_size(8, 8, 1)
fn update_screen_probes(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let probe_size = 1u << (cascade + 3u);
    let probe_count = (vec2<u32>(view.viewport.zw) + 7u) / probe_size;
    let probe_center_cell_offset = (probe_size / 2u) - 1u;

    var probe_center_pixel_id = ((global_id.xy / probe_size) * probe_size) + probe_center_cell_offset;
    probe_center_pixel_id = min(probe_center_pixel_id, vec2<u32>(view.viewport.zw) - 1u);
    let probe_center_uv = (vec2<f32>(probe_center_pixel_id) + 0.5) / view.viewport.zw;

    // Early out if the probe is placed on a background pixel
    let probe_depth = textureLoad(depth_buffer, probe_center_pixel_id, 0i);
    if probe_depth == 0.0 {
        textureStore(screen_probes, global_id.xy, cascade, vec4(0.0, 0.0, 0.0, 1.0));
        textureStore(screen_probes_confidence, global_id.xy, cascade, vec4(0u));
        return;
    }

    let motion_vector = textureLoad(motion_vectors, probe_center_pixel_id, 0i).rg;
    let reprojected_probe_center_uv = probe_center_uv - motion_vector;

    let reprojected_probe_id_f = reprojected_probe_center_uv * vec2<f32>(probe_count) - 0.5;
    let tl_probe_id = max(vec2<u32>(reprojected_probe_id_f), vec2(0u));
    let tr_probe_id = min(tl_probe_id + vec2(1u, 0u), probe_count - 1u);
    let bl_probe_id = min(tl_probe_id + vec2(0u, 1u), probe_count - 1u);
    let br_probe_id = min(tl_probe_id + vec2(1u, 1u), probe_count - 1u);

    let probe_cell_id = global_id.xy % probe_size;
    let tl_probe_sample = get_probe_cell((tl_probe_id * probe_size) + probe_cell_id);
    let tr_probe_sample = get_probe_cell((tr_probe_id * probe_size) + probe_cell_id);
    let bl_probe_sample = get_probe_cell((bl_probe_id * probe_size) + probe_cell_id);
    let br_probe_sample = get_probe_cell((br_probe_id * probe_size) + probe_cell_id);

    let probe_confidences = vec4(
        get_probe_confidence((tl_probe_id * probe_size) + probe_cell_id),
        get_probe_confidence((tr_probe_id * probe_size) + probe_cell_id),
        get_probe_confidence((bl_probe_id * probe_size) + probe_cell_id),
        get_probe_confidence((br_probe_id * probe_size) + probe_cell_id),
    );

    let current_depth = view.projection[3][2] / probe_depth;
    let probe_depths = vec4(
        get_probe_previous_depth((tl_probe_id * probe_size) + probe_center_cell_offset),
        get_probe_previous_depth((tr_probe_id * probe_size) + probe_center_cell_offset),
        get_probe_previous_depth((bl_probe_id * probe_size) + probe_center_cell_offset),
        get_probe_previous_depth((br_probe_id * probe_size) + probe_center_cell_offset),
    );
    let probe_depth_weights = pow(saturate(1.0 - abs(probe_depths - current_depth) / current_depth), vec4(f32(probe_size)));

    let r = fract(reprojected_probe_id_f);
    let probe_weights = vec4(
        (1.0 - r.x) * (1.0 - r.y),
        r.x * (1.0 - r.y),
        (1.0 - r.x) * r.y,
        r.x * r.y,
    ) * probe_depth_weights;

    var history_color = (tl_probe_sample * probe_weights.x) + (tr_probe_sample * probe_weights.y) + (bl_probe_sample * probe_weights.z) + (br_probe_sample * probe_weights.w);
    history_color /= dot(vec4(1.0), probe_weights);
    history_color = max(history_color, vec4(0.0));

    let screen_disocclusion = f32(all(saturate(reprojected_probe_center_uv) == reprojected_probe_center_uv));
    var history_confidence = dot(probe_confidences, probe_weights) / dot(vec4(1.0), probe_weights);
    history_confidence = max(history_confidence, 0.0);
    history_confidence = 1.0 + (clamp(history_confidence, 0.0, 31.0) * screen_disocclusion);

    // Calculate jittered world-space normal of the assigned probe texel for this thread
    let probe_cell_jitter = textureLoad(noise, global_id.xy % 64u, uniforms.frame_count % 32u, 0i).rg;
    let probe_cell_center = vec2<f32>(global_id.xy % probe_size) + probe_cell_jitter;
    let probe_cell_uv = probe_cell_center / f32(probe_size);
    let probe_cell_normal = octahedral_decode(probe_cell_uv);

    // Calculate radiance interval for this probe based on which cascade it's part of
    var radiance_interval_min = FIRST_RADIANCE_CASCADE_INTERVAL * (exp2(f32(cascade)) - 1.0);
    var radiance_interval_max = FIRST_RADIANCE_CASCADE_INTERVAL * (exp2(f32(cascade) + 1.0) - 1.0);
    if cascade == 0u {
        radiance_interval_min = 0.001;
    }

    // Trace radiance interval from probe position, query world cache for lighting at hit
    var color = vec4(0.0, 0.0, 0.0, 1.0);
    let probe_world_position = depth_to_world_position(probe_depth, probe_center_uv);
    let ray_hit = trace_ray(probe_world_position, probe_cell_normal, radiance_interval_min, radiance_interval_max);
    if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
        let ray_hit = map_ray_hit(ray_hit);
        let hit_color = ray_hit.material.emissive + ray_hit.material.base_color * query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal);
        color = vec4(hit_color, 0.0);
    }

    // Store blended lighting and hit/no-hit in probe texel
    let blended_color = mix(history_color, color, 1.0 / history_confidence);
    let history_confidence_out = vec4(u32(history_confidence), vec3(0u));
    textureStore(screen_probes, global_id.xy, cascade, blended_color);
    textureStore(screen_probes_confidence, global_id.xy, cascade, history_confidence_out);
}

fn get_probe_cell(pixel_id: vec2<u32>) -> vec4<f32> {
    return textureLoad(screen_probes_history, pixel_id, cascade, 0i);
}

fn get_probe_confidence(pixel_id: vec2<u32>) -> f32 {
    return f32(textureLoad(screen_probes_confidence_history, pixel_id, cascade, 0i).r);
}

fn get_probe_previous_depth(pixel_id: vec2<u32>) -> f32 {
    // TODO: Need to use previous view here
    let pixel_id_clamped = min(pixel_id, vec2<u32>(view.viewport.zw) - 1u);
    let depth = textureLoad(previous_depth_buffer, pixel_id_clamped, 0i);
    return view.projection[3][2] / depth;
}
