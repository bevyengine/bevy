#define_import_path bevy_pbr::pbr_fragment

#import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d}

#import bevy_pbr::{
    pbr_functions,
    pbr_functions::SampleBias,
    pbr_bindings,
    pbr_types,
    prepass_utils,
    lighting,
    mesh_bindings::mesh,
    mesh_view_bindings::view,
    parallax_mapping::parallaxed_uv,
    lightmap::lightmap,
}

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::mesh_view_bindings::screen_space_ambient_occlusion_texture
#import bevy_pbr::ssao_utils::ssao_multibounce
#endif

#ifdef MESHLET_MESH_MATERIAL_PASS
#import bevy_pbr::meshlet_visibility_buffer_resolve::VertexOutput
#else ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::VertexOutput
#else
#import bevy_pbr::forward_io::VertexOutput
#endif

#ifdef BINDLESS
#import bevy_pbr::pbr_bindings::material_indices
#endif  // BINDLESS

// prepare a basic PbrInput from the vertex stage output, mesh binding and view binding
fn pbr_input_from_vertex_output(
    in: VertexOutput,
    is_front: bool,
    double_sided: bool,
) -> pbr_types::PbrInput {
    var pbr_input: pbr_types::PbrInput = pbr_types::pbr_input_new();

#ifdef MESHLET_MESH_MATERIAL_PASS
    pbr_input.flags = in.mesh_flags;
#else
    pbr_input.flags = mesh[in.instance_index].flags;
#endif

    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = pbr_functions::calculate_view(in.world_position, pbr_input.is_orthographic);
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;

#ifdef VERTEX_COLORS
    pbr_input.material.base_color = in.color;
#endif

    pbr_input.world_normal = pbr_functions::prepare_world_normal(
        in.world_normal,
        double_sided,
        is_front,
    );

#ifdef LOAD_PREPASS_NORMALS
    pbr_input.N = prepass_utils::prepass_normal(in.position, 0u);
#else
    pbr_input.N = normalize(pbr_input.world_normal);
#endif

    return pbr_input;
}

