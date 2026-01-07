#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_view_bindings::depth_prepass_texture
#import bevy_pbr::mesh_view_bindings::normal_prepass_texture
#import bevy_pbr::mesh_view_bindings::motion_vector_prepass_texture
#import bevy_pbr::mesh_view_bindings::deferred_prepass_texture
#import bevy_pbr::view_transformations::depth_ndc_to_view_z

struct DebugBufferConfig {
    opacity: f32,
    mip_level: u32,
}

@group(1) @binding(0) var<uniform> config: DebugBufferConfig;

#ifdef DEBUG_DEPTH_PYRAMID
@group(1) @binding(1) var depth_pyramid_texture: texture_2d<f32>;
@group(1) @binding(2) var depth_pyramid_sampler: sampler;
#endif

@fragment
fn fragment(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag_coord.xy / view.viewport.zw;
    var output_color: vec4<f32> = vec4(0.0);

#ifdef DEBUG_DEPTH
#ifdef DEPTH_PREPASS
    let depth = textureLoad(depth_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    // Linearize depth for visualization
    let linearized_depth = -depth_ndc_to_view_z(depth);
    // Use a reasonable range for visualization, e.g. 0 to 20 units
    output_color = vec4(vec3(linearized_depth / 20.0), 1.0);
#else
    output_color = vec4(1.0, 0.0, 1.0, 1.0);
#endif
#endif

#ifdef DEBUG_NORMAL
#ifdef NORMAL_PREPASS
    let normal_sample = textureLoad(normal_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    output_color = vec4(normal_sample.xyz, 1.0);
#else
    output_color = vec4(1.0, 0.0, 1.0, 1.0);
#endif
#endif

#ifdef DEBUG_MOTION_VECTORS
#ifdef MOTION_VECTOR_PREPASS
    let motion_vector = textureLoad(motion_vector_prepass_texture, vec2<i32>(frag_coord.xy), 0).rg;
    // These motion vectors are stored in a format where 1.0 represents full-screen movement.
    // We use a power curve to amplify small movements while keeping them centered.
    let mapped_motion = sign(motion_vector) * pow(abs(motion_vector), vec2(0.2)) * 0.5 + 0.5;
    output_color = vec4(mapped_motion, 0.5, 1.0);
#else
    output_color = vec4(1.0, 0.0, 1.0, 1.0);
#endif
#endif

#ifdef DEBUG_DEFERRED
#ifdef DEFERRED_PREPASS
    let deferred = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    // Just show raw bits as colors for now
    output_color = vec4(vec3(f32(deferred.x) / 255.0, f32(deferred.y) / 255.0, f32(deferred.z) / 255.0), 1.0);
#else
    output_color = vec4(1.0, 0.0, 1.0, 1.0);
#endif
#endif

#ifdef DEBUG_DEPTH_PYRAMID
    let depth_pyramid = textureSampleLevel(depth_pyramid_texture, depth_pyramid_sampler, uv, f32(config.mip_level)).r;
    output_color = vec4(vec3(depth_pyramid), 1.0);
#endif

    return vec4(output_color.rgb, output_color.a * config.opacity);
}
