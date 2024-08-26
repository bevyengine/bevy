#import bevy_pbr::{
    meshlet_bindings::{
        meshlet_cluster_meshlet_ids,
        meshlets,
        meshlet_vertex_ids,
        meshlet_vertex_data,
        meshlet_cluster_instance_ids,
        meshlet_instance_uniforms,
        meshlet_raster_clusters,
        meshlet_visibility_buffer,
        view,
        get_meshlet_index,
        unpack_meshlet_vertex,
    },
    mesh_functions::mesh_position_local_to_world,
}
#import bevy_render::maths::affine3_to_square
var<push_constant> meshlet_raster_cluster_rightmost_slot: u32;

/// Vertex/fragment shader for rasterizing large clusters into a visibility buffer.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
    @location(0) @interpolate(flat) packed_ids: u32,
#endif
#ifdef DEPTH_CLAMP_ORTHO
    @location(0) unclamped_clip_depth: f32,
#endif
}

@vertex
fn vertex(@builtin(instance_index) instance_index: u32, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let cluster_id = meshlet_raster_clusters[meshlet_raster_cluster_rightmost_slot - instance_index];
    let meshlet_id = meshlet_cluster_meshlet_ids[cluster_id];
    let meshlet = meshlets[meshlet_id];

    let triangle_id = vertex_index / 3u;
    if triangle_id >= meshlet.triangle_count { return dummy_vertex(); }
    let index_id = (triangle_id * 3u) + (vertex_index % 3u);
    let index = get_meshlet_index(meshlet.start_index_id + index_id);
    let vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + index];
    let vertex = unpack_meshlet_vertex(meshlet_vertex_data[vertex_id]);

    let instance_id = meshlet_cluster_instance_ids[cluster_id];
    let instance_uniform = meshlet_instance_uniforms[instance_id];

    let world_from_local = affine3_to_square(instance_uniform.world_from_local);
    let world_position = mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    var clip_position = view.clip_from_world * vec4(world_position.xyz, 1.0);
#ifdef DEPTH_CLAMP_ORTHO
    let unclamped_clip_depth = clip_position.z;
    clip_position.z = min(clip_position.z, 1.0);
#endif

    return VertexOutput(
        clip_position,
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
        (cluster_id << 6u) | triangle_id,
#endif
#ifdef DEPTH_CLAMP_ORTHO
        unclamped_clip_depth,
#endif
    );
}

@fragment
fn fragment(vertex_output: VertexOutput) {
    let frag_coord_1d = u32(vertex_output.position.y) * u32(view.viewport.z) + u32(vertex_output.position.x);

#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
    let depth = bitcast<u32>(vertex_output.position.z);
    let visibility = (u64(depth) << 32u) | u64(vertex_output.packed_ids);
    atomicMax(&meshlet_visibility_buffer[frag_coord_1d], visibility);
#else ifdef DEPTH_CLAMP_ORTHO
    let depth = bitcast<u32>(vertex_output.unclamped_clip_depth);
    atomicMax(&meshlet_visibility_buffer[frag_coord_1d], depth);
#else
    let depth = bitcast<u32>(vertex_output.position.z);
    atomicMax(&meshlet_visibility_buffer[frag_coord_1d], depth);
#endif
}

fn dummy_vertex() -> VertexOutput {
    return VertexOutput(
        vec4(0.0),
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
        0u,
#endif
#ifdef DEPTH_CLAMP_ORTHO
        0.0,
#endif
    );
}
