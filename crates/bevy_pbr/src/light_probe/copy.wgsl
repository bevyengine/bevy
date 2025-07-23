// Copy the base mip (level 0) from a source cubemap to a destination cubemap,
// performing format conversion if needed (the destination is always rgba16float).
// The alpha channel is filled with 1.0.

@group(0) @binding(0) var src_cubemap: texture_2d_array<f32>;
@group(0) @binding(1) var dst_cubemap: texture_storage_2d_array<rgba16float, write>;

@compute
@workgroup_size(8, 8, 1)
fn copy(@builtin(global_invocation_id) global_id: vec3u) {
    let size = textureDimensions(src_cubemap).xy;

    // Bounds check
    if (any(global_id.xy >= size)) {
        return;
    }

    let color = textureLoad(src_cubemap, vec2u(global_id.xy), global_id.z, 0);

    textureStore(dst_cubemap, vec2u(global_id.xy), global_id.z, vec4f(color.rgb, 1.0));
} 