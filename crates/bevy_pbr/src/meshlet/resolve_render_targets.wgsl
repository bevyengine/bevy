#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
@group(0) @binding(0) var<storage, read> meshlet_visibility_buffer: array<u64>; // Per pixel
#else
@group(0) @binding(0) var<storage, read> meshlet_visibility_buffer: array<u32>; // Per pixel
#endif
@group(0) @binding(1) var<storage, read> meshlet_cluster_instance_ids: array<u32>;  // Per cluster
@group(0) @binding(2) var<storage, read> meshlet_instance_material_ids: array<u32>; // Per entity instance
var<push_constant> view_width: u32;

/// This pass writes out the depth texture.
@fragment
fn resolve_depth(in: FullscreenVertexOutput) -> @builtin(frag_depth) f32 {
    let frag_coord_1d = u32(in.position.y) * view_width + u32(in.position.x);
    let visibility = meshlet_visibility_buffer[frag_coord_1d];
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
    return bitcast<f32>(u32(visibility >> 32u));
#else
    return bitcast<f32>(visibility);
#endif
}

/// This pass writes out the material depth texture.
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
@fragment
fn resolve_material_depth(in: FullscreenVertexOutput) -> @builtin(frag_depth) f32 {
    let frag_coord_1d = u32(in.position.y) * view_width + u32(in.position.x);
    let visibility = meshlet_visibility_buffer[frag_coord_1d];

    let depth = visibility >> 32u;
    if depth == 0lu { return 0.0; }

    let cluster_id = u32(visibility) >> 6u;
    let instance_id = meshlet_cluster_instance_ids[cluster_id];
    let material_id = meshlet_instance_material_ids[instance_id];
    return f32(material_id) / 65535.0;
}
#endif
