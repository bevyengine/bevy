#define_import_path bevy_pbr::fragment

#import bevy_pbr::pbr_functions as pbr_functions
#import bevy_pbr::pbr_bindings as pbr_bindings
#import bevy_pbr::pbr_types as pbr_types
#import bevy_pbr::prepass_utils

#import bevy_pbr::mesh_vertex_output       MeshVertexOutput
#import bevy_pbr::mesh_bindings            mesh
#import bevy_pbr::mesh_view_bindings       view, fog
#import bevy_pbr::mesh_view_types          FOG_MODE_OFF
#import bevy_core_pipeline::tonemapping    screen_space_dither, powsafe, tone_mapping
#import bevy_pbr::parallax_mapping

#import bevy_pbr::prepass_utils

@fragment
fn fragment(
    in: MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = pbr_bindings::material.base_color;

    let is_orthographic = view.projection[3].w == 1.0;
    let V = pbr_functions::calculate_view(in.world_position, is_orthographic);
#ifdef VERTEX_UVS
    var uv = in.uv;
#ifdef VERTEX_TANGENTS
    if ((material.flags & STANDARD_MATERIAL_FLAGS_DEPTH_MAP_BIT) != 0u) {
        let N = in.world_normal;
        let T = in.world_tangent.xyz;
        let B = in.world_tangent.w * cross(N, T);
        // Transform V from fragment to camera in world space to tangent space.
        let Vt = vec3(dot(V, T), dot(V, B), dot(V, N));
        uv = parallaxed_uv(
            material.parallax_depth_scale,
            material.max_parallax_layer_count,
            material.max_relief_mapping_search_steps,
            uv,
            // Flip the direction of Vt to go toward the surface to make the
            // parallax mapping algorithm easier to understand and reason
            // about.
            -Vt,
        );
    }
#endif
#endif

#ifdef VERTEX_COLORS
    output_color = output_color * in.color;
#endif
#ifdef VERTEX_UVS
    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSample(pbr_bindings::base_color_texture, pbr_bindings::base_color_sampler, in.uv);
    }
#endif

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
        // the material members
        var pbr_input: pbr_functions::PbrInput;

        pbr_input.material.base_color = output_color;
        pbr_input.material.reflectance = pbr_bindings::material.reflectance;
        pbr_input.material.flags = pbr_bindings::material.flags;
        pbr_input.material.alpha_cutoff = pbr_bindings::material.alpha_cutoff;

        // TODO use .a for exposure compensation in HDR
        var emissive: vec4<f32> = pbr_bindings::material.emissive;
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * textureSample(pbr_bindings::emissive_texture, pbr_bindings::emissive_sampler, in.uv).rgb, 1.0);
        }
#endif
        pbr_input.material.emissive = emissive;

        var metallic: f32 = pbr_bindings::material.metallic;
        var perceptual_roughness: f32 = pbr_bindings::material.perceptual_roughness;
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSample(pbr_bindings::metallic_roughness_texture, pbr_bindings::metallic_roughness_sampler, in.uv);
            // Sampling from GLTF standard channels for now
            metallic = metallic * metallic_roughness.b;
            perceptual_roughness = perceptual_roughness * metallic_roughness.g;
        }
#endif
        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        var occlusion: f32 = 1.0;
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            occlusion = textureSample(pbr_bindings::occlusion_texture, pbr_bindings::occlusion_sampler, in.uv).r;
        }
#endif
        pbr_input.frag_coord = in.clip_position;
        pbr_input.world_position = in.world_position;

#ifdef LOAD_PREPASS_NORMALS
        pbr_input.world_normal = bevy_pbr::prepass_utils::prepass_normal(in.clip_position, 0u);
#else // LOAD_PREPASS_NORMALS
        pbr_input.world_normal = pbr_functions::prepare_world_normal(
            in.world_normal,
            (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
            is_front,
        );
#endif // LOAD_PREPASS_NORMALS

        pbr_input.is_orthographic = is_orthographic;

        pbr_input.N = pbr_functions::apply_normal_mapping(
            pbr_bindings::material.flags,
            pbr_input.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
            in.world_tangent,
#endif
#endif
#ifdef VERTEX_UVS
            uv,
#endif
        );
        pbr_input.V = V;
        pbr_input.occlusion = occlusion;

        pbr_input.flags = mesh.flags;

        output_color = pbr_functions::pbr(pbr_input);
    } else {
        output_color = pbr_functions::alpha_discard(pbr_bindings::material, output_color);
    }

    // fog
    if (fog.mode != FOG_MODE_OFF && (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT) != 0u) {
        output_color = pbr_functions::apply_fog(output_color, in.world_position.xyz, view.world_position.xyz);
    }

#ifdef TONEMAP_IN_SHADER
    output_color = tone_mapping(output_color, view.color_grading);
#ifdef DEBAND_DITHER
    var output_rgb = output_color.rgb;
    output_rgb = powsafe(output_rgb, 1.0 / 2.2);
    output_rgb = output_rgb + screen_space_dither(in.clip_position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb, 2.2);
    output_color = vec4(output_rgb, output_color.a);
#endif
#endif
#ifdef PREMULTIPLY_ALPHA
    output_color = pbr_functions::premultiply_alpha(pbr_bindings::material.flags, output_color);
#endif
    return output_color;
}
