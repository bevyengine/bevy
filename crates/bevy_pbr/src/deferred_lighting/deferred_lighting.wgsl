
#import bevy_core_pipeline::fullscreen_vertex_shader
#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings

#import bevy_pbr::pbr_types
#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::shadows
#import bevy_pbr::fog
#import bevy_pbr::pbr_functions
#import bevy_pbr::pbr_ambient

#import bevy_pbr::prepass_utils

//struct StandardMaterial {
//    base_color: vec4<f32>,
//    emissive: vec4<f32>,
//    perceptual_roughness: f32,
//    metallic: f32,
//    reflectance: f32,
//    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
//    flags: u32,
//    alpha_cutoff: f32,
//    parallax_depth_scale: f32,
//    max_parallax_layer_count: f32,
//    max_relief_mapping_search_steps: u32,
//};

//struct PbrInput {
//    material: StandardMaterial,
//    occlusion: f32,
//    frag_coord: vec4<f32>,
//    world_position: vec4<f32>,
//    // Normalized world normal used for shadow mapping as normal-mapping is not used for shadow
//    // mapping
//    world_normal: vec3<f32>,
//    // Normalized normal-mapped world normal used for lighting
//    N: vec3<f32>,
//    // Normalized view vector in world space, pointing from the fragment world position toward the
//    // view world position
//    V: vec3<f32>,
//    is_orthographic: bool,
//    flags: u32,
//};

// ---------------------------
// from https://github.com/DGriffin91/bevy_coordinate_systems/blob/main/src/transformations.wgsl
// ---------------------------

/// Convert a ndc space position to world space
fn position_ndc_to_world(ndc_pos: vec3<f32>) -> vec3<f32> {
    let world_pos = view.inverse_view_proj * vec4(ndc_pos, 1.0);
    return world_pos.xyz / world_pos.w;
}

/// Convert ndc space xy coordinate [-1.0 .. 1.0] to uv [0.0 .. 1.0]
fn ndc_to_uv(ndc: vec2<f32>) -> vec2<f32> {
    return ndc * vec2(0.5, -0.5) + vec2(0.5);
}

/// Convert uv [0.0 .. 1.0] coordinate to ndc space xy [-1.0 .. 1.0]
fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return (uv - vec2(0.5)) * vec2(2.0, -2.0);
}

/// returns the (0.0, 0.0) .. (1.0, 1.0) position within the viewport for the current render target
/// [0 .. render target viewport size] eg. [(0.0, 0.0) .. (1280.0, 720.0)] to [(0.0, 0.0) .. (1.0, 1.0)]
fn frag_coord_to_uv(frag_coord: vec2<f32>) -> vec2<f32> {
    return (frag_coord - view.viewport.xy) / view.viewport.zw;
}

/// Convert frag coord to ndc
fn frag_coord_to_ndc(frag_coord: vec4<f32>) -> vec3<f32> {
    return vec3(uv_to_ndc(frag_coord_to_uv(frag_coord.xy)), frag_coord.z);
}

// ---------------------------
// ---------------------------
// ---------------------------

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let depth = prepass_depth(in.position, 0u);
    let frag_coord = vec4(in.position.xy, depth, 0.0);

    let world_position = position_ndc_to_world(frag_coord_to_ndc(frag_coord));
    
    let is_orthographic = view.projection[3].w == 1.0;
    
    let V = calculate_view(vec4(world_position, 0.0), is_orthographic);

    let deferred_data = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);

    let base_color = unpack4x8unorm(deferred_data.r);
    let emissive = unpack4x8unorm(deferred_data.g);
    let misc = unpack4x8unorm(deferred_data.b);
    let metallic = misc.r;
    let perceptual_roughness = misc.g;
    let occlusion = misc.b;
    let reflectance = misc.a;


    var pbr_input = pbr_input_new();

    pbr_input.material.base_color = base_color;
    pbr_input.material.emissive = emissive;
    pbr_input.material.perceptual_roughness = perceptual_roughness;
    pbr_input.material.metallic = metallic;
    pbr_input.material.reflectance = reflectance;
    //pbr_input.material.flags = 
    pbr_input.material.alpha_cutoff = 0.5;
    pbr_input.material.parallax_depth_scale = 0.1; //default
    pbr_input.material.max_parallax_layer_count = 16.0; //default
    pbr_input.material.max_relief_mapping_search_steps = 5u; //default

    pbr_input.frag_coord = frag_coord;
    pbr_input.world_normal = prepass_normal(frag_coord, 0u);
    pbr_input.world_position = vec4(world_position, 0.0);
    pbr_input.N = pbr_input.world_normal;
    pbr_input.V = V;
    pbr_input.is_orthographic = is_orthographic;

    
    var output_color = pbr(pbr_input);

#ifdef TONEMAP_IN_SHADER
    output_color = tone_mapping(output_color);
#ifdef DEBAND_DITHER
    var output_rgb = output_color.rgb;
    output_rgb = powsafe(output_rgb, 1.0 / 2.2);
    output_rgb = output_rgb + screen_space_dither(frag_coord.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb, 2.2);
    output_color = vec4(output_rgb, output_color.a);
#endif
#endif
#ifdef PREMULTIPLY_ALPHA
    output_color = premultiply_alpha(material.flags, output_color);
#endif


    return output_color;//textureSample(screen_texture, texture_sampler, in.uv);
}

