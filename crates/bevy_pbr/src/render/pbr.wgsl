#define_import_path bevy_pbr::fragment

#import bevy_render::core_bindings
#import bevy_pbr::mesh_vertex_output
#import bevy_pbr::pbr_functions as pbr_functions
#import bevy_pbr::pbr_bindings as pbr_bindings
#import bevy_pbr::pbr_types as pbr_types
#import bevy_pbr::mesh_view_bindings
// load user-defined function overrides
#import bevy_pbr::user_overrides

@fragment
fn fragment(
    mesh: bevy_pbr::mesh_vertex_output::MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = pbr_bindings::material.base_color;

#ifdef VERTEX_COLORS
    output_color = output_color * mesh.color;
#endif
#ifdef VERTEX_UVS
    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSample(pbr_bindings::base_color_texture, pbr_bindings::base_color_sampler, mesh.uv);
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
            emissive = vec4<f32>(emissive.rgb * textureSample(pbr_bindings::emissive_texture, pbr_bindings::emissive_sampler, mesh.uv).rgb, 1.0);
        }
#endif
        pbr_input.material.emissive = emissive;

        var metallic: f32 = pbr_bindings::material.metallic;
        var perceptual_roughness: f32 = pbr_bindings::material.perceptual_roughness;
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSample(pbr_bindings::metallic_roughness_texture, pbr_bindings::metallic_roughness_sampler, mesh.uv);
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
            occlusion = textureSample(pbr_bindings::occlusion_texture, pbr_bindings::occlusion_sampler, mesh.uv).r;
        }
#endif
        pbr_input.occlusion = occlusion;

        pbr_input.frag_coord = mesh.clip_position;
        pbr_input.world_position = mesh.world_position;
        pbr_input.world_normal = pbr_functions::prepare_world_normal(
            mesh.world_normal,
            (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
            is_front,
        );

        pbr_input.is_orthographic = bevy_render::core_bindings::view.projection[3].w == 1.0;

        pbr_input.N = pbr_functions::apply_normal_mapping(
            pbr_bindings::material.flags,
            pbr_input.world_normal,
#ifdef VERTEX_TANGENTS
    #ifdef STANDARDMATERIAL_NORMAL_MAP
            mesh.world_tangent,
    #endif
#endif
#ifdef VERTEX_UVS
            mesh.uv,
#endif
        );
        pbr_input.V = pbr_functions::calculate_view(mesh.world_position, pbr_input.is_orthographic);
        output_color = pbr_functions::pbr(pbr_input);
    } else {
        output_color = pbr_functions::alpha_discard(pbr_bindings::material, output_color);
    }

#ifdef TONEMAP_IN_SHADER
        output_color = pbr_functions::tone_mapping(output_color);
#endif
#ifdef DEBAND_DITHER
        output_color = pbr_functions::dither(output_color, mesh.clip_position.xy);
#endif
    return output_color;
}
