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
    view_transformations::ndc_to_uv,
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
    let triangle_id = local_invocation_id.x;
    if triangle_id < meshlet.vertex_count {
        let vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + triangle_id];
        let vertex = unpack_meshlet_vertex(meshlet_vertex_data[vertex_id]);

        // Project vertex to screen space
        let instance_id = meshlet_cluster_instance_ids[cluster_id];
        let instance_uniform = meshlet_instance_uniforms[instance_id];
        let world_from_local = affine3_to_square(instance_uniform.world_from_local);
        let world_position = mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
        var clip_position = view.clip_from_world * vec4(world_position.xyz, 1.0);
    #ifdef DEPTH_CLAMP_ORTHO
        let unclamped_clip_depth = clip_position.z; // TODO: What to do with this?
        clip_position.z = min(clip_position.z, 1.0);
    #endif
        let ndc_position = clip_position.xyz / clip_position.w;
        let screen_position_xy = ndc_to_uv(ndc_position.xy) * view.viewport.zw;
        let screen_position = vec3(screen_position_xy, ndc_position.z);

        // Write screen space vertex to workgroup shared memory
        screen_space_vertices[triangle_id] = screen_position;
    }
    workgroupBarrier();

    // Load 1 triangle's worth of vertex data per thread
    if triangle_id >= meshlet.triangle_count { return; }
    let index_ids = meshlet.start_index_id + (triangle_id * 3u) + vec3(0u, 1u, 2u);
    let vertex_ids = vec3(get_meshlet_index(index_ids[0]), get_meshlet_index(index_ids[1]), get_meshlet_index(index_ids[2]));
    let vertex_1 = screen_space_vertices[vertex_ids[0]];
    let vertex_2 = screen_space_vertices[vertex_ids[1]];
    let vertex_3 = screen_space_vertices[vertex_ids[2]];

    // Compute triangle bounding box
    var min_x = floor(min3(vertex_1.x, vertex_2.x, vertex_3.x));
    var min_y = floor(min3(vertex_1.y, vertex_2.y, vertex_3.y));
    var max_x = ceil(max3(vertex_1.x, vertex_2.x, vertex_3.x));
    var max_y = ceil(max3(vertex_1.y, vertex_2.y, vertex_3.y));

    // Clip triangle bounding box against screen bounds
    min_x = max(min_x, 0.0);
    min_y = max(min_y, 0.0);
    max_x = min(min_x, view.viewport.z - 1.0);
    max_y = min(min_y, view.viewport.w - 1.0);

    let cluster_id_packed = cluster_id << 6u;
    let double_triangle_area = edge_function(vertex_1.xy, vertex_2.xy, vertex_3.xy);

    // Iterate over every pixel in the triangle's bounding box
    for (var y = min_y; y <= max_y; y += 1.0) {
        for (var x = min_x; x <= max_x; x += 1.0) {
            let x = x + 0.5;
            let y = y + 0.5;

            // Calculate edge functions for the current pixel
            let w0 = edge_function(vertex_2.xy, vertex_3.xy, vec2(x, y));
            let w1 = edge_function(vertex_3.xy, vertex_1.xy, vec2(x, y));
            let w2 = edge_function(vertex_1.xy, vertex_2.xy, vec2(x, y));

            // Check if point at pixel is within triangle
            if min3(w0, w1, w2) >= 0.0 {
                // Interpolate vertex depth for the current pixel
                let barycentrics = vec3(w0, w1, w2) / double_triangle_area;
                let z = dot(barycentrics, vec3(vertex_1.z, vertex_2.z, vertex_3.z));

                let frag_coord_1d = u32(y) * u32(view.viewport.z) + u32(x);

                // TODO: Remove dummy
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
                let depth = bitcast<u32>(z);
                let visibility = (u64(depth) << 32u) | u64(cluster_id_packed | triangle_id);
                let dummy = atomicMax(&meshlet_visibility_buffer[frag_coord_1d], visibility);
#else ifdef DEPTH_CLAMP_ORTHO
                let depth = bitcast<u32>(z); // TODO: unclamped_clip_depth
                let dummy = atomicMax(&meshlet_visibility_buffer[frag_coord_1d], depth);
#else
                let depth = bitcast<u32>(z);
                let dummy = atomicMax(&meshlet_visibility_buffer[frag_coord_1d], depth);
#endif
            }
        }
    }
}

fn edge_function(a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> f32 {
    return (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
}

fn min3(a: f32, b: f32, c: f32) -> f32 {
    return min(a, min(b, c));
}

fn max3(a: f32, b: f32, c: f32) -> f32 {
    return max(a, max(b, c));
}
