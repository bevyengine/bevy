
#import bevy_core_pipeline::fullscreen_vertex_shader

#import bevy_pbr::prepass_utils
#import bevy_pbr::pbr_types STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT, STANDARD_MATERIAL_FLAGS_UNLIT_BIT
#import bevy_pbr::pbr_functions as pbr_functions
#import bevy_pbr::pbr_deferred_types as deft
#import bevy_pbr::pbr_deferred_functions pbr_input_from_deferred_gbuffer, unpack_unorm3x4_plus_unorm_20_
#import bevy_pbr::mesh_view_types FOG_MODE_OFF

#import bevy_pbr::mesh_view_bindings deferred_prepass_texture, fog, view, screen_space_ambient_occlusion_texture
#import bevy_core_pipeline::fullscreen_vertex_shader FullscreenVertexOutput
#import bevy_core_pipeline::tonemapping screen_space_dither, powsafe, tone_mapping

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::gtao_utils gtao_multibounce
#endif


@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var frag_coord = vec4(in.position.xy, 0.0, 0.0);

    let deferred_data = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);

#ifdef WEBGL2
    frag_coord.z = deft::unpack_unorm3x4_plus_unorm_20_(deferred_data.b).w;
#else
    frag_coord.z = bevy_pbr::prepass_utils::prepass_depth(in.position, 0u);
#endif

    var pbr_input = pbr_input_from_deferred_gbuffer(frag_coord, deferred_data);
    var output_color = vec4(0.0);

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
        let ssao = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0i).r;
        let ssao_multibounce = gtao_multibounce(ssao, pbr_input.material.base_color.rgb);
        pbr_input.occlusion = min(pbr_input.occlusion, ssao_multibounce);
#endif // SCREEN_SPACE_AMBIENT_OCCLUSION
    
    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        output_color = pbr_functions::pbr(pbr_input);
    } else {
        output_color = pbr_input.material.base_color;
    }

    // fog
    if (fog.mode != FOG_MODE_OFF && (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT) != 0u) {
        output_color = pbr_functions::apply_fog(fog, output_color, pbr_input.world_position.xyz, view.world_position.xyz);
    }

#ifdef TONEMAP_IN_SHADER
    output_color = tone_mapping(output_color, view.color_grading);
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
    output_color = pbr_functions::premultiply_alpha(material.flags, output_color);
#endif

    return output_color;
}

