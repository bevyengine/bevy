#define_import_path bevy_render::utils


fn decompress_vertex_position(compressed_position: vec4<f32>, aabb_center: vec3<f32>, aabb_half_extents: vec3<f32>) -> vec3<f32> {
    return aabb_center + aabb_half_extents * compressed_position.xyz;
}

fn decompress_vertex_normal(compressed_normal: vec2<f32>) -> vec3<f32> {
    return octahedral_decode_signed(compressed_normal);
}

fn decompress_vertex_tangent(compressed_tangent: vec2<f32>) -> vec4<f32> {
    return octahedral_decode_tangent(compressed_tangent);
}

fn decompress_vertex_uv(compressed_uv: vec2<f32>, uv_min_and_extents: vec4<f32>) -> vec2<f32> {
    return uv_min_and_extents.xy + uv_min_and_extents.zw * compressed_uv;
}

// For decoding normals or unit direction vectors from octahedral coordinates. Input is [-1, 1].
fn octahedral_decode_signed(v: vec2<f32>) -> vec3<f32> {
    var n = vec3(v.xy, 1.0 - abs(v.x) - abs(v.y));
    let t = saturate(-n.z);
    let w = select(vec2(t), vec2(-t), n.xy >= vec2(0.0));
    n = vec3(n.xy + w, n.z);
    return normalize(n);
}

// Decode tangent vectors from octahedral coordinates and return the sign. Input is [-1, 1]. The y component should have been mapped to always be positive and then encoded the sign.
fn octahedral_decode_tangent(v: vec2<f32>) -> vec4<f32> {
    let sign = select(-1.0, 1.0, v.y >= 0.0);
    var f = v;
    f.y = abs(f.y);
    f.y = f.y * 2.0 - 1.0;
    return vec4<f32>(octahedral_decode_signed(f), sign);
}

// https://jcgt.org/published/0003/02/01/paper.pdf
// For encoding normals or unit direction vectors as octahedral coordinates.
fn octahedral_encode(v: vec3<f32>) -> vec2<f32> {
    var n = v / (abs(v.x) + abs(v.y) + abs(v.z));
    let octahedral_wrap = (1.0 - abs(n.yx)) * select(vec2(-1.0), vec2(1.0), n.xy > vec2f(0.0));
    let n_xy = select(octahedral_wrap, n.xy, n.z >= 0.0);
    return n_xy * 0.5 + 0.5;
}

// For decoding normals or unit direction vectors from octahedral coordinates.
fn octahedral_decode(v: vec2<f32>) -> vec3<f32> {
    let f = v * 2.0 - 1.0;
    return octahedral_decode_signed(f);
}
