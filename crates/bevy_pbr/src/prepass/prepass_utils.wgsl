#define_import_path bevy_pbr::prepass_utils

#ifndef NORMAL_PREPASS
fn prepass_normal(frag_coord: vec4<f32>, sample_index: u32) -> vec3<f32> {
#ifdef MULTISAMPLED
    let normal_sample = textureLoad(normal_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
#else
    let normal_sample = textureLoad(normal_prepass_texture, vec2<i32>(frag_coord.xy), 0);
#endif
    return normal_sample.xyz * 2.0 - vec3(1.0);
}
#endif // NORMAL_PREPASS

#ifndef DEPTH_PREPASS
fn prepass_depth(frag_coord: vec4<f32>, sample_index: u32) -> f32 {
#ifdef MULTISAMPLED
    let depth_sample = textureLoad(depth_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
#else
    let depth_sample = textureLoad(depth_prepass_texture, vec2<i32>(frag_coord.xy), 0);
#endif
    return depth_sample;
}
#endif // DEPTH_PREPASS
