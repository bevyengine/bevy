#define_import_path bevy_pbr::pbr_deferred_functions

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


// Creates the deferred gbuffer from a PbrInput
fn deferred_gbuffer_from_pbr_input(in: PbrInput, depth: f32) -> vec4<u32> {
#ifdef WEBGL // More crunched for webgl so we can fit also depth
    var props = pack_unorm3x4_plus_unorm_20(vec4(
        in.material.reflectance,
        in.material.metallic,
        in.occlusion, 
        depth));
#else
    var props = pack_unorm4x8(vec4(
        in.material.reflectance, // could be fewer bits
        in.material.metallic, // could be fewer bits
        in.occlusion, // is this usually included / worth including?
        0.0)); // spare
#endif //WEBGL
    let flags = deferred_flags_from_mesh_mat_flags(in.flags, in.material.flags);
    let oct_nor = octa_encode(normalize(in.N));
    var base_color_srgb = vec3(0.0);
    var emissive = in.material.emissive.rgb;
    if ((in.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u) {
        // Material is unlit, use emissive component of gbuffer for color data
        // unlit materials are effectively emissive
        emissive = in.material.base_color.rgb;
    } else {
        base_color_srgb = pow(in.material.base_color.rgb, vec3(1.0 / 2.2));
    }
    let deferred = vec4(
        pack_unorm4x8(vec4(base_color_srgb, in.material.perceptual_roughness)),
        float3_to_rgb9e5(emissive),
        props,
        pack_24bit_nor_and_flags(oct_nor, flags),
    );
    return deferred;
}

// Creates a PbrInput from the deferred gbuffer
fn pbr_input_from_deferred_gbuffer(frag_coord: vec4<f32>, gbuffer: vec4<u32>) -> PbrInput {
    let flags = unpack_flags(gbuffer.a);
    let deferred_flags = mesh_mat_flags_from_deferred_flags(flags);
    let mesh_flags = deferred_flags.x;
    let mat_flags = deferred_flags.y;

    let base_rough = unpack_unorm4x8(gbuffer.r);
    var base_color = pow(base_rough.rgb, vec3(2.2));
    let perceptual_roughness = base_rough.a;
    var emissive = rgb9e5_to_float3(gbuffer.g);
    if ((mat_flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u) {
        base_color = emissive;
        emissive = vec3(0.0);
    }
#ifdef WEBGL // More crunched for webgl so we can fit also depth
    let props = unpack_unorm3x4_plus_unorm_20(gbuffer.b);
    // bias to 0.5 since that's the value for almost all materials
    let reflectance = saturate(props.r - 0.03333333333); 
#else
    let props = unpack_unorm4x8(gbuffer.b);
    let reflectance = props.r;
#endif //WEBGL
    let metallic = props.g;
    let occlusion = props.b;
    let oct_nor = unpack_24bit_nor(gbuffer.a);
    let N = octa_decode(oct_nor);

    let world_position = vec4(position_ndc_to_world(frag_coord_to_ndc(frag_coord)), 1.0);
    let is_orthographic = view.projection[3].w == 1.0;
    let V = calculate_view(world_position, is_orthographic);
    
    var pbr_input: PbrInput;

    pbr_input.material = standard_material_new();

    pbr_input.flags = mesh_flags;
    pbr_input.occlusion = occlusion;

    pbr_input.material.base_color = vec4(base_color, 1.0);
    pbr_input.material.emissive = vec4(emissive, 1.0);
    pbr_input.material.perceptual_roughness = perceptual_roughness;
    pbr_input.material.metallic = metallic;
    pbr_input.material.reflectance = reflectance;
    pbr_input.material.flags = mat_flags;

    pbr_input.frag_coord = frag_coord;
    pbr_input.world_normal = N; 
    pbr_input.world_position = world_position;
    pbr_input.N = N;
    pbr_input.V = V;
    pbr_input.is_orthographic = is_orthographic;

    return pbr_input;
}


