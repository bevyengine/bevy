import bevy_render::maths::mat4x4_to_mat3x3;
import package::render::{
    mesh_functions,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
};

@if(PREPASS_PIPELINE)
import package::prepass::io::{Vertex, VertexOutput, decompress_vertex};
@else
import package::render::forward_io::{Vertex, VertexOutput, decompress_vertex};

@if(PREPASS_PIPELINE && MOTION_VECTOR_PREPASS)
import package::prepass::bindings::previous_view_uniforms;

/// Replaces the rotation of `world_from_local` with the view-facing `basis`,
/// keeping its translation and per-axis scale.
fn billboard_transform(world_from_local: mat4x4<f32>, basis: mat3x3<f32>) -> mat4x4<f32> {
    return mat4x4<f32>(
        vec4<f32>(basis[0] * length(world_from_local[0].xyz), 0.0),
        vec4<f32>(basis[1] * length(world_from_local[1].xyz), 0.0),
        vec4<f32>(basis[2] * length(world_from_local[2].xyz), 0.0),
        world_from_local[3],
    );
}

/// Returns the billboard's rotation for a view with the given `world_from_view` axes,
/// which may be scaled along with the camera.
fn view_facing_basis(view_axes: mat3x3<f32>) -> mat3x3<f32> {
@if(BILLBOARD_LOCK_Y) {
    let up = vec3<f32>(0.0, 1.0, 0.0);
    var right = vec3<f32>(view_axes[0].x, 0.0, view_axes[0].z);
    // A camera rolled onto its side has no horizontal right axis.
    if dot(right, right) < 1e-6 * dot(view_axes[0], view_axes[0]) {
        right = cross(up, view_axes[2]);
    }
    right = normalize(right);
    var back = cross(right, up);
    // A camera rolled past that point would otherwise see the back of the billboard.
    if dot(back, view_axes[2]) < 0.0 {
        right = -right;
        back = -back;
    }
    return mat3x3<f32>(right, up, back);
}
@else {
    return mat3x3<f32>(normalize(view_axes[0]), normalize(view_axes[1]), normalize(view_axes[2]));
}
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let uncompressed_vertex_no_morph = decompress_vertex(vertex_no_morph, vertex_no_morph.instance_index);
    var vertex = uncompressed_vertex_no_morph;

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

    let basis = view_facing_basis(mat4x4_to_mat3x3(view.world_from_view));
    var world_from_local = billboard_transform(mesh_world_from_local, basis);

@if(VERTEX_POSITIONS)
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
@if(VERTEX_POSITIONS)
    out.position = position_world_to_clip(out.world_position.xyz);
@if(PREPASS_PIPELINE && UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0); // Clamp depth to avoid clipping
}

@if(VERTEX_UVS_A)
    out.uv = vertex.uv;

@if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;

@if(!PREPASS_PIPELINE || NORMAL_PREPASS_OR_DEFERRED_PREPASS) {
@if(VERTEX_NORMALS)
    out.world_normal = basis * vertex.normal;
@else
    out.world_normal = basis[2];

@if(VERTEX_TANGENTS)
    out.world_tangent = vec4<f32>(basis * vertex.tangent.xyz, vertex.tangent.w);
}

@if(VERTEX_COLORS)
    out.color = vertex.color;

    // Compute the motion vector for TAA among other purposes. For this we need
    // to know where the vertex was last frame.
@if(PREPASS_PIPELINE && MOTION_VECTOR_PREPASS) {
    let prev_vertex = uncompressed_vertex_no_morph;

    // The transpose of a rotation is its inverse, up to the scale that
    // `view_facing_basis` removes.
    let prev_basis = view_facing_basis(
        transpose(mat4x4_to_mat3x3(previous_view_uniforms.view_from_world)),
    );
    // Use vertex_no_morph.instance_index instead of prev_vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    let prev_model = billboard_transform(
        mesh_functions::get_previous_world_from_local(vertex_no_morph.instance_index),
        prev_basis,
    );

    out.previous_world_position = mesh_functions::mesh_position_local_to_world(
        prev_model,
        vec4<f32>(prev_vertex.position, 1.0)
    );
}

@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    out.instance_index = vertex_no_morph.instance_index;

@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index, mesh_world_from_local[3]);

    return out;
}
