import bevy_pbr::render::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::{morph_position, morph_normal, morph_tangent},
    forward_io::{Vertex, UncompressedVertex, VertexOutput, decompress_vertex},
    view_transformations::position_world_to_clip,
};

@if(MORPH_TARGETS)
// The instance_index parameter must match vertex_in.instance_index. This is a work around for a wgpu dx12 bug.
// See https://github.com/gfx-rs/naga/issues/2416
fn morph_vertex(vertex_in: UncompressedVertex, instance_index: u32) -> UncompressedVertex {
    var vertex = vertex_in;
    let first_vertex = mesh[instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;
    let morph_descriptor_index = mesh[instance_index].morph_descriptor_index;

    let weight_count = bevy_pbr::render::morph::layer_count(morph_descriptor_index);
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = bevy_pbr::render::morph::weight_at(i, morph_descriptor_index);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph_position(vertex_index, i, morph_descriptor_index);
        @if(VERTEX_NORMALS)
        vertex.normal += weight * morph_normal(vertex_index, i, morph_descriptor_index);
        @if(VERTEX_TANGENTS)
        vertex.tangent += vec4(weight * morph_tangent(vertex_index, i, morph_descriptor_index), 0.0);
    }
    return vertex;
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let uncompressed_vertex_no_morph = decompress_vertex(vertex_no_morph, vertex_no_morph.instance_index);
@if(MORPH_TARGETS)
    var vertex = morph_vertex(uncompressed_vertex_no_morph, vertex_no_morph.instance_index);
@else
    var vertex = uncompressed_vertex_no_morph;

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

@if(SKINNED)
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416 .
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
@else
    var world_from_local = mesh_world_from_local;

@if(VERTEX_NORMALS && SKINNED)
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
@if(VERTEX_NORMALS && !SKINNED)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );

@if(VERTEX_POSITIONS)
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
@if(VERTEX_POSITIONS)
    out.position = position_world_to_clip(out.world_position.xyz);

@if(VERTEX_UVS_A)
    out.uv = vertex.uv;
@if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;

@if(VERTEX_TANGENTS)
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );

@if(VERTEX_COLORS)
    out.color = vertex.color;

@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    out.instance_index = vertex_no_morph.instance_index;

@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index, mesh_world_from_local[3]);

    return out;
}

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
@if(VERTEX_COLORS)
    return mesh.color;
@else
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}
