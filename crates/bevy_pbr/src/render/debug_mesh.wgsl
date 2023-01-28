@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
#ifdef DEBUG_WORLD_POSITION
    return vec4<f32>(world_position.xyz, 1.0);
#endif

#ifdef DEBUG_WORLD_NORMAL
    return vec4<f32>(world_normal.xyz, 1.0);
#endif

#ifdef DEBUG_UVS
#ifdef VERTEX_UVS
    // Modulo to show how the UVs would tile a texture
    // if the related sampler was set to repeat.
    // This is a more useful default than to saturate/clamp.
    return vec4<f32>(uv.x % 1.0, uv.y % 1.0, 0.0, 1.0);
#else
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
#endif
#endif

#ifdef DEBUG_WORLD_TANGENT
#ifdef VERTEX_UVS
    return vec4<f32>(world_tangent.xyz, 1.0);
#else
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
#endif
#endif
}