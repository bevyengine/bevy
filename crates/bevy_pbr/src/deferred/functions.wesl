import package::{
    render::{
        pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_UNLIT_BIT},
        pbr_functions,
        pbr_lighting as lighting,
        rgb9e5,
        mesh_view_bindings::view,
        view_transformations::{position_ndc_to_world, frag_coord_to_ndc},
    },
    deferred::types as deferred_types,
    prepass::io::FragmentOutput,
};
import bevy_render::utils::{octahedral_encode, octahedral_decode};

@if(DEFERRED_SPECULAR_TINT)
    import package::render::mesh_view_bindings::deferred_specular_tint_texture;

@if(MESHLET_MESH_MATERIAL_PASS) import package::meshlet::visibility_buffer_resolve::VertexOutput;
@else import package::prepass::io::VertexOutput;

@if(MOTION_VECTOR_PREPASS)
    import package::render::pbr_prepass_functions::calculate_motion_vector;

/// Creates the deferred gbuffer from a PbrInput.
fn deferred_gbuffer_from_pbr_input(in: PbrInput) -> vec4<u32> {
    // Only monochrome occlusion supported. May not be worth including at all.
    // Some models have baked occlusion, GLTF only supports monochrome.
    // Real time occlusion is applied in the deferred lighting pass.
    // Deriving luminance via Rec. 709. coefficients
    // https://en.wikipedia.org/wiki/Rec._709
    let rec_709_coeffs = vec3<f32>(0.2126, 0.7152, 0.0722);
    let diffuse_occlusion = dot(in.diffuse_occlusion, rec_709_coeffs);
    @if(WEBGL2) // More crunched for webgl so we can also fit depth.
    let ior_specular = deferred_types::pack_4bit_ior_specular(in.material.ior, in.material.specular_weight);
    @if(WEBGL2)
    let ior_specular_props = f32(ior_specular) / 15.0;
    @if(WEBGL2)
    var props = deferred_types::pack_unorm3x4_plus_unorm_20_(vec4(
        ior_specular_props,
        in.material.metallic,
        diffuse_occlusion,
        in.frag_coord.z));
    @if(!WEBGL2)
    let ior_specular = deferred_types::pack_8bit_ior_specular(in.material.ior, in.material.specular_weight);
    @if(!WEBGL2)
    let ior_specular_props = f32(ior_specular) / 255.0;
    @if(!WEBGL2)
    let clearcoat = u32(round(saturate(in.material.clearcoat) * 15.0));
    @if(!WEBGL2)
    let clearcoat_perceptual_roughness =
        u32(round(clamp(in.material.clearcoat_perceptual_roughness, 0.0, 0.5) * 30.0));
    @if(!WEBGL2)
    let clearcoat_props = f32(clearcoat | (clearcoat_perceptual_roughness << 4u)) / 255.0;
    @if(!WEBGL2)
    var props = deferred_types::pack_unorm4x8_(vec4(
        ior_specular_props,
        in.material.metallic, // could be fewer bits
        diffuse_occlusion, // is this worth including?
        clearcoat_props));
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

    // Utilize the emissive channel to transmit the lightmap data. To ensure
    // it matches the output in forward shading, pre-multiply it with the
    // reflectance that `apply_pbr_lighting` applies to lightmap light.
@if(LIGHTMAP) {
    let base_color = in.material.base_color.rgb;
    let metallic = in.material.metallic;
    let specular_transmission = in.material.specular_transmission;
    let diffuse_transmission = in.material.diffuse_transmission;
    let diffuse_color = pbr_functions::calculate_diffuse_color(
        base_color,
        metallic,
        specular_transmission,
        diffuse_transmission
    );
    @if(STANDARD_MATERIAL_CLEARCOAT)
    let reflectance_ior =
        pbr_functions::clearcoat_base_layer_ior(in.material.ior, in.material.clearcoat);
    @else
    let reflectance_ior = in.material.ior;
    let F0_dielectric =
        pbr_functions::calculate_F0_dielectric(reflectance_ior, in.material.specular_tint);
    let NdotV = max(dot(in.N, in.V), 0.0001);
    let F_ab = lighting::F_AB(in.material.perceptual_roughness, NdotV);
    let indirect_diffuse_reflectance = lighting::indirect_diffuse_reflectance(
        diffuse_color,
        metallic,
        F0_dielectric,
        base_color,
        F_ab,
        saturate(in.material.specular_weight),
    );
    emissive += in.lightmap_light * indirect_diffuse_reflectance * view.exposure;
}

    let deferred = vec4(
        deferred_types::pack_unorm4x8_(vec4(base_color_srgb, in.material.perceptual_roughness)),
        rgb9e5::vec3_to_rgb9e5_(emissive),
        props,
        deferred_types::pack_24bit_normal_and_flags(octahedral_normal, flags),
    );
    return deferred;
}

