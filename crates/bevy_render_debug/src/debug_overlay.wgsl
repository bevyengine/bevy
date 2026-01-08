#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_view_bindings::depth_prepass_texture
#import bevy_pbr::mesh_view_bindings::normal_prepass_texture
#import bevy_pbr::mesh_view_bindings::motion_vector_prepass_texture
#import bevy_pbr::mesh_view_bindings::deferred_prepass_texture
#import bevy_pbr::view_transformations::depth_ndc_to_view_z
#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::pbr_deferred_types::unpack_unorm4x8_
#import bevy_pbr::pbr_deferred_types::unpack_unorm3x4_plus_unorm_20_
#import bevy_pbr::rgb9e5::rgb9e5_to_vec3_
#import bevy_pbr::utils::octahedral_decode

struct DebugBufferConfig {
    opacity: f32,
    mip_level: u32,
}

@group(1) @binding(0) var<uniform> config: DebugBufferConfig;
@group(1) @binding(1) var background_texture: texture_2d<f32>;
@group(1) @binding(2) var background_sampler: sampler;

#ifdef DEBUG_DEPTH_PYRAMID
@group(1) @binding(3) var depth_pyramid_texture: texture_2d<f32>;
@group(1) @binding(4) var depth_pyramid_sampler: sampler;
#endif

@fragment
fn fragment(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag_coord.xy / view.viewport.zw;
    let background = textureSampleLevel(background_texture, background_sampler, uv, 0.0);
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
        #ifdef DEFERRED_PREPASS
            let deferred = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);
            let normal = octahedral_decode(unpack_24bit_normal(deferred.a));
            output_color = vec4(normal * 0.5 + 0.5, 1.0);
        #else
            output_color = vec4(1.0, 0.0, 1.0, 1.0);
        #endif
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
    output_color = vec4(vec3(f32(deferred.x) / 255.0, f32(deferred.y) / 255.0, f32(deferred.z) / 255.0), 1.0);
#else
    output_color = vec4(1.0, 0.0, 1.0, 1.0);
#endif
#endif

#ifdef DEBUG_DEFERRED_BASE_COLOR
#ifdef DEFERRED_PREPASS
    let deferred = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    let base_rough = unpack_unorm4x8_(deferred.x);
    output_color = vec4(pow(base_rough.rgb, vec3(2.2)), 1.0);
#else
    output_color = vec4(1.0, 0.0, 1.0, 1.0);
#endif
#endif

#ifdef DEBUG_DEFERRED_EMISSIVE
#ifdef DEFERRED_PREPASS
    let deferred = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    let emissive = rgb9e5_to_vec3_(deferred.y);
    output_color = vec4(emissive, 1.0);
#else
    output_color = vec4(1.0, 0.0, 1.0, 1.0);
#endif
#endif

#ifdef DEBUG_DEFERRED_METALLIC_ROUGHNESS
#ifdef DEFERRED_PREPASS
    let deferred = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    let base_rough = unpack_unorm4x8_(deferred.x);
    let props = unpack_unorm4x8_(deferred.z);
    // R: Reflectance, G: Metallic, B: Occlusion, A: Perceptual Roughness
    output_color = vec4(props.r, props.g, props.b, base_rough.a);
#else
    output_color = vec4(1.0, 0.0, 1.0, 1.0);
#endif
#endif

#ifdef DEBUG_DEPTH_PYRAMID
    let depth_pyramid = textureSampleLevel(depth_pyramid_texture, depth_pyramid_sampler, uv, f32(config.mip_level)).r;
    output_color = vec4(vec3(depth_pyramid), 1.0);
#endif

    let alpha = output_color.a * config.opacity;
    return vec4(mix(background.rgb, output_color.rgb, alpha), 1.0);
}
