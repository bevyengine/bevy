// Single pass downsampling shader for creating the mip chain for an array texture
// Ported from https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/blob/c16b1d286b5b438b75da159ab51ff426bacea3d1/sdk/include/FidelityFX/gpu/spd/ffx_spd.h

@group(0) @binding(0) var sampler_linear_clamp: sampler;
@group(0) @binding(1) var<uniform> constants: Constants;
#ifdef COMBINE_BIND_GROUP
@group(0) @binding(2) var mip_0: texture_2d_array<f32>;
@group(0) @binding(3) var mip_1: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(4) var mip_2: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(5) var mip_3: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(6) var mip_4: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(7) var mip_5: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(8) var mip_6: texture_storage_2d_array<rgba16float, read_write>;
@group(0) @binding(9) var mip_7: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(10) var mip_8: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(11) var mip_9: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(12) var mip_10: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(13) var mip_11: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(14) var mip_12: texture_storage_2d_array<rgba16float, write>;
#endif

#ifdef FIRST_PASS
@group(0) @binding(2) var mip_0: texture_2d_array<f32>;
@group(0) @binding(3) var mip_1: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(4) var mip_2: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(5) var mip_3: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(6) var mip_4: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(7) var mip_5: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(8) var mip_6: texture_storage_2d_array<rgba16float, write>;
#endif

#ifdef SECOND_PASS
@group(0) @binding(2) var mip_6: texture_2d_array<f32>;
@group(0) @binding(3) var mip_7: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(4) var mip_8: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(5) var mip_9: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(6) var mip_10: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(7) var mip_11: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(8) var mip_12: texture_storage_2d_array<rgba16float, write>;
#endif

struct Constants { mips: u32, inverse_input_size: vec2f }

var<workgroup> spd_intermediate_r: array<array<f32, 16>, 16>;
var<workgroup> spd_intermediate_g: array<array<f32, 16>, 16>;
var<workgroup> spd_intermediate_b: array<array<f32, 16>, 16>;
var<workgroup> spd_intermediate_a: array<array<f32, 16>, 16>;

@compute
@workgroup_size(256, 1, 1)
fn downsample_first(
    @builtin(workgroup_id) workgroup_id: vec3u,
    @builtin(local_invocation_index) local_invocation_index: u32
) {

    let sub_xy = remap_for_wave_reduction(local_invocation_index % 64u);
    let x = sub_xy.x + 8u * ((local_invocation_index >> 6u) % 2u);
    let y = sub_xy.y + 8u * (local_invocation_index >> 7u);

    spd_downsample_mips_0_1(x, y, workgroup_id.xy, local_invocation_index, constants.mips, workgroup_id.z);

    spd_downsample_next_four(x, y, workgroup_id.xy, local_invocation_index, 2u, constants.mips, workgroup_id.z);
}

// TODO: Once wgpu supports globallycoherent buffers, make it actually a single pass
@compute
@workgroup_size(256, 1, 1)
fn downsample_second(
    @builtin(workgroup_id) workgroup_id: vec3u,
    @builtin(local_invocation_index) local_invocation_index: u32,
) {
    let sub_xy = remap_for_wave_reduction(local_invocation_index % 64u);
    let x = sub_xy.x + 8u * ((local_invocation_index >> 6u) % 2u);
    let y = sub_xy.y + 8u * (local_invocation_index >> 7u);

    spd_downsample_mips_6_7(x, y, constants.mips, workgroup_id.z);

    spd_downsample_next_four(x, y, vec2(0u), local_invocation_index, 8u, constants.mips, workgroup_id.z);
}

