#import bevy_solari::scene_bindings map_ray_hit, uniforms
#import bevy_solari::global_illumination::view_bindings view, previous_depth_buffer, depth_buffer, motion_vectors, screen_probes_history, screen_probes, FIRST_RADIANCE_CASCADE_INTERVAL
#import bevy_solari::world_cache::query query_world_cache
#import bevy_solari::utils trace_ray, depth_to_world_position, rand_vec2f
#import bevy_pbr::utils octahedral_decode

@compute @workgroup_size(8, 8, 1)
fn update_screen_probes(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let probe_size = u32(exp2(f32(global_id.z) + 3.0));
    let probe_count = textureDimensions(screen_probes) / probe_size;
    let probe_center_cell_offset = (probe_size / 2u) - 1u;

    var probe_center_pixel_id = ((global_id.xy / probe_size) * probe_size) + probe_center_cell_offset;
    probe_center_pixel_id = min(probe_center_pixel_id, vec2<u32>(view.viewport.zw) - 1u);
    let probe_center_uv = (vec2<f32>(probe_center_pixel_id) + 0.5) / view.viewport.zw;

    let motion_vector = textureLoad(motion_vectors, probe_center_pixel_id, 0i).rg;
    let reprojected_probe_center_uv = probe_center_uv - motion_vector;

    let reprojected_probe_center_pixel_id_f = reprojected_probe_center_uv * vec2<f32>(probe_count) - 0.5;

    let tl_probe_id = max(vec2<u32>(reprojected_probe_center_pixel_id_f), vec2(0u));
    let tr_probe_id = min(tl_probe_id + vec2(1u, 0u), probe_count);
    let bl_probe_id = min(tl_probe_id + vec2(0u, 1u), probe_count);
    let br_probe_id = min(tl_probe_id + vec2(1u, 1u), probe_count);

    let probe_cell_id = global_id.xy % probe_size;
    let tl_probe_sample = get_probe_cell((tl_probe_id * probe_size) + probe_cell_id, global_id.z);
    let tr_probe_sample = get_probe_cell((tr_probe_id * probe_size) + probe_cell_id, global_id.z);
    let bl_probe_sample = get_probe_cell((bl_probe_id * probe_size) + probe_cell_id, global_id.z);
    let br_probe_sample = get_probe_cell((br_probe_id * probe_size) + probe_cell_id, global_id.z);

    let current_depth = get_probe_depth((tl_probe_id * probe_size) + probe_center_cell_offset);
    let tl_probe_depth = get_probe_previous_depth((tl_probe_id * probe_size) + probe_center_cell_offset);
    let tr_probe_depth = get_probe_previous_depth((tr_probe_id * probe_size) + probe_center_cell_offset);
    let bl_probe_depth = get_probe_previous_depth((bl_probe_id * probe_size) + probe_center_cell_offset);
    let br_probe_depth = get_probe_previous_depth((br_probe_id * probe_size) + probe_center_cell_offset);

    let tl_probe_depth_weight = pow(saturate(1.0 - abs(tl_probe_depth - current_depth) / current_depth), f32(probe_size));
    let tr_probe_depth_weight = pow(saturate(1.0 - abs(tr_probe_depth - current_depth) / current_depth), f32(probe_size));
    let bl_probe_depth_weight = pow(saturate(1.0 - abs(bl_probe_depth - current_depth) / current_depth), f32(probe_size));
    let br_probe_depth_weight = pow(saturate(1.0 - abs(br_probe_depth - current_depth) / current_depth), f32(probe_size));

    let r = fract(reprojected_probe_center_pixel_id_f);
    let screen_weight = f32(all(saturate(reprojected_probe_center_uv) == reprojected_probe_center_uv));
    let tl_probe_weight = (1.0 - r.x) * (1.0 - r.y) * tl_probe_depth_weight * screen_weight;
    let tr_probe_weight = r.x * (1.0 - r.y) * tr_probe_depth_weight * screen_weight;
    let bl_probe_weight = (1.0 - r.x) * r.y * bl_probe_depth_weight * screen_weight;
    let br_probe_weight = r.x * r.y * br_probe_depth_weight * screen_weight;

    var history_color = (tl_probe_sample * tl_probe_weight) + (tr_probe_sample * tr_probe_weight) + (bl_probe_sample * bl_probe_weight) + (br_probe_sample * br_probe_weight);
    history_color /= tl_probe_weight + tr_probe_weight + bl_probe_weight + br_probe_weight;
    history_color = max(history_color, vec4(0.0));

    // Reconstruct world position of the probe and early out if the probe is placed on a background pixel
    let probe_depth = textureLoad(depth_buffer, probe_center_pixel_id, 0i);
    if probe_depth == 0.0 {
        textureStore(screen_probes, global_id.xy, global_id.z, vec4(0.0, 0.0, 0.0, 1.0));
        return;
    }
    let probe_world_position = depth_to_world_position(probe_depth, probe_center_uv);

    // Calculate world-space normal of the assigned probe texel for this thread
    var rng = uniforms.frame_count * 5782582u;
    let probe_cell_center = vec2<f32>(global_id.xy % probe_size) + rand_vec2f(&rng);
    let probe_cell_uv = probe_cell_center / f32(probe_size);
    let probe_cell_normal = octahedral_decode(probe_cell_uv);

    // Calculate radiance interval for this probe based on which cascade it's part of
    let i = f32(global_id.z);
    var radiance_interval_min = FIRST_RADIANCE_CASCADE_INTERVAL * (exp2(i) - 1.0);
    var radiance_interval_max = FIRST_RADIANCE_CASCADE_INTERVAL * (exp2(i + 1.0) - 1.0);
    if global_id.z == 0u {
        radiance_interval_min = 0.001;
    }

    // Trace radiance interval, query world cache for lighting at hit
    var color = vec4(0.0, 0.0, 0.0, 1.0);
    let ray_hit = trace_ray(probe_world_position, probe_cell_normal, radiance_interval_min, radiance_interval_max);
    if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
        let ray_hit = map_ray_hit(ray_hit);
        let hit_color = ray_hit.material.emissive + ray_hit.material.base_color * query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal);
        color = vec4(hit_color, 0.0);
    }

    // Store blended lighting and hit/no-hit in probe texel
    let blended_color = mix(history_color, color, 0.1);
    textureStore(screen_probes, global_id.xy, global_id.z, blended_color);
}

fn get_probe_cell(pixel_id: vec2<u32>, cascade: u32) -> vec4<f32> {
    return textureLoad(screen_probes_history, pixel_id, cascade, 0i);
}

fn get_probe_previous_depth(pixel_id: vec2<u32>) -> f32 {
    // TODO: Need to use previous view here
    let pixel_id_clamped = min(pixel_id, vec2<u32>(view.viewport.zw) - 1u);
    let depth = textureLoad(previous_depth_buffer, pixel_id_clamped, 0i);
    return view.projection[3][2] / depth;
}

fn get_probe_depth(pixel_id: vec2<u32>) -> f32 {
    let pixel_id_clamped = min(pixel_id, vec2<u32>(view.viewport.zw) - 1u);
    let depth = textureLoad(depth_buffer, pixel_id_clamped, 0i);
    return view.projection[3][2] / depth;
}
