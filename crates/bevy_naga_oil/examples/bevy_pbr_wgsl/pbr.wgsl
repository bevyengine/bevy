#import bevy_pbr::mesh_vertex_output as OutputTypes
#import bevy_pbr::pbr_functions as PbrCore
#import bevy_pbr::pbr_bindings as MaterialBindings
#import bevy_pbr::pbr_types as PbrTypes
#import bevy_pbr::mesh_view_bindings as ViewBindings

@fragment
fn fragment(
    mesh: OutputTypes::MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = MaterialBindings::material.base_color;

#ifdef VERTEX_COLORS
    output_color = output_color * mesh.color;
#endif
#ifdef VERTEX_UVS
    if ((MaterialBindings::material.flags & PbrTypes::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSample(MaterialBindings::base_color_texture, MaterialBindings::base_color_sampler, mesh.uv);
    }
#endif

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((MaterialBindings::material.flags & PbrTypes::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
        // the material members
        var pbr_input: PbrCore::PbrInput;

        pbr_input.material.base_color = output_color;
        pbr_input.material.reflectance = MaterialBindings::material.reflectance;
        pbr_input.material.flags = MaterialBindings::material.flags;
        pbr_input.material.alpha_cutoff = MaterialBindings::material.alpha_cutoff;

        // TODO use .a for exposure compensation in HDR
        var emissive: vec4<f32> = MaterialBindings::material.emissive;
#ifdef VERTEX_UVS
        if ((MaterialBindings::material.flags & PbrTypes::STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * textureSample(MaterialBindings::emissive_texture, MaterialBindings::emissive_sampler, mesh.uv).rgb, 1.0);
        }
#endif
        pbr_input.material.emissive = emissive;

        var metallic: f32 = MaterialBindings::material.metallic;
        var perceptual_roughness: f32 = MaterialBindings::material.perceptual_roughness;
#ifdef VERTEX_UVS
        if ((MaterialBindings::material.flags & PbrTypes::STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSample(MaterialBindings::metallic_roughness_texture, MaterialBindings::metallic_roughness_sampler, mesh.uv);
            // Sampling from GLTF standard channels for now
            metallic = metallic * metallic_roughness.b;
            perceptual_roughness = perceptual_roughness * metallic_roughness.g;
        }
#endif
        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        var occlusion: f32 = 1.0;
#ifdef VERTEX_UVS
        if ((MaterialBindings::material.flags & PbrTypes::STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            occlusion = textureSample(MaterialBindings::occlusion_texture, MaterialBindings::occlusion_sampler, mesh.uv).r;
        }
#endif
        pbr_input.occlusion = occlusion;

        pbr_input.frag_coord = frag_coord;
        pbr_input.world_position = mesh.world_position;
        pbr_input.world_normal = mesh.world_normal;

        pbr_input.is_orthographic = ViewBindings::view.projection[3].w == 1.0;

        pbr_input.N = PbrCore::prepare_normal(
            MaterialBindings::material.flags,
            mesh.world_normal,
#ifdef VERTEX_TANGENTS
    #ifdef STANDARDMATERIAL_NORMAL_MAP
            mesh.world_tangent,
    #endif
#endif
#ifdef VERTEX_UVS
            mesh.uv,
#endif
            is_front,
        );
        pbr_input.V = PbrCore::calculate_view(mesh.world_position, pbr_input.is_orthographic);

        output_color = PbrCore::tone_mapping(PbrCore::pbr(pbr_input));
    }

    return output_color;
}