fn spd_downsample_mips_0_1(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32, mips: u32, slice: u32) {
    var v: array<vec4f, 4>;

    var tex = (workgroup_id * 64u) + vec2(x * 2u, y * 2u);
    var pix = (workgroup_id * 32u) + vec2(x, y);
    v[0] = spd_reduce_load_source_image(tex, slice);
    spd_store(pix, v[0], 0u, slice);

    tex = (workgroup_id * 64u) + vec2(x * 2u + 32u, y * 2u);
    pix = (workgroup_id * 32u) + vec2(x + 16u, y);
    v[1] = spd_reduce_load_source_image(tex, slice);
    spd_store(pix, v[1], 0u, slice);

    tex = (workgroup_id * 64u) + vec2(x * 2u, y * 2u + 32u);
    pix = (workgroup_id * 32u) + vec2(x, y + 16u);
    v[2] = spd_reduce_load_source_image(tex, slice);
    spd_store(pix, v[2], 0u, slice);

    tex = (workgroup_id * 64u) + vec2(x * 2u + 32u, y * 2u + 32u);
    pix = (workgroup_id * 32u) + vec2(x + 16u, y + 16u);
    v[3] = spd_reduce_load_source_image(tex, slice);
    spd_store(pix, v[3], 0u, slice);

    if mips <= 1u { return; }

#ifdef SUBGROUP_SUPPORT
    v[0] = spd_reduce_quad(v[0]);
    v[1] = spd_reduce_quad(v[1]);
    v[2] = spd_reduce_quad(v[2]);
    v[3] = spd_reduce_quad(v[3]);

    if local_invocation_index % 4u == 0u {
        spd_store((workgroup_id * 16u) + vec2(x / 2u, y / 2u), v[0], 1u, slice);
        spd_store_intermediate(x / 2u, y / 2u, v[0]);

        spd_store((workgroup_id * 16u) + vec2(x / 2u + 8u, y / 2u), v[1], 1u, slice);
        spd_store_intermediate(x / 2u + 8u, y / 2u, v[1]);

        spd_store((workgroup_id * 16u) + vec2(x / 2u, y / 2u + 8u), v[2], 1u, slice);
        spd_store_intermediate(x / 2u, y / 2u + 8u, v[2]);

        spd_store((workgroup_id * 16u) + vec2(x / 2u + 8u, y / 2u + 8u), v[3], 1u, slice);
        spd_store_intermediate(x / 2u + 8u, y / 2u + 8u, v[3]);
    }
#else
    for (var i = 0u; i < 4u; i++) {
        spd_store_intermediate(x, y, v[i]);
        workgroupBarrier();
        if local_invocation_index < 64u {
            v[i] = spd_reduce_intermediate(
                vec2(x * 2u + 0u, y * 2u + 0u),
                vec2(x * 2u + 1u, y * 2u + 0u),
                vec2(x * 2u + 0u, y * 2u + 1u),
                vec2(x * 2u + 1u, y * 2u + 1u),
            );
            spd_store(vec2(workgroup_id * 16) + vec2(x + (i % 2u) * 8u, y + (i / 2u) * 8u), v[i], 1u, slice);
        }
        workgroupBarrier();
    }

    if local_invocation_index < 64u {
        spd_store_intermediate(x + 0u, y + 0u, v[0]);
        spd_store_intermediate(x + 8u, y + 0u, v[1]);
        spd_store_intermediate(x + 0u, y + 8u, v[2]);
        spd_store_intermediate(x + 8u, y + 8u, v[3]);
    }
#endif
}

fn spd_downsample_next_four(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32, base_mip: u32, mips: u32, slice: u32) {
    if mips <= base_mip { return; }
    workgroupBarrier();
    spd_downsample_mip_2(x, y, workgroup_id, local_invocation_index, base_mip, slice);

    if mips <= base_mip + 1u { return; }
    workgroupBarrier();
    spd_downsample_mip_3(x, y, workgroup_id, local_invocation_index, base_mip + 1u, slice);

    if mips <= base_mip + 2u { return; }
    workgroupBarrier();
    spd_downsample_mip_4(x, y, workgroup_id, local_invocation_index, base_mip + 2u, slice);

    if mips <= base_mip + 3u { return; }
    workgroupBarrier();
    spd_downsample_mip_5(x, y, workgroup_id, local_invocation_index, base_mip + 3u, slice);
}

