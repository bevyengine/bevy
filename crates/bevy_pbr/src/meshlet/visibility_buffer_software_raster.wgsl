#import bevy_pbr::{
    meshlet_bindings::{
        meshlet_cluster_meshlet_ids,
        meshlets,
        meshlet_vertex_ids,
        meshlet_vertex_data,
        meshlet_cluster_instance_ids,
        meshlet_instance_uniforms,
        meshlet_raster_clusters,
        meshlet_software_raster_cluster_count,
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

// TODO: Subpixel precision and top-left rule

var<workgroup> viewport_vertices: array<vec3f, 64>;

@compute
@workgroup_size(64, 1, 1) // 64 threads per workgroup, 1 vertex/triangle per thread, 1 cluster per workgroup
fn rasterize_cluster(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_index) local_invocation_index: u32,
#ifdef MESHLET_2D_DISPATCH
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
#endif
) {
    var workgroup_id_1d = workgroup_id.x;

#ifdef MESHLET_2D_DISPATCH
    workgroup_id_1d += workgroup_id.y * num_workgroups.x;
    if workgroup_id_1d >= meshlet_software_raster_cluster_count { return; }
#endif

    let cluster_id = meshlet_raster_clusters[workgroup_id_1d];
    let meshlet_id = meshlet_cluster_meshlet_ids[cluster_id];
    let meshlet = meshlets[meshlet_id];

    // Load and project 1 vertex per thread
    let vertex_id = local_invocation_index;
    if vertex_id < meshlet.vertex_count {
        let meshlet_vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + vertex_id];
        let vertex = unpack_meshlet_vertex(meshlet_vertex_data[meshlet_vertex_id]);

        // Project vertex to viewport space
        let instance_id = meshlet_cluster_instance_ids[cluster_id];
        let instance_uniform = meshlet_instance_uniforms[instance_id];
        let world_from_local = affine3_to_square(instance_uniform.world_from_local);
        let world_position = mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
        var clip_position = view.clip_from_world * vec4(world_position.xyz, 1.0);
        var ndc_position = clip_position.xyz / clip_position.w;
#ifdef DEPTH_CLAMP_ORTHO
        ndc_position.z = 1.0 / clip_position.z;
#endif
        let viewport_position_xy = ndc_to_uv(ndc_position.xy) * view.viewport.zw;

        // Write vertex to workgroup shared memory
        viewport_vertices[vertex_id] = vec3(viewport_position_xy, ndc_position.z);
    }

    workgroupBarrier();

    // Load 1 triangle's worth of vertex data per thread
    let triangle_id = local_invocation_index;
    if triangle_id >= meshlet.triangle_count { return; }
    let index_ids = meshlet.start_index_id + (triangle_id * 3u) + vec3(0u, 1u, 2u);
    let vertex_ids = vec3(get_meshlet_index(index_ids[0]), get_meshlet_index(index_ids[1]), get_meshlet_index(index_ids[2]));
    let vertex_0 = viewport_vertices[vertex_ids[2]];
    let vertex_1 = viewport_vertices[vertex_ids[1]];
    let vertex_2 = viewport_vertices[vertex_ids[0]];
    let packed_ids = (cluster_id << 6u) | triangle_id;

    // Compute triangle bounding box
    let min_x = u32(min3(vertex_0.x, vertex_1.x, vertex_2.x));
    let min_y = u32(min3(vertex_0.y, vertex_1.y, vertex_2.y));
    var max_x = u32(ceil(max3(vertex_0.x, vertex_1.x, vertex_2.x)));
    var max_y = u32(ceil(max3(vertex_0.y, vertex_1.y, vertex_2.y)));
    max_x = min(max_x, u32(view.viewport.z) - 1u);
    max_y = min(max_y, u32(view.viewport.w) - 1u);
    if any(vec2(min_x, min_y) > vec2(max_x, max_y)) { return; }

    // Setup triangle gradients
    let w_x = vec3(vertex_1.y - vertex_2.y, vertex_2.y - vertex_0.y, vertex_0.y - vertex_1.y);
    let w_y = vec3(vertex_2.x - vertex_1.x, vertex_0.x - vertex_2.x, vertex_1.x - vertex_0.x);
    let triangle_double_area = edge_function(vertex_0.xy, vertex_1.xy, vertex_2.xy); // TODO: Reuse earlier calculations and take advantage of summing to 1
    let vertices_z = vec3(vertex_0.z, vertex_1.z, vertex_2.z) / triangle_double_area;
    let z_x = dot(vertices_z, w_x);
    let z_y = dot(vertices_z, w_y);

    // Setup initial triangle equations
    let starting_pixel = vec2(f32(min_x), f32(min_y)) + 0.5;
    var w_row = vec3(
        // TODO: Reuse earlier calculations and take advantage of summing to 1
        edge_function(vertex_1.xy, vertex_2.xy, starting_pixel),
        edge_function(vertex_2.xy, vertex_0.xy, starting_pixel),
        edge_function(vertex_0.xy, vertex_1.xy, starting_pixel),
    );
    var z_row = dot(vertices_z, w_row);
    let view_width = u32(view.viewport.z);
    var frag_coord_1d_row = min_y * view_width;

    // Rasterize triangle
    if subgroupAny(max_x - min_x > 4u) {
        // Scanline setup
        let edge_012 = -w_x;
        let open_edge = edge_012 < vec3(0.0);
        let inverse_edge_012 = select(1.0 / edge_012, vec3(1e8), edge_012 == vec3(0.0));
        let max_x_diff = vec3<f32>(max_x - min_x);
        for (var y = min_y; y <= max_y; y++) {
            // Calculate start and end X interval for pixels in this row within the triangle
            let cross_x = w_row * inverse_edge_012;
            let min_x2 = select(vec3(0.0), cross_x, open_edge);
            let max_x2 = select(cross_x, max_x_diff, open_edge);
            var x0 = u32(ceil(max3(min_x2[0], min_x2[1], min_x2[2])));
            var x1 = u32(min3(max_x2[0], max_x2[1], max_x2[2]));

            var w = w_row + w_x * f32(x0);
            var z = z_row + z_x * f32(x0);
            x0 += min_x;
            x1 += min_x;

            // Iterate scanline X interval
            for (var x = x0; x <= x1; x++) {
                // Check if point at pixel is within triangle (TODO: this shouldn't be needed, but there's bugs without it)
                if min3(w[0], w[1], w[2]) >= 0.0 {
                    write_visibility_buffer_pixel(frag_coord_1d_row + x, z, packed_ids);
                }

                // Increment edge functions along the X-axis
                w += w_x;
                z += z_x;
            }

            // Increment edge functions along the Y-axis
            w_row += w_y;
            z_row += z_y;
            frag_coord_1d_row += view_width;
        }
    } else {
        // Iterate over every pixel in the triangle's bounding box
        for (var y = min_y; y <= max_y; y++) {
            var w = w_row;
            var z = z_row;

            for (var x = min_x; x <= max_x; x++) {
                // Check if point at pixel is within triangle
                if min3(w[0], w[1], w[2]) >= 0.0 {
                    write_visibility_buffer_pixel(frag_coord_1d_row + x, z, packed_ids);
                }

                // Increment edge functions along the X-axis
                w += w_x;
                z += z_x;
            }

            // Increment edge functions along the Y-axis
            w_row += w_y;
            z_row += z_y;
            frag_coord_1d_row += view_width;
        }
    }
}

fn write_visibility_buffer_pixel(frag_coord_1d: u32, z: f32, packed_ids: u32) {
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
    let depth = bitcast<u32>(z);
    let visibility = (u64(depth) << 32u) | u64(packed_ids);
    atomicMax(&meshlet_visibility_buffer[frag_coord_1d], visibility);
#else ifdef DEPTH_CLAMP_ORTHO
    let depth = bitcast<u32>(1.0 / z);
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