// Prepare a full PbrInput by sampling all textures to resolve
// the material members
fn pbr_input_from_standard_material(
    in: VertexOutput,
    is_front: bool,
) -> pbr_types::PbrInput {
#ifdef MESHLET_MESH_MATERIAL_PASS
    let slot = in.material_bind_group_slot;
#else   // MESHLET_MESH_MATERIAL_PASS
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
    let flags = pbr_bindings::material_array[material_indices[slot].material].flags;
    let base_color = pbr_bindings::material_array[material_indices[slot].material].base_color;
    let deferred_lighting_pass_id =
        pbr_bindings::material_array[material_indices[slot].material].deferred_lighting_pass_id;
#else   // BINDLESS
    let flags = pbr_bindings::material.flags;
    let base_color = pbr_bindings::material.base_color;
    let deferred_lighting_pass_id = pbr_bindings::material.deferred_lighting_pass_id;
#endif

    let double_sided = (flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    var pbr_input: pbr_types::PbrInput = pbr_input_from_vertex_output(in, is_front, double_sided);
    pbr_input.material.flags = flags;
    pbr_input.material.base_color *= base_color;
    pbr_input.material.deferred_lighting_pass_id = deferred_lighting_pass_id;

    // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
    let NdotV = max(dot(pbr_input.N, pbr_input.V), 0.0001);

    // Fill in the sample bias so we can sample from textures.
    var bias: SampleBias;
#ifdef MESHLET_MESH_MATERIAL_PASS
    bias.ddx_uv = in.ddx_uv;
    bias.ddy_uv = in.ddy_uv;
#else   // MESHLET_MESH_MATERIAL_PASS
    bias.mip_bias = view.mip_bias;
#endif  // MESHLET_MESH_MATERIAL_PASS

// TODO: Transforming UVs mean we need to apply derivative chain rule for meshlet mesh material pass
#ifdef VERTEX_UVS

#ifdef BINDLESS
    let uv_transform = pbr_bindings::material_array[material_indices[slot].material].uv_transform;
#else   // BINDLESS
    let uv_transform = pbr_bindings::material.uv_transform;
#endif  // BINDLESS

#ifdef VERTEX_UVS_A
    var uv = (uv_transform * vec3(in.uv, 1.0)).xy;
#endif

// TODO: Transforming UVs mean we need to apply derivative chain rule for meshlet mesh material pass
#ifdef VERTEX_UVS_B
    var uv_b = (uv_transform * vec3(in.uv_b, 1.0)).xy;
#else
    var uv_b = uv;
#endif

#ifdef VERTEX_TANGENTS
    if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_DEPTH_MAP_BIT) != 0u) {
        let V = pbr_input.V;
        let TBN = pbr_functions::calculate_tbn_mikktspace(in.world_normal, in.world_tangent);
        let T = TBN[0];
        let B = TBN[1];
        let N = TBN[2];
        // Transform V from fragment to camera in world space to tangent space.
        let Vt = vec3(dot(V, T), dot(V, B), dot(V, N));
#ifdef VERTEX_UVS_A
        // TODO: Transforming UVs mean we need to apply derivative chain rule for meshlet mesh material pass
        uv = parallaxed_uv(
#ifdef BINDLESS
            pbr_bindings::material_array[material_indices[slot].material].parallax_depth_scale,
            pbr_bindings::material_array[material_indices[slot].material].max_parallax_layer_count,
            pbr_bindings::material_array[material_indices[slot].material].max_relief_mapping_search_steps,
#else   // BINDLESS
            pbr_bindings::material.parallax_depth_scale,
            pbr_bindings::material.max_parallax_layer_count,
            pbr_bindings::material.max_relief_mapping_search_steps,
#endif  // BINDLESS
            uv,
            // Flip the direction of Vt to go toward the surface to make the
            // parallax mapping algorithm easier to understand and reason
            // about.
            -Vt,
            slot,
        );
#endif

#ifdef VERTEX_UVS_B
        // TODO: Transforming UVs mean we need to apply derivative chain rule for meshlet mesh material pass
        uv_b = parallaxed_uv(
#ifdef BINDLESS
            pbr_bindings::material_array[material_indices[slot].material].parallax_depth_scale,
            pbr_bindings::material_array[material_indices[slot].material].max_parallax_layer_count,
            pbr_bindings::material_array[material_indices[slot].material].max_relief_mapping_search_steps,
#else   // BINDLESS
            pbr_bindings::material.parallax_depth_scale,
            pbr_bindings::material.max_parallax_layer_count,
            pbr_bindings::material.max_relief_mapping_search_steps,
#endif  // BINDLESS
            uv_b,
            // Flip the direction of Vt to go toward the surface to make the
            // parallax mapping algorithm easier to understand and reason
            // about.
            -Vt,
            slot,
        );
#else
        uv_b = uv;
#endif
    }
#endif // VERTEX_TANGENTS

    if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        pbr_input.material.base_color *=
#ifdef MESHLET_MESH_MATERIAL_PASS
            textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
            textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                bindless_textures_2d[material_indices[slot].base_color_texture],
                bindless_samplers_filtering[material_indices[slot].base_color_sampler],
#else   // BINDLESS
                pbr_bindings::base_color_texture,
                pbr_bindings::base_color_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_BASE_COLOR_UV_B
                uv_b,
#else
                uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                bias.ddx_uv,
                bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
        );

#ifdef ALPHA_TO_COVERAGE
    // Sharpen alpha edges.
    //
    // https://bgolus.medium.com/anti-aliased-alpha-test-the-esoteric-alpha-to-coverage-8b177335ae4f
    let alpha_mode = flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ALPHA_TO_COVERAGE {

#ifdef BINDLESS
        let alpha_cutoff = pbr_bindings::material_array[material_indices[slot].material].alpha_cutoff;
#else   // BINDLESS
        let alpha_cutoff = pbr_bindings::material.alpha_cutoff;
#endif  // BINDLESS

        pbr_input.material.base_color.a = (pbr_input.material.base_color.a - alpha_cutoff) /
                max(fwidth(pbr_input.material.base_color.a), 0.0001) + 0.5;
    }
#endif // ALPHA_TO_COVERAGE

    }