fn spd_downsample_mip_2(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32, base_mip: u32, slice: u32) {
#ifdef SUBGROUP_SUPPORT
    var v = spd_load_intermediate(x, y);
    v = spd_reduce_quad(v);
    if local_invocation_index % 4u == 0u {
        spd_store((workgroup_id * 8u) + vec2(x / 2u, y / 2u), v, base_mip, slice);
        spd_store_intermediate(x + (y / 2u) % 2u, y, v);
    }
#else
    if local_invocation_index < 64u {
        let v = spd_reduce_intermediate(
            vec2(x * 2u + 0u, y * 2u + 0u),
            vec2(x * 2u + 1u, y * 2u + 0u),
            vec2(x * 2u + 0u, y * 2u + 1u),
            vec2(x * 2u + 1u, y * 2u + 1u),
        );
        spd_store((workgroup_id * 8u) + vec2(x, y), v, base_mip, slice);
        spd_store_intermediate(x * 2u + y % 2u, y * 2u, v);
    }
#endif
}

fn spd_downsample_mip_3(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32, base_mip: u32, slice: u32) {
#ifdef SUBGROUP_SUPPORT
    if local_invocation_index < 64u {
        var v = spd_load_intermediate(x * 2u + y % 2u, y * 2u);
        v = spd_reduce_quad(v);
        if local_invocation_index % 4u == 0u {
            spd_store((workgroup_id * 4u) + vec2(x / 2u, y / 2u), v, base_mip, slice);
            spd_store_intermediate(x * 2u + y / 2u, y * 2u, v);
        }
    }
#else
    if local_invocation_index < 16u {
        let v = spd_reduce_intermediate(
            vec2(x * 4u + 0u + 0u, y * 4u + 0u),
            vec2(x * 4u + 2u + 0u, y * 4u + 0u),
            vec2(x * 4u + 0u + 1u, y * 4u + 2u),
            vec2(x * 4u + 2u + 1u, y * 4u + 2u),
        );
        spd_store((workgroup_id * 4u) + vec2(x, y), v, base_mip, slice);
        spd_store_intermediate(x * 4u + y, y * 4u, v);
    }
#endif
}

fn spd_downsample_mip_4(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32, base_mip: u32, slice: u32) {
#ifdef SUBGROUP_SUPPORT
    if local_invocation_index < 16u {
        var v = spd_load_intermediate(x * 4u + y, y * 4u);
        v = spd_reduce_quad(v);
        if local_invocation_index % 4u == 0u {
            spd_store((workgroup_id * 2u) + vec2(x / 2u, y / 2u), v, base_mip, slice);
            spd_store_intermediate(x / 2u + y, 0u, v);
        }
    }
#else
    if local_invocation_index < 4u {
        let v = spd_reduce_intermediate(
            vec2(x * 8u + 0u + 0u + y * 2u, y * 8u + 0u),
            vec2(x * 8u + 4u + 0u + y * 2u, y * 8u + 0u),
            vec2(x * 8u + 0u + 1u + y * 2u, y * 8u + 4u),
            vec2(x * 8u + 4u + 1u + y * 2u, y * 8u + 4u),
        );
        spd_store((workgroup_id * 2u) + vec2(x, y), v, base_mip, slice);
        spd_store_intermediate(x + y * 2u, 0u, v);
    }
#endif
}

fn spd_downsample_mip_5(x: u32, y: u32, workgroup_id: vec2u, local_invocation_index: u32, base_mip: u32, slice: u32) {
#ifdef SUBGROUP_SUPPORT
    if local_invocation_index < 4u {
        var v = spd_load_intermediate(local_invocation_index, 0u);
        v = spd_reduce_quad(v);
        if local_invocation_index % 4u == 0u {
            spd_store(workgroup_id, v, base_mip, slice);
        }
    }
#else
    if local_invocation_index < 1u {
        let v = spd_reduce_intermediate(vec2(0u, 0u), vec2(1u, 0u), vec2(2u, 0u), vec2(3u, 0u));
        spd_store(workgroup_id, v, base_mip, slice);
    }
#endif
}

