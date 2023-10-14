#import bevy_pbr::prepass_utils
#import bevy_pbr::pbr_types STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT, STANDARD_MATERIAL_FLAGS_UNLIT_BIT
#import bevy_pbr::pbr_functions as pbr_functions
#import bevy_pbr::pbr_deferred_types as deferred_types
#import bevy_pbr::pbr_deferred_functions pbr_input_from_deferred_gbuffer, unpack_unorm3x4_plus_unorm_20_
#import bevy_pbr::mesh_view_types FOG_MODE_OFF

#import bevy_pbr::mesh_view_bindings deferred_prepass_texture, fog, view, screen_space_ambient_occlusion_texture
#import bevy_core_pipeline::tonemapping screen_space_dither, powsafe, tone_mapping

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::gtao_utils gtao_multibounce
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
    frag_coord.z = deferred_types::unpack_unorm3x4_plus_unorm_20_(deferred_data.b).w;
#else
    frag_coord.z = bevy_pbr::prepass_utils::prepass_depth(in.position, 0u);
#endif

    var pbr_input = pbr_input_from_deferred_gbuffer(frag_coord, deferred_data);
    var output_color = vec4(0.0);

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
        let ssao = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0i).r;
        let ssao_multibounce = gtao_multibounce(ssao, pbr_input.material.base_color.rgb);
        pbr_input.occlusion = min(pbr_input.occlusion, ssao_multibounce);
#endif // SCREEN_SPACE_AMBIENT_OCCLUSION

        output_color = pbr_functions::apply_pbr_lighting(pbr_input);
    } else {
        output_color = pbr_input.material.base_color;
    }

    output_color = pbr_functions::main_pass_post_lighting_processing(pbr_input, output_color);

    return output_color;
}

