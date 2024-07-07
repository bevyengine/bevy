#import bevy_pbr::{
    meshlet_bindings::{
        meshlet_cluster_meshlet_ids,
        meshlets,
        meshlet_vertex_ids,
        meshlet_vertex_data,
        meshlet_cluster_instance_ids,
        meshlet_instance_uniforms,
        meshlet_software_raster_clusters,
        meshlet_visibility_buffer,
        view,
        get_meshlet_index,
        unpack_meshlet_vertex,
    },
    mesh_functions::mesh_position_local_to_world,
}
#import bevy_render::maths::affine3_to_square

/// Compute shader for rasterizing small clusters into a visibility buffer.

var<workgroup> screen_space_vertices: array<vec3f, 64>;

@compute
@workgroup_size(64, 1, 1) // 64 threads per workgroup, 1 vertex/triangle per thread, 1 cluster per workgroup
fn rasterize_cluster(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
    @builtin(local_invocation_id) local_invocation_id: vec3<u32>,
) {
    // Load and project 1 vertex per thread
    let cluster_id = meshlet_software_raster_clusters[workgroup_id.x];
    let meshlet_id = meshlet_cluster_meshlet_ids[cluster_id];
    let meshlet = meshlets[meshlet_id];
    if local_invocation_id.x < meshlet.vertex_count {
        let vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + local_invocation_id.x];
        let vertex = unpack_meshlet_vertex(meshlet_vertex_data[vertex_id]);

        // Project vertex to screen space
        let instance_id = meshlet_cluster_instance_ids[cluster_id];
        let instance_uniform = meshlet_instance_uniforms[instance_id];
        let world_from_local = affine3_to_square(instance_uniform.world_from_local);
        let world_position = mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
        var clip_position = view.clip_from_world * vec4(world_position.xyz, 1.0);
    #ifdef DEPTH_CLAMP_ORTHO
        let unclamped_clip_depth = clip_position.z;
        clip_position.z = min(clip_position.z, 1.0);
    #endif
        let screen_position = clip_position.xyz / clip_position.w;

        // Write screen space vertex to workgroup shared memory
        screen_space_vertices[local_invocation_id.x] = screen_position;
    }
    workgroupBarrier();

    // Load 1 triangle's worth of vertex data per thread
    if local_invocation_id.x >= meshlet.triangle_count { return; }
    let index_ids = meshlet.start_index_id + (local_invocation_id.x * 3u) + vec3(0u, 1u, 2u);
    let vertex_ids = vec3(get_meshlet_index(index_ids[0]), get_meshlet_index(index_ids[1]), get_meshlet_index(index_ids[2]));
    let vertex_1 = screen_space_vertices[vertex_ids[0]];
    let vertex_2 = screen_space_vertices[vertex_ids[1]];
    let vertex_3 = screen_space_vertices[vertex_ids[2]];

    // TODO: Software raster
}