fn spd_downsample_mips_6_7(x: u32, y: u32, mips: u32, slice: u32) {
    var tex = vec2(x * 4u + 0u, y * 4u + 0u);
    var pix = vec2(x * 2u + 0u, y * 2u + 0u);
    let v0 = spd_reduce_load_4(
        vec2(x * 4u + 0u, y * 4u + 0u),
        vec2(x * 4u + 1u, y * 4u + 0u),
        vec2(x * 4u + 0u, y * 4u + 1u),
        vec2(x * 4u + 1u, y * 4u + 1u),
        slice
    );
    spd_store(pix, v0, 6u, slice);

    tex = vec2(x * 4u + 2u, y * 4u + 0u);
    pix = vec2(x * 2u + 1u, y * 2u + 0u);
    let v1 = spd_reduce_load_4(
        vec2(x * 4u + 2u, y * 4u + 0u),
        vec2(x * 4u + 3u, y * 4u + 0u),
        vec2(x * 4u + 2u, y * 4u + 1u),
        vec2(x * 4u + 3u, y * 4u + 1u),
        slice
    );
    spd_store(pix, v1, 6u, slice);

    tex = vec2(x * 4u + 0u, y * 4u + 2u);
    pix = vec2(x * 2u + 0u, y * 2u + 1u);
    let v2 = spd_reduce_load_4(
        vec2(x * 4u + 0u, y * 4u + 2u),
        vec2(x * 4u + 1u, y * 4u + 2u),
        vec2(x * 4u + 0u, y * 4u + 3u),
        vec2(x * 4u + 1u, y * 4u + 3u),
        slice
    );
    spd_store(pix, v2, 6u, slice);

    tex = vec2(x * 4u + 2u, y * 4u + 2u);
    pix = vec2(x * 2u + 1u, y * 2u + 1u);
    let v3 = spd_reduce_load_4(
        vec2(x * 4u + 2u, y * 4u + 2u),
        vec2(x * 4u + 3u, y * 4u + 2u),
        vec2(x * 4u + 2u, y * 4u + 3u),
        vec2(x * 4u + 3u, y * 4u + 3u),
        slice
    );
    spd_store(pix, v3, 6u, slice);

    if mips < 7u { return; }

    let v = spd_reduce_4(v0, v1, v2, v3);
    spd_store(vec2(x, y), v, 7u, slice);
    spd_store_intermediate(x, y, v);
}

fn remap_for_wave_reduction(a: u32) -> vec2u {
    // This function maps linear thread IDs to 2D coordinates in a special pattern
    // to ensure that neighboring threads process neighboring pixels
    // For example, this transforms linear thread IDs 0,1,2,3 into a 2×2 square
    
    // Extract bits to form the X and Y coordinates
    let x = insertBits(extractBits(a, 2u, 3u), a, 0u, 1u);
    let y = insertBits(extractBits(a, 3u, 3u), extractBits(a, 1u, 2u), 0u, 2u);
    
    return vec2u(x, y);
}

fn spd_reduce_load_source_image(uv: vec2u, slice: u32) -> vec4f {
    let texture_coord = (vec2f(uv) + 0.5) * constants.inverse_input_size;

    #ifdef COMBINE_BIND_GROUP
    let result = textureSampleLevel(mip_0, sampler_linear_clamp, texture_coord, slice, 0.0);
    #endif
    #ifdef FIRST_PASS
    let result = textureSampleLevel(mip_0, sampler_linear_clamp, texture_coord, slice, 0.0);
    #endif
    #ifdef SECOND_PASS
    let result = textureSampleLevel(mip_6, sampler_linear_clamp, texture_coord, slice, 0.0);
    #endif

#ifdef SRGB_CONVERSION
    return vec4(
        srgb_from_linear(result.r),
        srgb_from_linear(result.g),
        srgb_from_linear(result.b),
        result.a
    );
#else
    return result;
#endif

}

