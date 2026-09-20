// Prepass vertex shader for hair strand ribbons.
//
// Feeds the depth / normal / motion-vector prepasses and the shadow map
// passes with the same view-facing ribbon geometry as `hair.wesl`, so hair
// casts shadows and occludes correctly. The default prepass fragment shader
// is used unchanged.

import package::hair_functions::{HairVertex, decode_params, strand_width_scale, expand_ribbon};
import bevy_pbr::{
    prepass::io::VertexOutput,
    render::{
        mesh_functions,
        view_transformations::position_world_to_clip,
    },
};

@vertex
fn vertex(vertex: HairVertex) -> VertexOutput {
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let params = decode_params(vertex.params);
    // Level of detail is decided once, from this frame's transform, so the
    // previous position below expands the same strands to the same width.
    let width_scale = strand_width_scale(world_from_local, params.z);
    let ribbon = expand_ribbon(
        world_from_local,
        vertex.position,
        vertex.tangent.xyz,
        params.x,
        params.y,
        width_scale,
    );

    var out: VertexOutput;
    out.world_position = ribbon.world_position;
    out.position = position_world_to_clip(ribbon.world_position.xyz);

@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0); // Clamp depth to avoid clipping
}

@if(NORMAL_PREPASS_OR_DEFERRED_PREPASS)
    out.world_normal = ribbon.world_normal;

@if(MOTION_VECTOR_PREPASS) {
    // Expand against last frame's transform so the ribbon edge moves with the
    // strand rather than popping when the object moves.
    let previous_world_from_local = mesh_functions::get_previous_world_from_local(vertex.instance_index);
    let previous_ribbon = expand_ribbon(
        previous_world_from_local,
        vertex.position,
        vertex.tangent.xyz,
        params.x,
        params.y,
        width_scale,
    );
    out.previous_world_position = previous_ribbon.world_position;
}

@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex.instance_index;

@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex.instance_index, world_from_local[3]);

    return out;
}