#endif // VERTEX_UVS

    pbr_input.material.flags = flags;

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
#ifdef BINDLESS
        pbr_input.material.ior = pbr_bindings::material_array[material_indices[slot].material].ior;
        pbr_input.material.attenuation_color =
                pbr_bindings::material_array[material_indices[slot].material].attenuation_color;
        pbr_input.material.attenuation_distance =
                pbr_bindings::material_array[material_indices[slot].material].attenuation_distance;
        pbr_input.material.alpha_cutoff =
                pbr_bindings::material_array[material_indices[slot].material].alpha_cutoff;
#else   // BINDLESS
        pbr_input.material.ior = pbr_bindings::material.ior;
        pbr_input.material.attenuation_color = pbr_bindings::material.attenuation_color;
        pbr_input.material.attenuation_distance = pbr_bindings::material.attenuation_distance;
        pbr_input.material.alpha_cutoff = pbr_bindings::material.alpha_cutoff;
#endif  // BINDLESS

        // reflectance
#ifdef BINDLESS
        pbr_input.material.reflectance =
                pbr_bindings::material_array[material_indices[slot].material].reflectance;
#else   // BINDLESS
        pbr_input.material.reflectance = pbr_bindings::material.reflectance;
#endif  // BINDLESS

#ifdef PBR_SPECULAR_TEXTURES_SUPPORTED
#ifdef VERTEX_UVS

        // Specular texture
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_SPECULAR_TEXTURE_BIT) != 0u) {
            let specular =
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                bindless_textures_2d[material_indices[slot].specular_texture],
                bindless_samplers_filtering[material_indices[slot].specular_sampler],
#else   // BINDLESS
                pbr_bindings::specular_texture,
                pbr_bindings::specular_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_SPECULAR_UV_B
                uv_b,
#else   // STANDARD_MATERIAL_SPECULAR_UV_B
                uv,
#endif  // STANDARD_MATERIAL_SPECULAR_UV_B
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
            ).a;
            // This 0.5 factor is from the `KHR_materials_specular` specification:
            // <https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_specular#materials-with-reflectance-parameter>
            pbr_input.material.reflectance *= specular * 0.5;
        }

        // Specular tint texture
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_SPECULAR_TINT_TEXTURE_BIT) != 0u) {
            let specular_tint =
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                bindless_textures_2d[material_indices[slot].specular_tint_texture],
                bindless_samplers_filtering[material_indices[slot].specular_tint_sampler],
#else   // BINDLESS
                pbr_bindings::specular_tint_texture,
                pbr_bindings::specular_tint_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_SPECULAR_TINT_UV_B
                uv_b,
#else   // STANDARD_MATERIAL_SPECULAR_TINT_UV_B
                uv,
#endif  // STANDARD_MATERIAL_SPECULAR_TINT_UV_B
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
            ).rgb;
            pbr_input.material.reflectance *= specular_tint;
        }

#endif  // VERTEX_UVS
#endif  // PBR_SPECULAR_TEXTURES_SUPPORTED

        // emissive
#ifdef BINDLESS
        var emissive: vec4<f32> = pbr_bindings::material_array[material_indices[slot].material].emissive;
#else   // BINDLESS
        var emissive: vec4<f32> = pbr_bindings::material.emissive;
#endif  // BINDLESS

#ifdef VERTEX_UVS
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb *
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                    bindless_textures_2d[material_indices[slot].emissive_texture],
                    bindless_samplers_filtering[material_indices[slot].emissive_sampler],
#else   // BINDLESS
                    pbr_bindings::emissive_texture,
                    pbr_bindings::emissive_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_EMISSIVE_UV_B
                    uv_b,
#else
                    uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
                ).rgb,
            emissive.a);
        }
#endif
        pbr_input.material.emissive = emissive;

        // metallic and perceptual roughness
#ifdef BINDLESS
        var metallic: f32 = pbr_bindings::material_array[material_indices[slot].material].metallic;
        var perceptual_roughness: f32 = pbr_bindings::material_array[material_indices[slot].material].perceptual_roughness;
#else   // BINDLESS
        var metallic: f32 = pbr_bindings::material.metallic;
        var perceptual_roughness: f32 = pbr_bindings::material.perceptual_roughness;
#endif  // BINDLESS

#ifdef VERTEX_UVS
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness =
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                    bindless_textures_2d[material_indices[slot].metallic_roughness_texture],
                    bindless_samplers_filtering[material_indices[slot].metallic_roughness_sampler],
