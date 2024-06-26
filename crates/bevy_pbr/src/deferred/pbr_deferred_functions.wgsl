#define_import_path bevy_pbr::pbr_deferred_functions

#import bevy_pbr::{
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_UNLIT_BIT},
    pbr_deferred_types as deferred_types,
    pbr_functions,
    rgb9e5,
    mesh_view_bindings::view,
    utils::{octahedral_encode, octahedral_decode},
    prepass_io::FragmentOutput,
    view_transformations::{position_ndc_to_world, frag_coord_to_ndc},
}

#ifdef MESHLET_MESH_MATERIAL_PASS
#import bevy_pbr::meshlet_visibility_buffer_resolve::VertexOutput
#else
#import bevy_pbr::prepass_io::VertexOutput
#endif

#ifdef MOTION_VECTOR_PREPASS
    #import bevy_pbr::pbr_prepass_functions::calculate_motion_vector
#endif

// Creates the deferred gbuffer from a PbrInput.
fn deferred_gbuffer_from_pbr_input(in: PbrInput) -> vec4<u32> {
     // Only monochrome occlusion supported. May not be worth including at all.
     // Some models have baked occlusion, GLTF only supports monochrome.
     // Real time occlusion is applied in the deferred lighting pass.
     // Deriving luminance via Rec. 709. coefficients
     // https://en.wikipedia.org/wiki/Rec._709
    let diffuse_occlusion = dot(in.diffuse_occlusion, vec3<f32>(0.2126, 0.7152, 0.0722));
#ifdef WEBGL2 // More crunched for webgl so we can also fit depth.
    var props = deferred_types::pack_unorm3x4_plus_unorm_20_(vec4(
        in.material.reflectance,
        in.material.metallic,
        diffuse_occlusion,
        in.frag_coord.z));
#else
    var props = deferred_types::pack_unorm4x8_(vec4(
        in.material.reflectance, // could be fewer bits
        in.material.metallic, // could be fewer bits
        diffuse_occlusion, // is this worth including?
        0.0)); // spare
#endif // WEBGL2
    let flags = deferred_types::deferred_flags_from_mesh_material_flags(in.flags, in.material.flags);
    let octahedral_normal = octahedral_encode(normalize(in.N));
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
        deferred_types::pack_unorm4x8_(vec4(base_color_srgb, in.material.perceptual_roughness)),
        rgb9e5::vec3_to_rgb9e5_(emissive),
        props,
        deferred_types::pack_24bit_normal_and_flags(octahedral_normal, flags),
    );
    return deferred;
}

// Creates a PbrInput from the deferred gbuffer.
fn pbr_input_from_deferred_gbuffer(frag_coord: vec4<f32>, gbuffer: vec4<u32>) -> PbrInput {
    var pbr = pbr_input_new();

    let flags = deferred_types::unpack_flags(gbuffer.a);
    let deferred_flags = deferred_types::mesh_material_flags_from_deferred_flags(flags);
    pbr.flags = deferred_flags.x;
    pbr.material.flags = deferred_flags.y;

    let base_rough = deferred_types::unpack_unorm4x8_(gbuffer.r);
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
    let props = deferred_types::unpack_unorm3x4_plus_unorm_20_(gbuffer.b);
    // Bias to 0.5 since that's the value for almost all materials.
    pbr.material.reflectance = saturate(props.r - 0.03333333333);
#else
    let props = deferred_types::unpack_unorm4x8_(gbuffer.b);
    pbr.material.reflectance = props.r;
#endif // WEBGL2
    pbr.material.metallic = props.g;
    pbr.diffuse_occlusion = vec3(props.b);
    let octahedral_normal = deferred_types::unpack_24bit_normal(gbuffer.a);
    let N = octahedral_decode(octahedral_normal);

    let world_position = vec4(position_ndc_to_world(frag_coord_to_ndc(frag_coord)), 1.0);
    let is_orthographic = view.clip_from_view[3].w == 1.0;
    let V = pbr_functions::calculate_view(world_position, is_orthographic);

    pbr.frag_coord = frag_coord;
    pbr.world_normal = N;
    pbr.world_position = world_position;
    pbr.N = N;
    pbr.V = V;
    pbr.is_orthographic = is_orthographic;

    return pbr;
}

#ifdef PREPASS_PIPELINE
fn deferred_output(in: VertexOutput, pbr_input: PbrInput) -> FragmentOutput {
    var out: FragmentOutput;

    // gbuffer
    out.deferred = deferred_gbuffer_from_pbr_input(pbr_input);
    // lighting pass id (used to determine which lighting shader to run for the fragment)
    out.deferred_lighting_pass_id = pbr_input.material.deferred_lighting_pass_id;
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
