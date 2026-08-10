import bevy_pbr::render::mesh_view_bindings as view_bindings;

// Under MULTIVIEW the non-multisampled prepass bindings are `_array` variants,
// so the read indexes this eye's layer via `current_view_index`. The
// multisampled bindings stay single-layer (WGSL has no
// `texture_depth_multisampled_2d_array`), so that path is unchanged.
@if(DEPTH_PREPASS)
fn prepass_depth(frag_coord: vec4<f32>, sample_index: u32) -> f32 {
    @if(MULTISAMPLED)
    return textureLoad(view_bindings::depth_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
    @elif(MULTIVIEW)
    return textureLoad(view_bindings::depth_prepass_texture, vec2<i32>(frag_coord.xy), view_bindings::current_view_index, 0);
    @else
    return textureLoad(view_bindings::depth_prepass_texture, vec2<i32>(frag_coord.xy), 0);
}

@if(NORMAL_PREPASS)
fn prepass_normal(frag_coord: vec4<f32>, sample_index: u32) -> vec3<f32> {
    @if(MULTISAMPLED)
    let normal_sample = textureLoad(view_bindings::normal_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
    @elif(MULTIVIEW)
    let normal_sample = textureLoad(view_bindings::normal_prepass_texture, vec2<i32>(frag_coord.xy), view_bindings::current_view_index, 0);
    @else
    let normal_sample = textureLoad(view_bindings::normal_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    return normalize(normal_sample.xyz * 2.0 - vec3(1.0));
}

@if(MOTION_VECTOR_PREPASS)
fn prepass_motion_vector(frag_coord: vec4<f32>, sample_index: u32) -> vec2<f32> {
    @if(MULTISAMPLED)
    let motion_vector_sample = textureLoad(view_bindings::motion_vector_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
    @elif(MULTIVIEW)
    let motion_vector_sample = textureLoad(view_bindings::motion_vector_prepass_texture, vec2<i32>(frag_coord.xy), view_bindings::current_view_index, 0);
    @else
    let motion_vector_sample = textureLoad(view_bindings::motion_vector_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    return motion_vector_sample.rg;
}
