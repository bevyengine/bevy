import bevy_pbr::render::mesh_view_bindings as view_bindings;

@if(DEPTH_PREPASS)
fn prepass_depth(frag_coord: vec4<f32>, sample_index: u32) -> f32 {
    @if(MULTISAMPLED)
    return textureLoad(view_bindings::depth_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
    @else // MULTISAMPLED
    return textureLoad(view_bindings::depth_prepass_texture, vec2<i32>(frag_coord.xy), 0);
}

@if(NORMAL_PREPASS)
fn prepass_normal(frag_coord: vec4<f32>, sample_index: u32) -> vec3<f32> {
    @if(MULTISAMPLED)
    let normal_sample = textureLoad(view_bindings::normal_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
    @else
    let normal_sample = textureLoad(view_bindings::normal_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    return normalize(normal_sample.xyz * 2.0 - vec3(1.0));
}

@if(MOTION_VECTOR_PREPASS)
fn prepass_motion_vector(frag_coord: vec4<f32>, sample_index: u32) -> vec2<f32> {
    @if(MULTISAMPLED)
    let motion_vector_sample = textureLoad(view_bindings::motion_vector_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
    @else
    let motion_vector_sample = textureLoad(view_bindings::motion_vector_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    return motion_vector_sample.rg;
}
