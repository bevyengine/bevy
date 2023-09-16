#import bevy_solari::global_illumination::view_bindings screen_probes_a

var<push_constant> lower_cascade: u32;

@compute @workgroup_size(8, 8, 1)
fn merge_screen_probe_cascades(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let lower_cascade_sample = textureLoad(screen_probes_a, global_id.xy, lower_cascade);
    if lower_cascade_sample.a == 0.0 {
        return;
    }

    let lower_probe_size = u32(exp2(f32(lower_cascade) + 3.0));
    let lower_probe_count = textureDimensions(screen_probes_a) / lower_probe_size;
    let upper_probe_size = lower_probe_size * 2u;
    let upper_probe_count = textureDimensions(screen_probes_a) / upper_probe_size;

    let lower_probe_id = global_id.xy / lower_probe_size;
    let lower_probe_uv = (vec2<f32>(lower_probe_id) + 0.5) / vec2<f32>(lower_probe_count);
    let upper_probe_id_f = lower_probe_uv * vec2<f32>(upper_probe_count) - 0.5;

    let tl_probe_id = vec2<u32>(upper_probe_id_f); // TODO: Max with vec2(0u)
    let tr_probe_id = min(tl_probe_id + vec2(1u, 0u), upper_probe_count);
    let bl_probe_id = min(tl_probe_id + vec2(0u, 1u), upper_probe_count);
    let br_probe_id = min(tl_probe_id + vec2(1u, 1u), upper_probe_count);

    let upper_probe_offset = (global_id.xy % lower_probe_size) * 2u;
    let tl_probe_sample = sample_upper_probe((tl_probe_id * upper_probe_size) + upper_probe_offset);
    let tr_probe_sample = sample_upper_probe((tr_probe_id * upper_probe_size) + upper_probe_offset);
    let bl_probe_sample = sample_upper_probe((bl_probe_id * upper_probe_size) + upper_probe_offset);
    let br_probe_sample = sample_upper_probe((br_probe_id * upper_probe_size) + upper_probe_offset);

    // TODO: Multiply weights by depth weights per probe
    // let depth_weight = pow(saturate(1.0 - abs(probe_depth - center_probe_depth) / center_probe_depth), 8.0);
    let r = fract(upper_probe_id_f);
    let tl_probe_weight = (1.0 - r.x) * (1.0 - r.y);
    let tr_probe_weight = r.x * (1.0 - r.y);
    let bl_probe_weight = (1.0 - r.x) * r.y;
    let br_probe_weight = r.x * r.y;

    var upper_cascade_interpolated = (tl_probe_sample * tl_probe_weight) + (tr_probe_sample * tr_probe_weight) + (bl_probe_sample * bl_probe_weight) + (br_probe_sample * br_probe_weight);
    // upper_cascade_interpolated /= tl_probe_weight + tr_probe_weight + bl_probe_weight + br_probe_weight;

    let merged_sample_rgb = lower_cascade_sample.rgb + (lower_cascade_sample.a * upper_cascade_interpolated.rgb);
    let merged_sample_a = lower_cascade_sample.a * upper_cascade_interpolated.a;
    let merged_sample = vec4(merged_sample_rgb, merged_sample_a);

    textureStore(screen_probes_a, global_id.xy, lower_cascade, merged_sample);
}

fn sample_upper_probe(tl_pixel_id: vec2<u32>) -> vec4<f32> {
    let tl_direction_sample = textureLoad(screen_probes_a, tl_pixel_id, lower_cascade + 1u);
    let tr_direction_sample = textureLoad(screen_probes_a, tl_pixel_id + vec2(1u, 0u), lower_cascade + 1u);
    let bl_direction_sample = textureLoad(screen_probes_a, tl_pixel_id + vec2(0u, 1u), lower_cascade + 1u);
    let br_direction_sample = textureLoad(screen_probes_a, tl_pixel_id + vec2(1u, 1u), lower_cascade + 1u);
    return (tl_direction_sample + tr_direction_sample + bl_direction_sample + br_direction_sample) / 4.0;
}
