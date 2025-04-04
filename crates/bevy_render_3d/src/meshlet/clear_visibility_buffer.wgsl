#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
@group(0) @binding(0) var meshlet_visibility_buffer: texture_storage_2d<r64uint, write>;
#else
@group(0) @binding(0) var meshlet_visibility_buffer: texture_storage_2d<r32uint, write>;
#endif
var<push_constant> view_size: vec2<u32>;

@compute
@workgroup_size(16, 16, 1)
fn clear_visibility_buffer(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= view_size) { return; }

#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
    textureStore(meshlet_visibility_buffer, global_id.xy, vec4(0lu));
#else
    textureStore(meshlet_visibility_buffer, global_id.xy, vec4(0u));
#endif
}
