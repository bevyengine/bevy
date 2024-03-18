#import bevy_pbr::{
    prepass_utils,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    pbr_functions,
    pbr_deferred_functions::pbr_input_from_deferred_gbuffer,
    pbr_deferred_types::unpack_unorm3x4_plus_unorm_20_,
    lighting,
    mesh_view_bindings::deferred_prepass_texture,
}

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::mesh_view_bindings::screen_space_ambient_occlusion_texture
#import bevy_pbr::gtao_utils::gtao_multibounce
#endif

struct FullscreenVertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
};

struct PbrDeferredLightingDepthId {
    depth_id: u32, // limited to u8
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding_0: f32,
    _webgl2_padding_1: f32,
    _webgl2_padding_2: f32,
#endif
}
@group(1) @binding(0)
var<uniform> depth_id: PbrDeferredLightingDepthId;

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    // See the full screen vertex shader for explanation above for how this works.
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    // Depth is stored as unorm, so we are dividing the u8 depth_id by 255.0 here.
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), f32(depth_id.depth_id) / 255.0, 1.0);

    return FullscreenVertexOutput(clip_position, uv);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var frag_coord = vec4(in.position.xy, 0.0, 0.0);

    let deferred_data = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);

#ifdef WEBGL2
    frag_coord.z = unpack_unorm3x4_plus_unorm_20_(deferred_data.b).w;
#else
#ifdef DEPTH_PREPASS
    frag_coord.z = prepass_utils::prepass_depth(in.position, 0u);
#endif
#endif

    var pbr_input = pbr_input_from_deferred_gbuffer(frag_coord, deferred_data);
    var output_color = vec4(0.0);

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
        let ssao = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0i).r;
        let ssao_multibounce = gtao_multibounce(ssao, pbr_input.material.base_color.rgb);
        pbr_input.diffuse_occlusion = min(pbr_input.diffuse_occlusion, ssao_multibounce);

        // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
        let NdotV = max(dot(pbr_input.N, pbr_input.V), 0.0001); 
        var perceptual_roughness: f32 = pbr_input.material.perceptual_roughness;
        let roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);
        // Use SSAO to estimate the specular occlusion.
        // Lagarde and Rousiers 2014, "Moving Frostbite to Physically Based Rendering"
        pbr_input.specular_occlusion =  saturate(pow(NdotV + ssao, exp2(-16.0 * roughness - 1.0)) - 1.0 + ssao);
#endif // SCREEN_SPACE_AMBIENT_OCCLUSION

        output_color = pbr_functions::apply_pbr_lighting(pbr_input);
    } else {
        output_color = pbr_input.material.base_color;
    }

    output_color = pbr_functions::main_pass_post_lighting_processing(pbr_input, output_color);

    return output_color;
}

