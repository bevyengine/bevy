#import bevy_solari::global_illumination::view_bindings view, previous_depth_buffer, depth_buffer, motion_vectors, previous_screen_probes, screen_probes_a
#import bevy_solari::utils depth_to_world_position,
#import bevy_pbr::utils octahedral_decode

@compute @workgroup_size(8, 8, 1)
fn reproject_screen_probes(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let probe_size = u32(exp2(f32(global_id.z) + 3.0));
    let probe_count = textureDimensions(screen_probes_a) / probe_size;
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

    var irradiance_interpolated = (tl_probe_sample * tl_probe_weight) + (tr_probe_sample * tr_probe_weight) + (bl_probe_sample * bl_probe_weight) + (br_probe_sample * br_probe_weight);
    irradiance_interpolated /= tl_probe_weight + tr_probe_weight + bl_probe_weight + br_probe_weight;
    irradiance_interpolated = max(irradiance_interpolated, vec4(0.0));

    textureStore(screen_probes_a, global_id.xy, global_id.z, irradiance_interpolated);
}

fn get_probe_cell(pixel_id: vec2<u32>, cascade: u32) -> vec4<f32> {
    return textureLoad(previous_screen_probes, pixel_id, cascade, 0i);
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
