#import bevy_solari::global_illumination::view_bindings view, depth_buffer, normals_buffer, screen_probes_spherical_harmonics, diffuse_irradiance_output
#import bevy_solari::utils depth_to_world_position, get_spherical_harmonics_coefficents

@compute @workgroup_size(8, 8, 1)
fn interpolate_screen_probes(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let screen_size = vec2<u32>(view.viewport.zw);
    if any(global_id.xy >= screen_size) { return; }

    let pixel_depth = view.projection[3][2] / textureLoad(depth_buffer, global_id.xy, 0i);
    if pixel_depth == 0.0 {
        textureStore(diffuse_irradiance_output, global_id.xy, vec4(0.0, 0.0, 0.0, 1.0));
        return;
    }
    let pixel_uv = (vec2<f32>(global_id.xy) + 0.5) / view.viewport.zw;
    let pixel_world_position = depth_to_world_position(pixel_depth, pixel_uv);
    let pixel_world_normal = normalize(textureLoad(normals_buffer, global_id.xy, 0i).xyz * 2.0 - vec3(1.0));

    let probe_count = (vec2<u32>(view.viewport.zw) + 7u) / 8u;
    let probe_id_f = pixel_uv * vec2<f32>(probe_count) - 0.5;

    let tl_probe_id = max(vec2<u32>(probe_id_f), vec2(0u));
    let tr_probe_id = min(tl_probe_id + vec2(1u, 0u), probe_count - 1u);
    let bl_probe_id = min(tl_probe_id + vec2(0u, 1u), probe_count - 1u);
    let br_probe_id = min(tl_probe_id + vec2(1u, 1u), probe_count - 1u);

    let tl_probe_sample = get_probe_irradiance(tl_probe_id, pixel_world_normal, probe_count);
    let tr_probe_sample = get_probe_irradiance(tr_probe_id, pixel_world_normal, probe_count);
    let bl_probe_sample = get_probe_irradiance(bl_probe_id, pixel_world_normal, probe_count);
    let br_probe_sample = get_probe_irradiance(tr_probe_id, pixel_world_normal, probe_count);

    let probe_plane_distance_weights = vec4(
        plane_distance_weight(tl_probe_id, pixel_world_position, pixel_world_normal),
        plane_distance_weight(tr_probe_id, pixel_world_position, pixel_world_normal),
        plane_distance_weight(bl_probe_id, pixel_world_position, pixel_world_normal),
        plane_distance_weight(br_probe_id, pixel_world_position, pixel_world_normal),
    );

    let r = fract(probe_id_f);
    let probe_weights = vec4(
        (1.0 - r.x) * (1.0 - r.y),
        r.x * (1.0 - r.y),
        (1.0 - r.x) * r.y,
        r.x * r.y,
    ) * probe_plane_distance_weights;

    var irradiance = (tl_probe_sample * probe_weights.x) + (tr_probe_sample * probe_weights.y) + (bl_probe_sample * probe_weights.z) + (br_probe_sample * probe_weights.w);
    irradiance /= dot(vec4(1.0), probe_weights);
    irradiance = max(irradiance, vec3(0.0));

    textureStore(diffuse_irradiance_output, global_id.xy, vec4(irradiance, 1.0));
}

fn get_probe_irradiance(probe_id: vec2<u32>, pixel_world_normal: vec3<f32>, probe_count: vec2<u32>) -> vec3<f32> {
    let probe_sh_packed = screen_probes_spherical_harmonics[probe_id.x + probe_id.y * probe_count.x];
    var probe_sh: array<vec3<f32>, 9>;
    probe_sh[0] = probe_sh_packed.a.xyz;
    probe_sh[1] = vec3(probe_sh_packed.a.w, probe_sh_packed.b.xy);
    probe_sh[2] = vec3(probe_sh_packed.b.zw, probe_sh_packed.c.x);
    probe_sh[3] = probe_sh_packed.c.yzw;
    probe_sh[4] = probe_sh_packed.d.xyz;
    probe_sh[5] = vec3(probe_sh_packed.d.w, probe_sh_packed.e.xy);
    probe_sh[6] = vec3(probe_sh_packed.e.zw, probe_sh_packed.f.x);
    probe_sh[7] = probe_sh_packed.f.yzw;
    probe_sh[8] = probe_sh_packed.g.xyz;

    let pixel_sh = get_spherical_harmonics_coefficents(pixel_world_normal);

    var irradiance = vec3(0.0);
    irradiance += pixel_sh[0] * probe_sh[0];
    irradiance += pixel_sh[1] * probe_sh[1];
    irradiance += pixel_sh[2] * probe_sh[2];
    irradiance += pixel_sh[3] * probe_sh[3];
    irradiance += pixel_sh[4] * probe_sh[4];
    irradiance += pixel_sh[5] * probe_sh[5];
    irradiance += pixel_sh[6] * probe_sh[6];
    irradiance += pixel_sh[7] * probe_sh[7];
    irradiance += pixel_sh[8] * probe_sh[8];
    return irradiance;
}

fn plane_distance_weight(probe_id: vec2<u32>, pixel_world_position: vec3<f32>, pixel_world_normal: vec3<f32>) -> f32 {
    var probe_center_pixel_id = (probe_id * 8u) + 3u;
    probe_center_pixel_id = min(probe_center_pixel_id, vec2<u32>(view.viewport.zw) - 1u);

    let probe_depth = textureLoad(depth_buffer, probe_center_pixel_id, 0i);
    let probe_center_uv = (vec2<f32>(probe_center_pixel_id) + 0.5) / view.viewport.zw;
    let probe_world_position = depth_to_world_position(probe_depth, probe_center_uv);

    let plane_distance = abs(dot(probe_world_position - pixel_world_position, pixel_world_normal));

    return step(0.1, plane_distance);
}
