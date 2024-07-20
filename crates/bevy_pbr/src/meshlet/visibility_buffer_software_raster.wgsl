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

var<workgroup> viewport_vertices: array<vec3f, 64>;

@compute
@workgroup_size(64, 1, 1) // 64 threads per workgroup, 1 vertex/triangle per thread, 1 cluster per workgroup
fn rasterize_cluster(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_invocation_id: vec3<u32>,
) {
    let cluster_id = meshlet_software_raster_clusters[workgroup_id.x];
    let meshlet_id = meshlet_cluster_meshlet_ids[cluster_id];
    let meshlet = meshlets[meshlet_id];

    // Load and project 1 vertex per thread
    let vertex_id = local_invocation_id.x;
    if vertex_id < meshlet.vertex_count {
        let meshlet_vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + vertex_id];
        let vertex = unpack_meshlet_vertex(meshlet_vertex_data[meshlet_vertex_id]);

        // Project vertex to viewport space
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
        let viewport_position_xy = ndc_to_uv(ndc_position.xy) * view.viewport.zw;

        // Write vertex to workgroup shared memory
        viewport_vertices[vertex_id] = vec3(viewport_position_xy, ndc_position.z);
    }

    workgroupBarrier();

    // Load 1 triangle's worth of vertex data per thread
    let triangle_id = local_invocation_id.x;
    if triangle_id >= meshlet.triangle_count { return; }
    let index_ids = meshlet.start_index_id + (triangle_id * 3u) + vec3(0u, 1u, 2u);
    let vertex_ids = vec3(get_meshlet_index(index_ids[0]), get_meshlet_index(index_ids[1]), get_meshlet_index(index_ids[2]));
    let vertex_0 = viewport_vertices[vertex_ids[0]];
    let vertex_1 = viewport_vertices[vertex_ids[1]];
    let vertex_2 = viewport_vertices[vertex_ids[2]];

    // Compute triangle bounding box
    var min_x = floor(min3(vertex_0.x, vertex_1.x, vertex_2.x));
    var min_y = floor(min3(vertex_0.y, vertex_1.y, vertex_2.y));
    var max_x = ceil(max3(vertex_0.x, vertex_1.x, vertex_2.x));
    var max_y = ceil(max3(vertex_0.y, vertex_1.y, vertex_2.y));

    // Clip triangle bounding box against screen bounds
    min_x = max(min_x, 0.0);
    min_y = max(min_y, 0.0);
    max_x = min(max_x, view.viewport.z - 1.0);
    max_y = min(max_y, view.viewport.w - 1.0);

    // Setup initial edge functions
    let a = vec3(vertex_1.y - vertex_2.y, vertex_2.y - vertex_0.y, vertex_0.y - vertex_1.y);
    let b = vec3(vertex_2.x - vertex_1.x, vertex_0.x - vertex_2.x, vertex_1.x - vertex_0.x);
    var w_row = vec3(
        edge_function(vertex_1.xy, vertex_2.xy, vec2(min_x, min_y) + 0.5),
        edge_function(vertex_2.xy, vertex_0.xy, vec2(min_x, min_y) + 0.5),
        edge_function(vertex_0.xy, vertex_1.xy, vec2(min_x, min_y) + 0.5),
    );
    let inverse_double_triangle_area = 1.0 / edge_function(vertex_0.xy, vertex_1.xy, vertex_2.xy);

    let vertices_z = vec3(vertex_0.z, vertex_1.z, vertex_2.z);
    let view_width = u32(view.viewport.z);
    let packed_ids = (cluster_id << 6u) | triangle_id;

    // Iterate over every pixel in the triangle's bounding box
    for (var y = min_y; y <= max_y; y += 1.0) {
        var w = w_row;

        for (var x = min_x; x <= max_x; x += 1.0) {
            // Check if point at pixel is within triangle
            if min3(w[0], w[1], w[2]) >= 0.0 {
                // Interpolate vertex depth for the current pixel
                let z = dot(w * inverse_double_triangle_area, vertices_z);

                // Write pixel to visibility buffer
                let frag_coord_1d = u32(y) * view_width + u32(x);
                write_visibility_buffer_pixel(frag_coord_1d, z, packed_ids);
            }

            // Increment edge functions along the X-axis
            w += a;
        }

        // Increment edge functions along the Y-axis
        w_row += b;
    }
}

fn write_visibility_buffer_pixel(frag_coord_1d: u32, z: f32, packed_ids: u32) {
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
    let depth = bitcast<u32>(z);
    let visibility = (u64(depth) << 32u) | u64(packed_ids);
    atomicMax(&meshlet_visibility_buffer[frag_coord_1d], visibility);
#else ifdef DEPTH_CLAMP_ORTHO
    let depth = bitcast<u32>(z); // TODO: unclamped_clip_depth
    atomicMax(&meshlet_visibility_buffer[frag_coord_1d], depth);
#else
    let depth = bitcast<u32>(z);
    atomicMax(&meshlet_visibility_buffer[frag_coord_1d], depth);
#endif
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
