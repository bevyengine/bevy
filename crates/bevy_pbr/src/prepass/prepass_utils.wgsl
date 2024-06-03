#define_import_path bevy_pbr::prepass_utils

#import bevy_pbr::{
    mesh_view_bindings as view_bindings,
    pbr_deferred_functions::deferred_gbuffer_from_pbr_input,
}

#ifdef DEPTH_PREPASS
fn prepass_depth(frag_coord: vec4<f32>, sample_index: u32) -> f32 {
#ifdef MULTISAMPLED
    return textureLoad(view_bindings::depth_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
#else // MULTISAMPLED
    return textureLoad(view_bindings::depth_prepass_texture, vec2<i32>(frag_coord.xy), 0);
#endif // MULTISAMPLED
}
#endif // DEPTH_PREPASS

#ifdef NORMAL_PREPASS
fn prepass_normal(frag_coord: vec4<f32>, sample_index: u32) -> vec3<f32> {
#ifdef MULTISAMPLED
    let normal_sample = textureLoad(view_bindings::normal_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
#else
    let normal_sample = textureLoad(view_bindings::normal_prepass_texture, vec2<i32>(frag_coord.xy), 0);
#endif // MULTISAMPLED
    return normalize(normal_sample.xyz * 2.0 - vec3(1.0));
}
#endif // NORMAL_PREPASS

#ifdef MOTION_VECTOR_PREPASS
fn prepass_motion_vector(frag_coord: vec4<f32>, sample_index: u32) -> vec2<f32> {
#ifdef MULTISAMPLED
    let motion_vector_sample = textureLoad(view_bindings::motion_vector_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
#else
    let motion_vector_sample = textureLoad(view_bindings::motion_vector_prepass_texture, vec2<i32>(frag_coord.xy), 0);
#endif
    return motion_vector_sample.rg;
}
#endif // MOTION_VECTOR_PREPASS

#ifdef PREPASS_PIPELINE
fn prepass_output(in: VertexOutput, pbr_input: PbrInput) -> FragmentOutput {
    var out: FragmentOutput;

    // gbuffer
#ifdef DEFERRED_PREPASS
    out.deferred = deferred_gbuffer_from_pbr_input(pbr_input);
    // lighting pass id (used to determine which lighting shader to run for the fragment)
    out.deferred_lighting_pass_id = pbr_input.material.deferred_lighting_pass_id;
#endif
    // normal if required
#ifdef NORMAL_PREPASS
    out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
#endif
    // motion vectors if required
#ifdef MOTION_VECTOR_PREPASS
#ifdef MESHLET_MESH_MATERIAL_PASS
    out.motion_vector = in.motion_vector;
#else
    out.motion_vector = calculate_motion_vector(in.world_position, in.previous_world_position);
#endif
#endif

    return out;
}
#endif
