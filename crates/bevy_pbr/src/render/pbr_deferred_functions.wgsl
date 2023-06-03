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

// Creates a PbrInput with default values
fn pbr_input_from_deferred_gbuffer(frag_coord: vec4<f32>, gbuffer: vec4<u32>) -> PbrInput {
    let base_color = unpack4x8unorm(gbuffer.r).rgb; //spare 8 bits
    let emissive = vec4(rgb9e5_to_float3(gbuffer.g), 1.0);
    let props = unpack4x8unorm(gbuffer.b);
    let metallic = props.r; // could be fewer bits
    let perceptual_roughness = props.g;
    let occlusion = props.b; // is this usually included / worth including?
    let reflectance = props.a; // could be fewer bits
    let deferred_flags = mesh_mat_flags_from_deferred_flags(gbuffer.a); // could be fewer bits
    let mesh_flags = deferred_flags.x;
    let mat_flags = deferred_flags.y;

    let world_position = vec4(position_ndc_to_world(frag_coord_to_ndc(frag_coord)), 1.0);
    let is_orthographic = view.projection[3].w == 1.0;
    let V = calculate_view(world_position, is_orthographic);
    
    var pbr_input: PbrInput;

    pbr_input.material = standard_material_new();

    pbr_input.flags = mesh_flags;
    pbr_input.occlusion = occlusion;

    pbr_input.material.base_color = vec4(base_color, 1.0);
    pbr_input.material.emissive = emissive;
    pbr_input.material.perceptual_roughness = perceptual_roughness;
    pbr_input.material.metallic = metallic;
    pbr_input.material.reflectance = reflectance;
    pbr_input.material.flags = mat_flags;

    pbr_input.frag_coord = frag_coord;
    // TODO Griffin: shouldn't need the normalize here. 
    // Was getting stepping artifacts on the tonemapping example
    pbr_input.world_normal = normalize(prepass_normal(frag_coord, 0u)); 
    pbr_input.world_position = world_position;
    pbr_input.N = pbr_input.world_normal;
    pbr_input.V = V;
    pbr_input.is_orthographic = is_orthographic;

    return pbr_input;
}