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

fn decompress_vertex_axis_angle_to_normal_tangent(
    octahedral_axis: vec2f,
    angle: f32,
    out_normal: ptr<function,vec3f>,
    out_tangent: ptr<function,vec4f>,
) {
    let axis = octahedral_decode_signed(octahedral_axis);
    axis_angle_to_normal_tangent(axis, 2.0 * bevy_render::maths::PI * angle, out_normal, out_tangent);
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

fn axis_angle_to_normal_tangent(
    axis: vec3f,
    angle: f32,
    out_normal: ptr<function,vec3f>,
    out_tangent: ptr<function,vec4f>,
) {
    let sign = select(-1.0, 1.0, angle >= 0.0);
    let angle_h = angle * 0.5 * sign;
    let c = cos(angle_h);
    let s = sin(angle_h);
    let v = axis * s;
    let rotation = vec4f(v.x, v.y, v.z, c);
    let x2 = rotation.x + rotation.x;
    let y2 = rotation.y + rotation.y;
    let z2 = rotation.z + rotation.z;
    let xx = rotation.x * x2;
    let xy = rotation.x * y2;
    let xz = rotation.x * z2;
    let yy = rotation.y * y2;
    let yz = rotation.y * z2;
    let zz = rotation.z * z2;
    let wx = rotation.w * x2;
    let wy = rotation.w * y2;
    let wz = rotation.w * z2;

    *out_tangent = vec4f(1.0 - (yy + zz), xy + wz, xz - wy, sign);
    *out_normal = vec3f(xz + wy, yz - wx, 1.0 - (xx + yy));
}
