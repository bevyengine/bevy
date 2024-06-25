#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var meshlet_visibility_buffer: texture_2d<u32>;
@group(0) @binding(1) var<storage, read> meshlet_cluster_instance_ids: array<u32>;  // Per cluster
@group(0) @binding(2) var<storage, read> meshlet_instance_material_ids: array<u32>; // Per entity instance

/// This pass writes out the material depth texture.
@fragment
fn resolve_material_depth(in: FullscreenVertexOutput) -> @builtin(frag_depth) f32 {
    // TODO: Return 0.0 if the depth of this pixel is the background
    let cluster_id = textureLoad(meshlet_visibility_buffer, vec2<i32>(in.position.xy), 0).r >> 6u;
    let instance_id = meshlet_cluster_instance_ids[cluster_id];
    let material_id = meshlet_instance_material_ids[instance_id];
    return f32(material_id) / 65535.0;
}
