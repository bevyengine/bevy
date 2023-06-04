
#import bevy_core_pipeline::fullscreen_vertex_shader
#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings

#import bevy_pbr::prepass_utils
#import bevy_pbr::pbr_types
#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::shadows
#import bevy_pbr::fog
#import bevy_pbr::pbr_functions
#import bevy_pbr::pbr_deferred_types
#import bevy_pbr::pbr_deferred_functions
#import bevy_pbr::pbr_ambient


@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var frag_coord = vec4(in.position.xy, 0.0, 0.0);

    let deferred_data = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    // needed if using webgl:
    frag_coord.z = unpack_unorm1x16_from_end(deferred_data.b);
    // works otherwise:
    // frag_coord.z = prepass_depth(in.position, 0u);

    var pbr_input = pbr_input_from_deferred_gbuffer(frag_coord, deferred_data);
    
    var output_color = pbr(pbr_input);

    // fog
    if (fog.mode != FOG_MODE_OFF && (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT) != 0u) {
        output_color = apply_fog(fog, output_color, pbr_input.world_position.xyz, view.world_position.xyz);
    }

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

    return output_color;
}

