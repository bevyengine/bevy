#import bevy_solari::global_illumination::view_bindings screen_probes, screen_probes_merge_buffer, depth_buffer, view, screen_probes_spherical_harmonics, SphericalHarmonicsPacked
#import bevy_solari::utils get_spherical_harmonics_coefficents
#import bevy_pbr::utils octahedral_decode, PI

var<push_constant> lower_cascade: u32;
var<workgroup> sh_coefficents: array<array<vec3<f32>, 9>, 64>;

@compute @workgroup_size(8, 8, 1)
fn merge_screen_probe_cascades(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(local_invocation_index) local_index: u32,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(num_workgroups) workgroup_count: vec3<u32>,
) {
    let lower_cascade_sample = textureLoad(screen_probes, global_id.xy, lower_cascade);
    if lower_cascade_sample.a == 0.0 {
        return;
    }

    let lower_probe_size = u32(exp2(f32(lower_cascade) + 3.0));
    let lower_probe_count = textureDimensions(screen_probes) / lower_probe_size;
    let upper_probe_size = lower_probe_size * 2u;
    let upper_probe_count = textureDimensions(screen_probes) / upper_probe_size;

    let lower_probe_id = global_id.xy / lower_probe_size;
    let lower_probe_uv = (vec2<f32>(lower_probe_id) + 0.5) / vec2<f32>(lower_probe_count);
    let upper_probe_id_f = lower_probe_uv * vec2<f32>(upper_probe_count) - 0.5;

    let tl_probe_id = max(vec2<u32>(upper_probe_id_f), vec2(0u));
    let tr_probe_id = min(tl_probe_id + vec2(1u, 0u), upper_probe_count);
    let bl_probe_id = min(tl_probe_id + vec2(0u, 1u), upper_probe_count);
    let br_probe_id = min(tl_probe_id + vec2(1u, 1u), upper_probe_count);

    let upper_probe_offset = (global_id.xy % lower_probe_size) * 2u;
    let tl_probe_sample = sample_upper_probe((tl_probe_id * upper_probe_size) + upper_probe_offset);
    let tr_probe_sample = sample_upper_probe((tr_probe_id * upper_probe_size) + upper_probe_offset);
    let bl_probe_sample = sample_upper_probe((bl_probe_id * upper_probe_size) + upper_probe_offset);
    let br_probe_sample = sample_upper_probe((br_probe_id * upper_probe_size) + upper_probe_offset);

    let tl_probe_depth = get_probe_depth((tl_probe_id * upper_probe_size) - (lower_probe_size - 1u));
    let tr_probe_depth = get_probe_depth((tr_probe_id * upper_probe_size) - (lower_probe_size - 1u));
    let bl_probe_depth = get_probe_depth((bl_probe_id * upper_probe_size) - (lower_probe_size - 1u));
    let br_probe_depth = get_probe_depth((br_probe_id * upper_probe_size) - (lower_probe_size - 1u));
    let lower_probe_depth = get_probe_depth(((global_id.xy / lower_probe_size) * lower_probe_size) + (lower_probe_size / 2u - 1u));

    let tl_probe_depth_weight = pow(saturate(1.0 - abs(tl_probe_depth - lower_probe_depth) / lower_probe_depth), f32(lower_probe_size));
    let tr_probe_depth_weight = pow(saturate(1.0 - abs(tr_probe_depth - lower_probe_depth) / lower_probe_depth), f32(lower_probe_size));
    let bl_probe_depth_weight = pow(saturate(1.0 - abs(bl_probe_depth - lower_probe_depth) / lower_probe_depth), f32(lower_probe_size));
    let br_probe_depth_weight = pow(saturate(1.0 - abs(br_probe_depth - lower_probe_depth) / lower_probe_depth), f32(lower_probe_size));

    let r = fract(upper_probe_id_f);
    let tl_probe_weight = (1.0 - r.x) * (1.0 - r.y) * tl_probe_depth_weight;
    let tr_probe_weight = r.x * (1.0 - r.y) * tr_probe_depth_weight;
    let bl_probe_weight = (1.0 - r.x) * r.y * bl_probe_depth_weight;
    let br_probe_weight = r.x * r.y * br_probe_depth_weight;

    var upper_cascade_interpolated = (tl_probe_sample * tl_probe_weight) + (tr_probe_sample * tr_probe_weight) + (bl_probe_sample * bl_probe_weight) + (br_probe_sample * br_probe_weight);
    upper_cascade_interpolated /= tl_probe_weight + tr_probe_weight + bl_probe_weight + br_probe_weight;
    upper_cascade_interpolated = max(upper_cascade_interpolated, vec4(0.0));

    let merged_sample_rgb = lower_cascade_sample.rgb + (lower_cascade_sample.a * upper_cascade_interpolated.rgb);
    let merged_sample_a = lower_cascade_sample.a * upper_cascade_interpolated.a;
    let merged_sample = vec4(merged_sample_rgb, merged_sample_a);

    if lower_cascade == 2u {
        textureStore(screen_probes_merge_buffer, global_id.xy, 0i, merged_sample);
    } else if lower_cascade == 1u {
        textureStore(screen_probes_merge_buffer, global_id.xy, 1i, merged_sample);
    } else {
        convert_to_spherical_harmonics(merged_sample.rgb, local_id.xy, local_index, workgroup_id.x + workgroup_id.y * workgroup_count.x);
    }
}

