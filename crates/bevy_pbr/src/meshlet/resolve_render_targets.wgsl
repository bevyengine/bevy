#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::meshlet_bindings::InstancedOffset

#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
@group(0) @binding(0) var meshlet_visibility_buffer: texture_storage_2d<r64uint, read>;
#else
@group(0) @binding(0) var meshlet_visibility_buffer: texture_storage_2d<r32uint, read>;
#endif
@group(0) @binding(1) var<storage, read> meshlet_raster_clusters: array<InstancedOffset>;  // Per cluster
@group(0) @binding(2) var<storage, read> meshlet_instance_material_ids: array<u32>; // Per entity instance

/// This pass writes out the depth texture.
@fragment
fn resolve_depth(in: FullscreenVertexOutput) -> @builtin(frag_depth) f32 {
    let visibility = textureLoad(meshlet_visibility_buffer, vec2<u32>(in.position.xy)).r;
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
    let depth = u32(visibility >> 32u);
#else
    let depth = visibility;
#endif

    if depth == 0u { discard; }

    return bitcast<f32>(depth);
}

/// This pass writes out the material depth texture.
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
@fragment
fn resolve_material_depth(in: FullscreenVertexOutput) -> @builtin(frag_depth) f32 {
    let visibility = textureLoad(meshlet_visibility_buffer, vec2<u32>(in.position.xy)).r;

    let depth = visibility >> 32u;
    if depth == 0lu { discard; }

    let cluster_id = u32(visibility) >> 7u;
    let instance_id = meshlet_raster_clusters[cluster_id].instance_id;
    let material_id = meshlet_instance_material_ids[instance_id];
    return f32(material_id) / 65535.0;
}
#endif