fn spd_store(pix: vec2u, value: vec4f, mip: u32, slice: u32) {
    if mip >= constants.mips { return; }
    switch mip {
        #ifdef COMBINE_BIND_GROUP
        case 0u: { textureStore(mip_1, pix, slice, value); }
        case 1u: { textureStore(mip_2, pix, slice, value); }
        case 2u: { textureStore(mip_3, pix, slice, value); }
        case 3u: { textureStore(mip_4, pix, slice, value); }
        case 4u: { textureStore(mip_5, pix, slice, value); }
        case 5u: { textureStore(mip_6, pix, slice, value); }
        case 6u: { textureStore(mip_7, pix, slice, value); }
        case 7u: { textureStore(mip_8, pix, slice, value); }
        case 8u: { textureStore(mip_9, pix, slice, value); }
        case 9u: { textureStore(mip_10, pix, slice, value); }
        case 10u: { textureStore(mip_11, pix, slice, value); }
        case 11u: { textureStore(mip_12, pix, slice, value); }
        #endif
        #ifdef FIRST_PASS
        case 0u: { textureStore(mip_1, pix, slice, value); }
        case 1u: { textureStore(mip_2, pix, slice, value); }
        case 2u: { textureStore(mip_3, pix, slice, value); }
        case 3u: { textureStore(mip_4, pix, slice, value); }
        case 4u: { textureStore(mip_5, pix, slice, value); }
        case 5u: { textureStore(mip_6, pix, slice, value); }
        #endif
        #ifdef SECOND_PASS
        case 6u: { textureStore(mip_7, pix, slice, value); }
        case 7u: { textureStore(mip_8, pix, slice, value); }
        case 8u: { textureStore(mip_9, pix, slice, value); }
        case 9u: { textureStore(mip_10, pix, slice, value); }
        case 10u: { textureStore(mip_11, pix, slice, value); }
        case 11u: { textureStore(mip_12, pix, slice, value); }
        #endif
        default: {}
    }
}

fn spd_store_intermediate(x: u32, y: u32, value: vec4f) {
    spd_intermediate_r[x][y] = value.x;
    spd_intermediate_g[x][y] = value.y;
    spd_intermediate_b[x][y] = value.z;
    spd_intermediate_a[x][y] = value.w;
}

fn spd_load_intermediate(x: u32, y: u32) -> vec4f {
    return vec4(spd_intermediate_r[x][y], spd_intermediate_g[x][y], spd_intermediate_b[x][y], spd_intermediate_a[x][y]);
}

fn spd_reduce_intermediate(i0: vec2u, i1: vec2u, i2: vec2u, i3: vec2u) -> vec4f {
    let v0 = spd_load_intermediate(i0.x, i0.y);
    let v1 = spd_load_intermediate(i1.x, i1.y);
    let v2 = spd_load_intermediate(i2.x, i2.y);
    let v3 = spd_load_intermediate(i3.x, i3.y);
    return spd_reduce_4(v0, v1, v2, v3);
}

fn spd_reduce_load_4(i0: vec2u, i1: vec2u, i2: vec2u, i3: vec2u, slice: u32) -> vec4f {
    #ifdef COMBINE_BIND_GROUP
    let v0 = textureLoad(mip_6, i0, slice);
    let v1 = textureLoad(mip_6, i1, slice);
    let v2 = textureLoad(mip_6, i2, slice);
    let v3 = textureLoad(mip_6, i3, slice);
    return spd_reduce_4(v0, v1, v2, v3);
    #endif
    #ifdef FIRST_PASS
    return vec4(0.0, 0.0, 0.0, 0.0);
    #endif
    #ifdef SECOND_PASS
    let v0 = textureLoad(mip_6, i0, slice, 0);
    let v1 = textureLoad(mip_6, i1, slice, 0);
    let v2 = textureLoad(mip_6, i2, slice, 0);
    let v3 = textureLoad(mip_6, i3, slice, 0);
    return spd_reduce_4(v0, v1, v2, v3);
    #endif
}

fn spd_reduce_4(v0: vec4f, v1: vec4f, v2: vec4f, v3: vec4f) -> vec4f {
    return (v0 + v1 + v2 + v3) * 0.25;
}

#ifdef SUBGROUP_SUPPORT
fn spd_reduce_quad(v: vec4f) -> vec4f {
    let v0 = v;
    let v1 = quadSwapX(v);
    let v2 = quadSwapY(v);
    let v3 = quadSwapDiagonal(v);
    return spd_reduce_4(v0, v1, v2, v3);
}
#endif

fn srgb_from_linear(value: f32) -> f32 {
    let j = vec3(0.0031308 * 12.92, 12.92, 1.0 / 2.4);
    let k = vec2(1.055, -0.055);
    return clamp(j.x, value * j.y, pow(value, j.z) * k.x + k.y);
}