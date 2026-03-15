#define_import_path bevy_sprite::mesh2d_functions

#import bevy_sprite::{
    mesh2d_view_bindings::view,
    mesh2d_bindings::mesh,
}
#import bevy_render::maths::{affine3_to_square, mat2x4_f32_to_mat3x3_unpack}

fn get_world_from_local(instance_index: u32) -> mat4x4<f32> {
    return affine3_to_square(mesh[instance_index].world_from_local);
}

fn mesh2d_position_local_to_world(world_from_local: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32> {
    return world_from_local * vertex_position;
}

fn mesh2d_position_world_to_clip(world_position: vec4<f32>) -> vec4<f32> {
    return view.clip_from_world * world_position;
}

// NOTE: The intermediate world_position assignment is important
// for precision purposes when using the 'equals' depth comparison
// function.
fn mesh2d_position_local_to_clip(world_from_local: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32> {
    let world_position = mesh2d_position_local_to_world(world_from_local, vertex_position);
    return mesh2d_position_world_to_clip(world_position);
}

fn mesh2d_normal_local_to_world(vertex_normal: vec3<f32>, instance_index: u32) -> vec3<f32> {
    return mat2x4_f32_to_mat3x3_unpack(
        mesh[instance_index].local_from_world_transpose_a,
        mesh[instance_index].local_from_world_transpose_b,
    ) * vertex_normal;
}

fn mesh2d_tangent_local_to_world(world_from_local: mat4x4<f32>, vertex_tangent: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        mat3x3<f32>(
            world_from_local[0].xyz,
            world_from_local[1].xyz,
            world_from_local[2].xyz
        ) * vertex_tangent.xyz,
        vertex_tangent.w
    );
}

fn get_tag(instance_index: u32) -> u32 {
    return mesh[instance_index].tag;
}

fn decompress_vertex_position(instance_index: u32, compressed_position: vec4<f32>) -> vec3<f32> {
    let aabb_center = bevy_sprite::mesh2d_bindings::mesh[instance_index].aabb_center;
    let aabb_half_extents = bevy_sprite::mesh2d_bindings::mesh[instance_index].aabb_half_extents;
    return aabb_center + aabb_half_extents * compressed_position.xyz;
}

fn decompress_vertex_normal(compressed_normal: vec2<f32>) -> vec3<f32> {
    return octahedral_decode_signed(compressed_normal);
}

fn decompress_vertex_tangent(compressed_tangent: vec2<f32>) -> vec4<f32> {
    return octahedral_decode_tangent(compressed_tangent);
}

fn decompress_vertex_uv(instance_index: u32, compressed_uv: vec2<f32>) -> vec2<f32> {
    let uv_range = bevy_sprite::mesh2d_bindings::mesh[instance_index].uv0_range;
    return uv_range.xy + uv_range.zw * compressed_uv;
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
