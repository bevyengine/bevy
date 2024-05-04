@group(0) @binding(0) var mip_0: texture_depth_2d;
@group(0) @binding(1) var mip_1: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var mip_2: texture_storage_2d<r32float, write>;
@group(0) @binding(3) var mip_3: texture_storage_2d<r32float, write>;
@group(0) @binding(4) var mip_4: texture_storage_2d<r32float, write>;
@group(0) @binding(5) var mip_5: texture_storage_2d<r32float, write>;
@group(0) @binding(6) var mip_6: texture_storage_2d<r32float, read_write>;
@group(0) @binding(7) var mip_7: texture_storage_2d<r32float, write>;
@group(0) @binding(8) var mip_8: texture_storage_2d<r32float, write>;
@group(0) @binding(9) var mip_9: texture_storage_2d<r32float, write>;
@group(0) @binding(10) var mip_10: texture_storage_2d<r32float, write>;
@group(0) @binding(11) var mip_11: texture_storage_2d<r32float, write>;
@group(0) @binding(12) var mip_12: texture_storage_2d<r32float, write>;
@group(0) @binding(13) var samplr: sampler;
var<push_constant> max_mip_level: u32;

/// Generates a hierarchical depth buffer.
/// Based on FidelityFX SPD v2.1 https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/blob/d7531ae47d8b36a5d4025663e731a47a38be882f/sdk/include/FidelityFX/gpu/spd/ffx_spd.h#L528

var<workgroup> intermediate_memory: array<array<f32, 16>, 16>;

@compute
@workgroup_size(256, 1, 1)
fn downsample_depth_first(
    @builtin(num_workgroups) num_workgroups: vec3u,
    @builtin(workgroup_id) workgroup_id: vec3u,
    @builtin(local_invocation_index) local_invocation_index: u32,
) {
    let sub_xy = remap_for_wave_reduction(local_invocation_index % 64u);
    let x = sub_xy.x + 8u * ((local_invocation_index >> 6u) % 2u);
    let y = sub_xy.y + 8u * (local_invocation_index >> 7u);

    downsample_mips_0_and_1(x, y, workgroup_id.xy, local_invocation_index);

    downsample_mips_2_to_5(x, y, workgroup_id.xy, local_invocation_index);
}

@compute
@workgroup_size(256, 1, 1)
fn downsample_depth_second(@builtin(local_invocation_index) local_invocation_index: u32) {
    let sub_xy = remap_for_wave_reduction(local_invocation_index % 64u);
    let x = sub_xy.x + 8u * ((local_invocation_index >> 6u) % 2u);
    let y = sub_xy.y + 8u * (local_invocation_index >> 7u);

    downsample_mips_6_and_7(x, y);

    downsample_mips_8_to_11(x, y, local_invocation_index);
}

fn downsample_mips_0_and_1(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32) {
    var v: vec4f;

    var tex = vec2(workgroup_id * 64u) + vec2(x * 2u, y * 2u);
    var pix = vec2(workgroup_id * 32u) + vec2(x, y);
    v[0] = reduce_load_mip_0(tex);
    textureStore(mip_1, pix, vec4(v[0]));

    tex = vec2(workgroup_id * 64u) + vec2(x * 2u + 32u, y * 2u);
    pix = vec2(workgroup_id * 32u) + vec2(x + 16u, y);
    v[1] = reduce_load_mip_0(tex);
    textureStore(mip_1, pix, vec4(v[1]));

    tex = vec2(workgroup_id * 64u) + vec2(x * 2u, y * 2u + 32u);
    pix = vec2(workgroup_id * 32u) + vec2(x, y + 16u);
    v[2] = reduce_load_mip_0(tex);
    textureStore(mip_1, pix, vec4(v[2]));

    tex = vec2(workgroup_id * 64u) + vec2(x * 2u + 32u, y * 2u + 32u);
    pix = vec2(workgroup_id * 32u) + vec2(x + 16u, y + 16u);
    v[3] = reduce_load_mip_0(tex);
    textureStore(mip_1, pix, vec4(v[3]));

    if max_mip_level <= 1u { return; }

    for (var i = 0u; i < 4u; i++) {
        intermediate_memory[x][y] = v[i];
        workgroupBarrier();
        if local_invocation_index < 64u {
            v[i] = reduce_4(vec4(
                intermediate_memory[x * 2u + 0u][y * 2u + 0u],
                intermediate_memory[x * 2u + 1u][y * 2u + 0u],
                intermediate_memory[x * 2u + 0u][y * 2u + 1u],
                intermediate_memory[x * 2u + 1u][y * 2u + 1u],
            ));
            pix = (workgroup_id * 16u) + vec2(
                x + (i % 2u) * 8u,
                y + (i / 2u) * 8u,
            );
            textureStore(mip_2, pix, vec4(v[i]));
        }
        workgroupBarrier();
    }

    if local_invocation_index < 64u {
        intermediate_memory[x + 0u][y + 0u] = v[0];
        intermediate_memory[x + 8u][y + 0u] = v[1];
        intermediate_memory[x + 0u][y + 8u] = v[2];
        intermediate_memory[x + 8u][y + 8u] = v[3];
    }
}