fn sample_upper_probe(tl_cell_id: vec2<u32>) -> vec4<f32> {
    let tl_direction_sample = sample_upper_probe_texture(tl_cell_id);
    let tr_direction_sample = sample_upper_probe_texture(tl_cell_id + vec2(1u, 0u));
    let bl_direction_sample = sample_upper_probe_texture(tl_cell_id + vec2(0u, 1u));
    let br_direction_sample = sample_upper_probe_texture(tl_cell_id + vec2(1u, 1u));
    return (tl_direction_sample + tr_direction_sample + bl_direction_sample + br_direction_sample) / 4.0;
}

fn sample_upper_probe_texture(cell_id: vec2<u32>) -> vec4<f32> {
    if lower_cascade == 2u {
        return textureLoad(screen_probes, cell_id, 3i);
    }
    if lower_cascade == 1u {
        return textureLoad(screen_probes_merge_buffer, cell_id, 0i);
    }
    return textureLoad(screen_probes_merge_buffer, cell_id, 1i);
}

fn get_probe_depth(pixel_id: vec2<u32>) -> f32 {
    let pixel_id_clamped = min(pixel_id, vec2<u32>(view.viewport.zw) - 1u);
    let depth = textureLoad(depth_buffer, pixel_id_clamped, 0i);
    return view.projection[3][2] / depth;
}

// TODO: Replace with subgroup/wave ops when supported
fn convert_to_spherical_harmonics(irradiance: vec3<f32>, cell_id: vec2<u32>, cell_index: u32, probe_index: u32) {
    let octahedral_pixel_center = vec2<f32>(cell_id) + 0.5;
    let octahedral_normal = octahedral_decode(octahedral_pixel_center / 8.0);

    let local_sh = get_spherical_harmonics_coefficents(octahedral_normal);
    sh_coefficents[cell_index][0] = local_sh[0] * irradiance;
    sh_coefficents[cell_index][1] = local_sh[1] * irradiance;
    sh_coefficents[cell_index][2] = local_sh[2] * irradiance;
    sh_coefficents[cell_index][3] = local_sh[3] * irradiance;
    sh_coefficents[cell_index][4] = local_sh[4] * irradiance;
    sh_coefficents[cell_index][5] = local_sh[5] * irradiance;
    sh_coefficents[cell_index][6] = local_sh[6] * irradiance;
    sh_coefficents[cell_index][7] = local_sh[7] * irradiance;
    sh_coefficents[cell_index][8] = local_sh[8] * irradiance;
    workgroupBarrier();
    for (var t = 32u; t > 0u; t >>= 1u) {
        if cell_index < t {
            for (var i = 0u; i < 9u; i++) {
                sh_coefficents[cell_index][i] += sh_coefficents[cell_index + t][i];
            }
        }
        workgroupBarrier();
    }

    if cell_index == 0u {
        var sh: array<vec3<f32>, 9>;
        for (var i = 0u; i < 9u; i++) {
            sh[i] = sh_coefficents[0][i] * (4.0 * PI) / 64.0;
        }

        screen_probes_spherical_harmonics[probe_index] = SphericalHarmonicsPacked(
            vec4(sh[0], sh[1].x),
            vec4(sh[1].yz, sh[2].xy),
            vec4(sh[2].z, sh[3]),
            vec4(sh[4], sh[5].x),
            vec4(sh[5].yz, sh[6].xy),
            vec4(sh[6].z, sh[7]),
            vec4(sh[8], 0.0),
        );
    }
}