#else   // BINDLESS
                    pbr_bindings::metallic_roughness_texture,
                    pbr_bindings::metallic_roughness_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_METALLIC_ROUGHNESS_UV_B
                    uv_b,
#else
                    uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
                );
            // Sampling from GLTF standard channels for now
            metallic *= metallic_roughness.b;
            perceptual_roughness *= metallic_roughness.g;
        }
#endif
        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        // Clearcoat factor
#ifdef BINDLESS
        pbr_input.material.clearcoat =
                pbr_bindings::material_array[material_indices[slot].material].clearcoat;
#else   // BINDLESS
        pbr_input.material.clearcoat = pbr_bindings::material.clearcoat;
#endif  // BINDLESS

#ifdef VERTEX_UVS
#ifdef PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_CLEARCOAT_TEXTURE_BIT) != 0u) {
            pbr_input.material.clearcoat *=
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                    bindless_textures_2d[material_indices[slot].clearcoat_texture],
                    bindless_samplers_filtering[material_indices[slot].clearcoat_sampler],
#else   // BINDLESS
                    pbr_bindings::clearcoat_texture,
                    pbr_bindings::clearcoat_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_CLEARCOAT_UV_B
                    uv_b,
#else
                    uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
                ).r;
        }
#endif  // PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED
#endif  // VERTEX_UVS

        // Clearcoat roughness
#ifdef BINDLESS
        pbr_input.material.clearcoat_perceptual_roughness =
            pbr_bindings::material_array[material_indices[slot].material].clearcoat_perceptual_roughness;
#else   // BINDLESS
        pbr_input.material.clearcoat_perceptual_roughness =
            pbr_bindings::material.clearcoat_perceptual_roughness;
#endif  // BINDLESS

#ifdef VERTEX_UVS
#ifdef PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_CLEARCOAT_ROUGHNESS_TEXTURE_BIT) != 0u) {
            pbr_input.material.clearcoat_perceptual_roughness *=
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                    bindless_textures_2d[material_indices[slot].clearcoat_roughness_texture],
                    bindless_samplers_filtering[material_indices[slot].clearcoat_roughness_sampler],
#else   // BINDLESS
                    pbr_bindings::clearcoat_roughness_texture,
                    pbr_bindings::clearcoat_roughness_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_CLEARCOAT_ROUGHNESS_UV_B
                    uv_b,
#else
                    uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
                ).g;
        }
#endif  // PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED
#endif  // VERTEX_UVS

#ifdef BINDLESS
        var specular_transmission: f32 = pbr_bindings::material_array[slot].specular_transmission;
#else   // BINDLESS
        var specular_transmission: f32 = pbr_bindings::material.specular_transmission;
#endif  // BINDLESS

#ifdef VERTEX_UVS
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_SPECULAR_TRANSMISSION_TEXTURE_BIT) != 0u) {
            specular_transmission *=
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                    bindless_textures_2d[
                        material_indices[slot].specular_transmission_texture
                    ],
                    bindless_samplers_filtering[
                        material_indices[slot].specular_transmission_sampler
                    ],
#else   // BINDLESS
                    pbr_bindings::specular_transmission_texture,
                    pbr_bindings::specular_transmission_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_SPECULAR_TRANSMISSION_UV_B
                    uv_b,
#else
                    uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
                ).r;
        }
#endif
#endif
        pbr_input.material.specular_transmission = specular_transmission;

#ifdef BINDLESS
        var thickness: f32 = pbr_bindings::material_array[material_indices[slot].material].thickness;
#else   // BINDLESS
        var thickness: f32 = pbr_bindings::material.thickness;
#endif  // BINDLESS

#ifdef VERTEX_UVS
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_THICKNESS_TEXTURE_BIT) != 0u) {
            thickness *=
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                    bindless_textures_2d[material_indices[slot].thickness_texture],
                    bindless_samplers_filtering[material_indices[slot].thickness_sampler],
#else   // BINDLESS
                    pbr_bindings::thickness_texture,
                    pbr_bindings::thickness_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_THICKNESS_UV_B
                    uv_b,
#else
                    uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
                ).g;
        }
#endif
#endif
        // scale thickness, accounting for non-uniform scaling (e.g. a “squished” mesh)
        // TODO: Meshlet support