fn downsample_mips_2_to_5(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32) {
    if max_mip_level <= 2u { return; }
    workgroupBarrier();
    downsample_mip_2(x, y, workgroup_id, local_invocation_index);

    if max_mip_level <= 3u { return; }
    workgroupBarrier();
    downsample_mip_3(x, y, workgroup_id, local_invocation_index);

    if max_mip_level <= 4u { return; }
    workgroupBarrier();
    downsample_mip_4(x, y, workgroup_id, local_invocation_index);

    if max_mip_level <= 5u { return; }
    workgroupBarrier();
    downsample_mip_5(workgroup_id, local_invocation_index);
}

fn downsample_mip_2(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32) {
    if local_invocation_index < 64u {
        let v = reduce_4(vec4(
            intermediate_memory[x * 2u + 0u][y * 2u + 0u],
            intermediate_memory[x * 2u + 1u][y * 2u + 0u],
            intermediate_memory[x * 2u + 0u][y * 2u + 1u],
            intermediate_memory[x * 2u + 1u][y * 2u + 1u],
        ));
        textureStore(mip_3, (workgroup_id * 8u) + vec2(x, y), vec4(v));
        intermediate_memory[x * 2u + y % 2u][y * 2u] = v;
    }
}

fn downsample_mip_3(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32) {
    if local_invocation_index < 16u {
        let v = reduce_4(vec4(
            intermediate_memory[x * 4u + 0u + 0u][y * 4u + 0u],
            intermediate_memory[x * 4u + 2u + 0u][y * 4u + 0u],
            intermediate_memory[x * 4u + 0u + 1u][y * 4u + 2u],
            intermediate_memory[x * 4u + 2u + 1u][y * 4u + 2u],
        ));
        textureStore(mip_4, (workgroup_id * 4u) + vec2(x, y), vec4(v));
        intermediate_memory[x * 4u + y][y * 4u] = v;
    }
}

fn downsample_mip_4(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32) {
    if local_invocation_index < 4u {
        let v = reduce_4(vec4(
            intermediate_memory[x * 8u + 0u + 0u + y * 2u][y * 8u + 0u],
            intermediate_memory[x * 8u + 4u + 0u + y * 2u][y * 8u + 0u],
            intermediate_memory[x * 8u + 0u + 1u + y * 2u][y * 8u + 4u],
            intermediate_memory[x * 8u + 4u + 1u + y * 2u][y * 8u + 4u],
        ));
        textureStore(mip_5, (workgroup_id * 2u) + vec2(x, y), vec4(v));
        intermediate_memory[x + y * 2u][0u] = v;
    }
}

fn downsample_mip_5(workgroup_id: vec2u, local_invocation_index: u32) {
    if local_invocation_index < 1u {
        let v = reduce_4(vec4(
            intermediate_memory[0u][0u],
            intermediate_memory[1u][0u],
            intermediate_memory[2u][0u],
            intermediate_memory[3u][0u],
        ));
        textureStore(mip_6, workgroup_id, vec4(v));
    }
}

fn downsample_mips_6_and_7(x: u32, y: u32) {
    var v: vec4f;

    var tex = vec2(x * 4u + 0u, y * 4u + 0u);
    var pix = vec2(x * 2u + 0u, y * 2u + 0u);
    v[0] = reduce_load_mip_6(tex);
    textureStore(mip_7, pix, vec4(v[0]));

    tex = vec2(x * 4u + 2u, y * 4u + 0u);
    pix = vec2(x * 2u + 1u, y * 2u + 0u);
    v[1] = reduce_load_mip_6(tex);
    textureStore(mip_7, pix, vec4(v[1]));

    tex = vec2(x * 4u + 0u, y * 4u + 2u);
    pix = vec2(x * 2u + 0u, y * 2u + 1u);
    v[2] = reduce_load_mip_6(tex);
    textureStore(mip_7, pix, vec4(v[2]));

    tex = vec2(x * 4u + 2u, y * 4u + 2u);
    pix = vec2(x * 2u + 1u, y * 2u + 1u);
    v[3] = reduce_load_mip_6(tex);
    textureStore(mip_7, pix, vec4(v[3]));

    if max_mip_level <= 7u { return; }

    let vr = reduce_4(v);
    textureStore(mip_8, vec2(x, y), vec4(vr));
    intermediate_memory[x][y] = vr;
}

