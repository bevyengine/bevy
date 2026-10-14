// Box-filter downsample for one 3D noise mip level (src mip N -> dst mip N+1).
@group(0) @binding(13) var src_texture: texture_3d<f32>;
@group(0) @binding(14) var dst_texture: texture_storage_3d<rgba16float, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(dst_texture);
    if (gid.x >= dims.x || gid.y >= dims.y || gid.z >= dims.z) {
        return;
    }

    let base = vec3<i32>(gid) * 2;
    var sum = vec4<f32>(0.0);
    for (var zo = 0; zo < 2; zo++) {
        for (var yo = 0; yo < 2; yo++) {
            for (var xo = 0; xo < 2; xo++) {
                sum += textureLoad(src_texture, base + vec3<i32>(xo, yo, zo), 0);
            }
        }
    }
    textureStore(dst_texture, vec3<i32>(gid), sum * 0.125);
}