#ifndef MESHLET_MESH_MATERIAL_PASS
        thickness *= length(
            (transpose(mesh[in.instance_index].world_from_local) * vec4(pbr_input.N, 0.0)).xyz
        );
#endif
        pbr_input.material.thickness = thickness;

#ifdef BINDLESS
        var diffuse_transmission =
                pbr_bindings::material_array[material_indices[slot].material].diffuse_transmission;
#else   // BINDLESS
        var diffuse_transmission = pbr_bindings::material.diffuse_transmission;
#endif  // BINDLESS

#ifdef VERTEX_UVS
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_DIFFUSE_TRANSMISSION_TEXTURE_BIT) != 0u) {
            diffuse_transmission *=
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                    bindless_textures_2d[material_indices[slot].diffuse_transmission_texture],
                    bindless_samplers_filtering[material_indices[slot].diffuse_transmission_sampler],
#else   // BINDLESS
                    pbr_bindings::diffuse_transmission_texture,
                    pbr_bindings::diffuse_transmission_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_DIFFUSE_TRANSMISSION_UV_B
                    uv_b,
#else
                    uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
                ).a;
        }
#endif
#endif
        pbr_input.material.diffuse_transmission = diffuse_transmission;

        var diffuse_occlusion: vec3<f32> = vec3(1.0);
        var specular_occlusion: f32 = 1.0;
#ifdef VERTEX_UVS
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            diffuse_occlusion *=
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                    bindless_textures_2d[material_indices[slot].occlusion_texture],
                    bindless_samplers_filtering[material_indices[slot].occlusion_sampler],
#else   // BINDLESS
                    pbr_bindings::occlusion_texture,
                    pbr_bindings::occlusion_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_OCCLUSION_UV_B
                    uv_b,
#else
                    uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
                ).r;
        }
#endif
#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
        let ssao = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0i).r;
        let ssao_multibounce = ssao_multibounce(ssao, pbr_input.material.base_color.rgb);
        diffuse_occlusion = min(diffuse_occlusion, ssao_multibounce);
        // Use SSAO to estimate the specular occlusion.
        // Lagarde and Rousiers 2014, "Moving Frostbite to Physically Based Rendering"
        let roughness = lighting::perceptualRoughnessToRoughness(pbr_input.material.perceptual_roughness);
        specular_occlusion = saturate(pow(NdotV + ssao, exp2(-16.0 * roughness - 1.0)) - 1.0 + ssao);
#endif
        pbr_input.diffuse_occlusion = diffuse_occlusion;
        pbr_input.specular_occlusion = specular_occlusion;

        // N (normal vector)
#ifndef LOAD_PREPASS_NORMALS

        pbr_input.N = normalize(pbr_input.world_normal);
        pbr_input.clearcoat_N = pbr_input.N;

#ifdef VERTEX_UVS
#ifdef VERTEX_TANGENTS

        let TBN = pbr_functions::calculate_tbn_mikktspace(pbr_input.world_normal, in.world_tangent);

#ifdef STANDARD_MATERIAL_NORMAL_MAP

        let Nt =
#ifdef MESHLET_MESH_MATERIAL_PASS
            textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
            textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                bindless_textures_2d[material_indices[slot].normal_map_texture],
                bindless_samplers_filtering[material_indices[slot].normal_map_sampler],
#else   // BINDLESS
                pbr_bindings::normal_map_texture,
                pbr_bindings::normal_map_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_NORMAL_MAP_UV_B
                uv_b,
#else
                uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                bias.ddx_uv,
                bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
            ).rgb;

        pbr_input.N = pbr_functions::apply_normal_mapping(flags, TBN, double_sided, is_front, Nt);

#endif  // STANDARD_MATERIAL_NORMAL_MAP

#ifdef STANDARD_MATERIAL_CLEARCOAT

        // Note: `KHR_materials_clearcoat` specifies that, if there's no
        // clearcoat normal map, we must set the normal to the mesh's normal,
        // and not to the main layer's bumped normal.

#ifdef STANDARD_MATERIAL_CLEARCOAT_NORMAL_MAP

        let clearcoat_Nt =
#ifdef MESHLET_MESH_MATERIAL_PASS
            textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
            textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                bindless_textures_2d[material_indices[slot].clearcoat_normal_texture],
                bindless_samplers_filtering[material_indices[slot].clearcoat_normal_sampler],
