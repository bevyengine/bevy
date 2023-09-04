#define_import_path bevy_pbr::pbr_deferred_functions
#import bevy_pbr::pbr_types PbrInput, standard_material_new, STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT, STANDARD_MATERIAL_FLAGS_UNLIT_BIT
#import bevy_pbr::pbr_deferred_types as deft
#import bevy_pbr::pbr_functions as pbr_functions
#import bevy_pbr::rgb9e5 as rgb9e5
#import bevy_pbr::mesh_view_bindings as view_bindings

//TODO Griffin This doesn't seem to work
//#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS 
//#import bevy_pbr::prepass_bindings view
//#else
//#import bevy_pbr::mesh_view_bindings view
//#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

// TODO Griffin using mesh_view_bindings here because of:
//  ┌─ bevy_pbr\src\prepass\prepass_bindings.wgsl:7:1
//  │
//7 │ var<uniform> view: bevy_render::view::View;
//  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^ naga::GlobalVariable [36]
//  │
//  = Bindings for [36] conflict with other resource
#import bevy_pbr::mesh_view_bindings view

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

/// Returns the (0.0, 0.0) .. (1.0, 1.0) position within the viewport for the current render target.
/// [0 .. render target viewport size] eg. [(0.0, 0.0) .. (1280.0, 720.0)] to [(0.0, 0.0) .. (1.0, 1.0)]
fn frag_coord_to_uv(frag_coord: vec2<f32>) -> vec2<f32> {
    return (frag_coord - view.viewport.xy) / view.viewport.zw;
}

/// Convert frag coord to ndc.
fn frag_coord_to_ndc(frag_coord: vec4<f32>) -> vec3<f32> {
    return vec3(uv_to_ndc(frag_coord_to_uv(frag_coord.xy)), frag_coord.z);
}

// ---------------------------
// ---------------------------
// ---------------------------


// Creates the deferred gbuffer from a PbrInput.
fn deferred_gbuffer_from_pbr_input(in: PbrInput) -> vec4<u32> {
     // Only monochrome occlusion supported. May not be worth including at all.
     // Some models have baked occlusion, GLTF only supports monochrome. 
     // Real time occlusion is applied in the deferred lighting pass.
    let occlusion = dot(in.occlusion, vec3<f32>(0.2126, 0.7152, 0.0722));
#ifdef WEBGL2 // More crunched for webgl so we can also fit depth.
    var props = deft::pack_unorm3x4_plus_unorm_20_(vec4(
        in.material.reflectance,
        in.material.metallic,
        occlusion, 
        in.frag_coord.z));
#else
    var props = deft::pack_unorm4x8_(vec4(
        in.material.reflectance, // could be fewer bits
        in.material.metallic, // could be fewer bits
        occlusion, // is this worth including?
        0.0)); // spare
#endif // WEBGL
    let flags = deft::deferred_flags_from_mesh_material_flags(in.flags, in.material.flags);
    let oct_nor = deft::octa_encode(normalize(in.N));
    var base_color_srgb = vec3(0.0);
    var emissive = in.material.emissive.rgb;
    if ((in.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u) {
        // Material is unlit, use emissive component of gbuffer for color data.
        // Unlit materials are effectively emissive.
        emissive = in.material.base_color.rgb;
    } else {
        base_color_srgb = pow(in.material.base_color.rgb, vec3(1.0 / 2.2));
    }
    let deferred = vec4(
        deft::pack_unorm4x8_(vec4(base_color_srgb, in.material.perceptual_roughness)),
        rgb9e5::vec3_to_rgb9e5_(emissive),
        props,
        deft::pack_24bit_nor_and_flags(oct_nor, flags),
    );
    return deferred;
}

// Creates a PbrInput from the deferred gbuffer.
fn pbr_input_from_deferred_gbuffer(frag_coord: vec4<f32>, gbuffer: vec4<u32>) -> PbrInput {
    var pbr: PbrInput;
    pbr.material = standard_material_new();

    let flags = deft::unpack_flags(gbuffer.a);
    let deferred_flags = deft::mesh_material_flags_from_deferred_flags(flags);
    pbr.flags = deferred_flags.x;
    pbr.material.flags = deferred_flags.y;

    let base_rough = deft::unpack_unorm4x8_(gbuffer.r);
    pbr.material.perceptual_roughness = base_rough.a;
    let emissive = rgb9e5::rgb9e5_to_vec3_(gbuffer.g);
    if ((pbr.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u) {
        pbr.material.base_color = vec4(emissive, 1.0);
        pbr.material.emissive = vec4(vec3(0.0), 1.0);
    } else {
        pbr.material.base_color = vec4(pow(base_rough.rgb, vec3(2.2)), 1.0);
        pbr.material.emissive = vec4(emissive, 1.0);
    }
#ifdef WEBGL2 // More crunched for webgl so we can also fit depth.
    let props = deft::unpack_unorm3x4_plus_unorm_20_(gbuffer.b);
    // Bias to 0.5 since that's the value for almost all materials.
    pbr.material.reflectance = saturate(props.r - 0.03333333333); 
#else
    let props = deft::unpack_unorm4x8_(gbuffer.b);
    pbr.material.reflectance = props.r;
#endif // WEBGL
    pbr.material.metallic = props.g;
    pbr.occlusion = vec3(props.b);
    let oct_nor = deft::unpack_24bit_nor(gbuffer.a);
    let N = deft::octa_decode(oct_nor);

    let world_position = vec4(position_ndc_to_world(frag_coord_to_ndc(frag_coord)), 1.0);
    let is_orthographic = view.projection[3].w == 1.0;
    let V = pbr_functions::calculate_view(world_position, is_orthographic);
    
    pbr.frag_coord = frag_coord;
    pbr.world_normal = N; 
    pbr.world_position = world_position;
    pbr.N = N;
    pbr.V = V;
    pbr.is_orthographic = is_orthographic;

    return pbr;
}