/// Creates a PbrInput from the deferred gbuffer.
///
/// With the `DEFERRED_SPECULAR_TINT` shader def, the specular tint is loaded from
/// `deferred_specular_tint_texture` at `frag_coord.xy`. Without it, the tint is white.
fn pbr_input_from_deferred_gbuffer(frag_coord: vec4<f32>, gbuffer: vec4<u32>) -> PbrInput {
    var pbr = pbr_input_new();

    @if(DEFERRED_SPECULAR_TINT)
    pbr.material.specular_tint = deferred_types::unpack_specular_tint(
        textureLoad(deferred_specular_tint_texture, vec2<i32>(frag_coord.xy), 0).r);

    let flags = deferred_types::unpack_flags(gbuffer.a);
    let deferred_flags = deferred_types::mesh_material_flags_from_deferred_flags(flags);
    pbr.flags = deferred_flags.x;
    pbr.material.flags = deferred_flags.y;

    let base_rough = deferred_types::unpack_unorm4x8_(gbuffer.r);
    pbr.material.perceptual_roughness = base_rough.a;
    let emissive = rgb9e5::rgb9e5_to_vec3_(gbuffer.g);
    if ((pbr.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u) {
        pbr.material.base_color = vec4(emissive, 1.0);
        pbr.material.emissive = vec4(vec3(0.0), 0.0);
    } else {
        pbr.material.base_color = vec4(pow(base_rough.rgb, vec3(2.2)), 1.0);
        pbr.material.emissive = vec4(emissive, 0.0);
    }
    @if(WEBGL2) // More crunched for webgl so we can also fit depth.
    let props = deferred_types::unpack_unorm3x4_plus_unorm_20_(gbuffer.b);
    @if(WEBGL2)
    let ior_specular_props = u32(round(props.r * 15.0));
    @if(WEBGL2)
    let ior_specular = deferred_types::unpack_4bit_ior_specular(ior_specular_props);
    @if(!WEBGL2)
    let props = deferred_types::unpack_unorm4x8_(gbuffer.b);
    @if(!WEBGL2)
    let ior_specular_props = u32(round(props.r * 255.0));
    @if(!WEBGL2)
    let ior_specular = deferred_types::unpack_8bit_ior_specular(ior_specular_props);
    @if(!WEBGL2) {
    let clearcoat_props = u32(round(props.a * 255.0));
    pbr.material.clearcoat = f32(clearcoat_props & 0x0fu) / 15.0;
    pbr.material.clearcoat_perceptual_roughness = f32(clearcoat_props >> 4u) / 30.0;
    }
    pbr.material.ior = ior_specular.x;
    pbr.material.specular_weight = ior_specular.y;
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
    pbr.clearcoat_N = N;
    pbr.is_orthographic = is_orthographic;

    return pbr;
}

@if(PREPASS_PIPELINE)
fn deferred_output(in: VertexOutput, pbr_input: PbrInput) -> FragmentOutput {
    var out: FragmentOutput;

    // gbuffer
    out.deferred = deferred_gbuffer_from_pbr_input(pbr_input);
    // lighting pass id (used to determine which lighting shader to run for the fragment)
    out.deferred_lighting_pass_id = pbr_input.material.deferred_lighting_pass_id;
    // specular tint
    @if(DEFERRED_SPECULAR_TINT)
    out.deferred_specular_tint = deferred_types::pack_specular_tint(pbr_input.material.specular_tint);
    // normal if required
    @if(NORMAL_PREPASS)
    out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
    // motion vectors if required
    @if(MOTION_VECTOR_PREPASS && MESHLET_MESH_MATERIAL_PASS)
    out.motion_vector = in.motion_vector;
    @if(MOTION_VECTOR_PREPASS && !MESHLET_MESH_MATERIAL_PASS)
    out.motion_vector = calculate_motion_vector(in.world_position, in.previous_world_position);

    return out;
}