#else   // BINDLESS
                pbr_bindings::clearcoat_normal_texture,
                pbr_bindings::clearcoat_normal_sampler,
#endif  // BINDLESS
#ifdef STANDARD_MATERIAL_CLEARCOAT_NORMAL_UV_B
                uv_b,
#else
                uv,
#endif
#ifdef MESHLET_MESH_MATERIAL_PASS
                bias.ddx_uv,
                bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
            ).rgb;

        pbr_input.clearcoat_N = pbr_functions::apply_normal_mapping(
            flags,
            TBN,
            double_sided,
            is_front,
            clearcoat_Nt,
        );

#endif  // STANDARD_MATERIAL_CLEARCOAT_NORMAL_MAP

#endif  // STANDARD_MATERIAL_CLEARCOAT

#endif  // VERTEX_TANGENTS
#endif  // VERTEX_UVS

        // Take anisotropy into account.
        //
        // This code comes from the `KHR_materials_anisotropy` spec:
        // <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md#individual-lights>
#ifdef PBR_ANISOTROPY_TEXTURE_SUPPORTED
#ifdef VERTEX_TANGENTS
#ifdef STANDARD_MATERIAL_ANISOTROPY

#ifdef BINDLESS
        var anisotropy_strength =
                pbr_bindings::material_array[material_indices[slot].material].anisotropy_strength;
        var anisotropy_direction =
                pbr_bindings::material_array[material_indices[slot].material].anisotropy_rotation;
#else   // BINDLESS
        var anisotropy_strength = pbr_bindings::material.anisotropy_strength;
        var anisotropy_direction = pbr_bindings::material.anisotropy_rotation;
#endif  // BINDLESS

        // Adjust based on the anisotropy map if there is one.
        if ((flags & pbr_types::STANDARD_MATERIAL_FLAGS_ANISOTROPY_TEXTURE_BIT) != 0u) {
            let anisotropy_texel =
#ifdef MESHLET_MESH_MATERIAL_PASS
                textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
                textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                    bindless_textures_2d[material_indices[slot].anisotropy_texture],
                    bindless_samplers_filtering[material_indices[slot].anisotropy_sampler],
#else   // BINDLESS
                    pbr_bindings::anisotropy_texture,
                    pbr_bindings::anisotropy_sampler,
#endif
#ifdef STANDARD_MATERIAL_ANISOTROPY_UV_B
                    uv_b,
#else   // STANDARD_MATERIAL_ANISOTROPY_UV_B
                    uv,
#endif  // STANDARD_MATERIAL_ANISOTROPY_UV_B
#ifdef MESHLET_MESH_MATERIAL_PASS
                    bias.ddx_uv,
                    bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                    bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
                ).rgb;

            let anisotropy_direction_from_texture = normalize(anisotropy_texel.rg * 2.0 - 1.0);
            // Rotate by the anisotropy direction.
            anisotropy_direction =
                mat2x2(anisotropy_direction.xy, anisotropy_direction.yx * vec2(-1.0, 1.0)) *
                anisotropy_direction_from_texture;
            anisotropy_strength *= anisotropy_texel.b;
        }

        pbr_input.anisotropy_strength = anisotropy_strength;

        let anisotropy_T = normalize(TBN * vec3(anisotropy_direction, 0.0));
        let anisotropy_B = normalize(cross(pbr_input.world_normal, anisotropy_T));
        pbr_input.anisotropy_T = anisotropy_T;
        pbr_input.anisotropy_B = anisotropy_B;

#endif  // STANDARD_MATERIAL_ANISOTROPY
#endif  // VERTEX_TANGENTS
#endif  // PBR_ANISOTROPY_TEXTURE_SUPPORTED

#endif  // LOAD_PREPASS_NORMALS

// TODO: Meshlet support
#ifdef LIGHTMAP

#ifdef BINDLESS
        let lightmap_exposure =
                pbr_bindings::material_array[material_indices[slot].material].lightmap_exposure;
#else   // BINDLESS
        let lightmap_exposure = pbr_bindings::material.lightmap_exposure;
#endif  // BINDLESS

        pbr_input.lightmap_light = lightmap(in.uv_b, lightmap_exposure, in.instance_index);
#endif
    }

    return pbr_input;
}