fn downsample_mips_8_to_11(x: u32, y: u32, local_invocation_index: u32) {
    if max_mip_level <= 8u { return; }
    workgroupBarrier();
    downsample_mip_8(x, y, local_invocation_index);

    if max_mip_level <= 9u { return; }
    workgroupBarrier();
    downsample_mip_9(x, y, local_invocation_index);

    if max_mip_level <= 10u { return; }
    workgroupBarrier();
    downsample_mip_10(x, y, local_invocation_index);

    if max_mip_level <= 11u { return; }
    workgroupBarrier();
    downsample_mip_11(local_invocation_index);
}

fn downsample_mip_8(x: u32, y: u32, local_invocation_index: u32) {
    if local_invocation_index < 64u {
        let v = reduce_4(vec4(
            intermediate_memory[x * 2u + 0u][y * 2u + 0u],
            intermediate_memory[x * 2u + 1u][y * 2u + 0u],
            intermediate_memory[x * 2u + 0u][y * 2u + 1u],
            intermediate_memory[x * 2u + 1u][y * 2u + 1u],
        ));
        textureStore(mip_9, vec2(x, y), vec4(v));
        intermediate_memory[x * 2u + y % 2u][y * 2u] = v;
    }
}

fn downsample_mip_9(x: u32, y: u32, local_invocation_index: u32) {
    if local_invocation_index < 16u {
        let v = reduce_4(vec4(
            intermediate_memory[x * 4u + 0u + 0u][y * 4u + 0u],
            intermediate_memory[x * 4u + 2u + 0u][y * 4u + 0u],
            intermediate_memory[x * 4u + 0u + 1u][y * 4u + 2u],
            intermediate_memory[x * 4u + 2u + 1u][y * 4u + 2u],
        ));
        textureStore(mip_10, vec2(x, y), vec4(v));
        intermediate_memory[x * 4u + y][y * 4u] = v;
    }
}

fn downsample_mip_10(x: u32, y: u32, local_invocation_index: u32) {
    if local_invocation_index < 4u {
        let v = reduce_4(vec4(
            intermediate_memory[x * 8u + 0u + 0u + y * 2u][y * 8u + 0u],
            intermediate_memory[x * 8u + 4u + 0u + y * 2u][y * 8u + 0u],
            intermediate_memory[x * 8u + 0u + 1u + y * 2u][y * 8u + 4u],
            intermediate_memory[x * 8u + 4u + 1u + y * 2u][y * 8u + 4u],
        ));
        textureStore(mip_11, vec2(x, y), vec4(v));
        intermediate_memory[x + y * 2u][0u] = v;
    }
}

fn downsample_mip_11(local_invocation_index: u32) {
    if local_invocation_index < 1u {
        let v = reduce_4(vec4(
            intermediate_memory[0u][0u],
            intermediate_memory[1u][0u],
            intermediate_memory[2u][0u],
            intermediate_memory[3u][0u],
        ));
        textureStore(mip_12, vec2(0u, 0u), vec4(v));
    }
}

fn remap_for_wave_reduction(a: u32) -> vec2u {
    return vec2(
        insertBits(extractBits(a, 2u, 3u), a, 0u, 1u),
        insertBits(extractBits(a, 3u, 3u), extractBits(a, 1u, 2u), 0u, 2u),
    );
}

fn reduce_load_mip_0(tex: vec2u) -> f32 {
    let uv = (vec2f(tex) + 0.5) / vec2f(textureDimensions(mip_0));
    return reduce_4(textureGather(mip_0, samplr, uv));
}

fn reduce_load_mip_6(tex: vec2u) -> f32 {
    return reduce_4(vec4(
        textureLoad(mip_6, tex + vec2(0u, 0u)).r,
        textureLoad(mip_6, tex + vec2(0u, 1u)).r,
        textureLoad(mip_6, tex + vec2(1u, 0u)).r,
        textureLoad(mip_6, tex + vec2(1u, 1u)).r,
    ));
}

fn reduce_4(v: vec4f) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}
