#define_import_path bevy_pbr::pbr_functions

#import bevy_pbr::{
    pbr_types,
    pbr_bindings,
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    lighting,
    lighting::{LAYER_BASE, LAYER_CLEARCOAT},
    transmission,
    clustered_forward as clustering,
    shadows,
    ambient,
    irradiance_volume,
    mesh_types::{MESH_FLAGS_SHADOW_RECEIVER_BIT, MESH_FLAGS_TRANSMITTED_SHADOW_RECEIVER_BIT},
}
#import bevy_render::maths::{E, powsafe}

#ifdef MESHLET_MESH_MATERIAL_PASS
#import bevy_pbr::meshlet_visibility_buffer_resolve::VertexOutput
#else ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::VertexOutput
#else   // PREPASS_PIPELINE
#import bevy_pbr::forward_io::VertexOutput
#endif  // PREPASS_PIPELINE

#ifdef ENVIRONMENT_MAP
#import bevy_pbr::environment_map
#endif

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping::{tone_mapping, screen_space_dither}
#endif


// Biasing info needed to sample from a texture when calling `sample_texture`.
// How this is done depends on whether we're rendering meshlets or regular
// meshes.
struct SampleBias {
#ifdef MESHLET_MESH_MATERIAL_PASS
    ddx_uv: vec2<f32>,
    ddy_uv: vec2<f32>,
#else   // MESHLET_MESH_MATERIAL_PASS
    mip_bias: f32,
#endif  // MESHLET_MESH_MATERIAL_PASS
}

// This is the standard 4x4 ordered dithering pattern from [1].
//
// We can't use `array<vec4<u32>, 4>` because they can't be indexed dynamically
// due to Naga limitations. So instead we pack into a single `vec4` and extract
// individual bytes.
//
// [1]: https://en.wikipedia.org/wiki/Ordered_dithering#Threshold_map
const DITHER_THRESHOLD_MAP: vec4<u32> = vec4(
    0x0a020800,
    0x060e040c,
    0x09010b03,
    0x050d070f
);

// Processes a visibility range dither value and discards the fragment if
// needed.
//
// Visibility ranges, also known as HLODs, are crossfades between different
// levels of detail.
//
// The `dither` value ranges from [-16, 16]. When zooming out, positive values
// are used for meshes that are in the process of disappearing, while negative
// values are used for meshes that are in the process of appearing. In other
// words, when the camera is moving backwards, the `dither` value counts up from
// -16 to 0 when the object is fading in, stays at 0 while the object is
// visible, and then counts up to 16 while the object is fading out.
// Distinguishing between negative and positive values allows the dither
// patterns for different LOD levels of a single mesh to mesh together properly.
#ifdef VISIBILITY_RANGE_DITHER
fn visibility_range_dither(frag_coord: vec4<f32>, dither: i32) {
    // If `dither` is 0, the object is visible.
    if (dither == 0) {
        return;
    }

    // If `dither` is less than -15 or greater than 15, the object is culled.
    if (dither <= -16 || dither >= 16) {
        discard;
    }

    // Otherwise, check the dither pattern.
    let coords = vec2<u32>(floor(frag_coord.xy)) % 4u;
    let threshold = i32((DITHER_THRESHOLD_MAP[coords.y] >> (coords.x * 8)) & 0xff);
    if ((dither >= 0 && dither + threshold >= 16) || (dither < 0 && 1 + dither + threshold <= 0)) {
        discard;
    }
}
#endif

fn alpha_discard(material: pbr_types::StandardMaterial, output_color: vec4<f32>) -> vec4<f32> {
    var color = output_color;
    let alpha_mode = material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        // NOTE: If rendering as opaque, alpha should be ignored so set to 1.0
        color.a = 1.0;
    }

#ifdef MAY_DISCARD
    // NOTE: `MAY_DISCARD` is only defined in the alpha to coverage case if MSAA
    // was off. This special situation causes alpha to coverage to fall back to
    // alpha mask.
    else if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK ||
            alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ALPHA_TO_COVERAGE {
        if color.a >= material.alpha_cutoff {
            // NOTE: If rendering as masked alpha and >= the cutoff, render as fully opaque
            color.a = 1.0;
        } else {
            // NOTE: output_color.a < in.material.alpha_cutoff should not be rendered
            discard;
        }
    }
#endif

    return color;
}

// Samples a texture using the appropriate biasing metric for the type of mesh
// in use (mesh vs. meshlet).
fn sample_texture(
    texture: texture_2d<f32>,
    samp: sampler,
    uv: vec2<f32>,
    bias: SampleBias,
) -> vec4<f32> {
#ifdef MESHLET_MESH_MATERIAL_PASS
    return textureSampleGrad(texture, samp, uv, bias.ddx_uv, bias.ddy_uv);
#else
    return textureSampleBias(texture, samp, uv, bias.mip_bias);
#endif
}

fn prepare_world_normal(
    world_normal: vec3<f32>,
    double_sided: bool,
    is_front: bool,
) -> vec3<f32> {
    var output: vec3<f32> = world_normal;
#ifndef VERTEX_TANGENTS
#ifndef STANDARD_MATERIAL_NORMAL_MAP
    // NOTE: When NOT using normal-mapping, if looking at the back face of a double-sided
    // material, the normal needs to be inverted. This is a branchless version of that.
    output = (f32(!double_sided || is_front) * 2.0 - 1.0) * output;
#endif
#endif
    return output;
}

// Calculates the three TBN vectors according to [mikktspace]. Returns a matrix
// with T, B, N columns in that order.
//
// [mikktspace]: http://www.mikktspace.com/
fn calculate_tbn_mikktspace(world_normal: vec3<f32>, world_tangent: vec4<f32>) -> mat3x3<f32> {
    // NOTE: The mikktspace method of normal mapping explicitly requires that the world normal NOT
    // be re-normalized in the fragment shader. This is primarily to match the way mikktspace
    // bakes vertex tangents and normal maps so that this is the exact inverse. Blender, Unity,
    // Unreal Engine, Godot, and more all use the mikktspace method. Do not change this code
    // unless you really know what you are doing.
    // http://www.mikktspace.com/
    var N: vec3<f32> = world_normal;

    // NOTE: The mikktspace method of normal mapping explicitly requires that these NOT be
    // normalized nor any Gram-Schmidt applied to ensure the vertex normal is orthogonal to the
    // vertex tangent! Do not change this code unless you really know what you are doing.
    // http://www.mikktspace.com/
    var T: vec3<f32> = world_tangent.xyz;
    var B: vec3<f32> = world_tangent.w * cross(N, T);

    return mat3x3(T, B, N);
}

fn apply_normal_mapping(
    standard_material_flags: u32,
    TBN: mat3x3<f32>,
    double_sided: bool,
    is_front: bool,
    in_Nt: vec3<f32>,
) -> vec3<f32> {
    // Unpack the TBN vectors.
    var T = TBN[0];
    var B = TBN[1];
    var N = TBN[2];

    // Nt is the tangent-space normal.
    var Nt = in_Nt;
    if (standard_material_flags & pbr_types::STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP) != 0u {
        // Only use the xy components and derive z for 2-component normal maps.
        Nt = vec3<f32>(Nt.rg * 2.0 - 1.0, 0.0);
        Nt.z = sqrt(1.0 - Nt.x * Nt.x - Nt.y * Nt.y);
    } else {
        Nt = Nt * 2.0 - 1.0;
    }
    // Normal maps authored for DirectX require flipping the y component
    if (standard_material_flags & pbr_types::STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y) != 0u {
        Nt.y = -Nt.y;
    }

    if double_sided && !is_front {
        Nt = -Nt;
    }

    // NOTE: The mikktspace method of normal mapping applies maps the tangent-space normal from
    // the normal map texture in this way to be an EXACT inverse of how the normal map baker
    // calculates the normal maps so there is no error introduced. Do not change this code
    // unless you really know what you are doing.
    // http://www.mikktspace.com/
    N = Nt.x * T + Nt.y * B + Nt.z * N;

    return normalize(N);
}

#ifdef STANDARD_MATERIAL_ANISOTROPY

// Modifies the normal to achieve a better approximate direction from the
// environment map when using anisotropy.
//
// This follows the suggested implementation in the `KHR_materials_anisotropy` specification:
// https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md#image-based-lighting
fn bend_normal_for_anisotropy(lighting_input: ptr<function, lighting::LightingInput>) {
    // Unpack.
    let N = (*lighting_input).layers[LAYER_BASE].N;
    let roughness = (*lighting_input).layers[LAYER_BASE].roughness;
    let V = (*lighting_input).V;
    let anisotropy = (*lighting_input).anisotropy;
    let Ba = (*lighting_input).Ba;

    var bent_normal = normalize(cross(cross(Ba, V), Ba));

    // The `KHR_materials_anisotropy` spec states:
    //
    // > This heuristic can probably be improved upon
    let a = pow(2.0, pow(2.0, 1.0 - anisotropy * (1.0 - roughness)));
    bent_normal = normalize(mix(bent_normal, N, a));

    // The `KHR_materials_anisotropy` spec states:
    //
    // > Mixing the reflection with the normal is more accurate both with and
    // > without anisotropy and keeps rough objects from gathering light from
    // > behind their tangent plane.
    let R = normalize(mix(reflect(-V, bent_normal), bent_normal, roughness * roughness));

    (*lighting_input).layers[LAYER_BASE].N = bent_normal;
    (*lighting_input).layers[LAYER_BASE].R = R;
}

#endif  // STANDARD_MATERIAL_ANISTROPY

// NOTE: Correctly calculates the view vector depending on whether
// the projection is orthographic or perspective.
fn calculate_view(
    world_position: vec4<f32>,
    is_orthographic: bool,
) -> vec3<f32> {
    var V: vec3<f32>;
    if is_orthographic {
        // Orthographic view vector
        V = normalize(vec3<f32>(view_bindings::view.clip_from_world[0].z, view_bindings::view.clip_from_world[1].z, view_bindings::view.clip_from_world[2].z));
    } else {
        // Only valid for a perspective projection
        V = normalize(view_bindings::view.world_position.xyz - world_position.xyz);
    }
    return V;
}

// Diffuse strength is inversely related to metallicity, specular and diffuse transmission
fn calculate_diffuse_color(
    base_color: vec3<f32>,
    metallic: f32,
    specular_transmission: f32,
    diffuse_transmission: f32
) -> vec3<f32> {
    return base_color * (1.0 - metallic) * (1.0 - specular_transmission) *
        (1.0 - diffuse_transmission);
}

// Remapping [0,1] reflectance to F0
// See https://google.github.io/filament/Filament.html#materialsystem/parameterization/remapping
fn calculate_F0(base_color: vec3<f32>, metallic: f32, reflectance: f32) -> vec3<f32> {
    return 0.16 * reflectance * reflectance * (1.0 - metallic) + base_color * metallic;
}

#ifndef PREPASS_FRAGMENT
fn apply_pbr_lighting(
    in: pbr_types::PbrInput,
) -> vec4<f32> {
    var output_color: vec4<f32> = in.material.base_color;

    let emissive = in.material.emissive;

    // calculate non-linear roughness from linear perceptualRoughness
    let metallic = in.material.metallic;
    let perceptual_roughness = in.material.perceptual_roughness;
    let roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);
    let ior = in.material.ior;
    let thickness = in.material.thickness;
    let reflectance = in.material.reflectance;
    let diffuse_transmission = in.material.diffuse_transmission;
    let specular_transmission = in.material.specular_transmission;

    let specular_transmissive_color = specular_transmission * in.material.base_color.rgb;

    let diffuse_occlusion = in.diffuse_occlusion;
    let specular_occlusion = in.specular_occlusion;

    // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
    let NdotV = max(dot(in.N, in.V), 0.0001);
    let R = reflect(-in.V, in.N);

#ifdef STANDARD_MATERIAL_CLEARCOAT
    // Do the above calculations again for the clearcoat layer. Remember that
    // the clearcoat can have its own roughness and its own normal.
    let clearcoat = in.material.clearcoat;
    let clearcoat_perceptual_roughness = in.material.clearcoat_perceptual_roughness;
    let clearcoat_roughness = lighting::perceptualRoughnessToRoughness(clearcoat_perceptual_roughness);
    let clearcoat_N = in.clearcoat_N;
    let clearcoat_NdotV = max(dot(clearcoat_N, in.V), 0.0001);
    let clearcoat_R = reflect(-in.V, clearcoat_N);
#endif  // STANDARD_MATERIAL_CLEARCOAT

    let diffuse_color = calculate_diffuse_color(
        output_color.rgb,
        metallic,
        specular_transmission,
        diffuse_transmission
    );

    // Diffuse transmissive strength is inversely related to metallicity and specular transmission, but directly related to diffuse transmission
    let diffuse_transmissive_color = output_color.rgb * (1.0 - metallic) * (1.0 - specular_transmission) * diffuse_transmission;

    // Calculate the world position of the second Lambertian lobe used for diffuse transmission, by subtracting material thickness
    let diffuse_transmissive_lobe_world_position = in.world_position - vec4<f32>(in.world_normal, 0.0) * thickness;

    let F0 = calculate_F0(output_color.rgb, metallic, reflectance);
    let F_ab = lighting::F_AB(perceptual_roughness, NdotV);

    var direct_light: vec3<f32> = vec3<f32>(0.0);

    // Transmitted Light (Specular and Diffuse)
    var transmitted_light: vec3<f32> = vec3<f32>(0.0);

    // Pack all the values into a structure.
    var lighting_input: lighting::LightingInput;
    lighting_input.layers[LAYER_BASE].NdotV = NdotV;
    lighting_input.layers[LAYER_BASE].N = in.N;
    lighting_input.layers[LAYER_BASE].R = R;
    lighting_input.layers[LAYER_BASE].perceptual_roughness = perceptual_roughness;
    lighting_input.layers[LAYER_BASE].roughness = roughness;
    lighting_input.P = in.world_position.xyz;
    lighting_input.V = in.V;
    lighting_input.diffuse_color = diffuse_color;
    lighting_input.F0_ = F0;
    lighting_input.F_ab = F_ab;
#ifdef STANDARD_MATERIAL_CLEARCOAT
    lighting_input.layers[LAYER_CLEARCOAT].NdotV = clearcoat_NdotV;
    lighting_input.layers[LAYER_CLEARCOAT].N = clearcoat_N;
    lighting_input.layers[LAYER_CLEARCOAT].R = clearcoat_R;
    lighting_input.layers[LAYER_CLEARCOAT].perceptual_roughness = clearcoat_perceptual_roughness;
    lighting_input.layers[LAYER_CLEARCOAT].roughness = clearcoat_roughness;
    lighting_input.clearcoat_strength = clearcoat;
#endif  // STANDARD_MATERIAL_CLEARCOAT
#ifdef STANDARD_MATERIAL_ANISOTROPY
    lighting_input.anisotropy = in.anisotropy_strength;
    lighting_input.Ta = in.anisotropy_T;
    lighting_input.Ba = in.anisotropy_B;
#endif  // STANDARD_MATERIAL_ANISOTROPY

    // And do the same for transmissive if we need to.
#ifdef STANDARD_MATERIAL_DIFFUSE_TRANSMISSION
    var transmissive_lighting_input: lighting::LightingInput;
    transmissive_lighting_input.layers[LAYER_BASE].NdotV = 1.0;
    transmissive_lighting_input.layers[LAYER_BASE].N = -in.N;
    transmissive_lighting_input.layers[LAYER_BASE].R = vec3(0.0);
    transmissive_lighting_input.layers[LAYER_BASE].perceptual_roughness = 1.0;
    transmissive_lighting_input.layers[LAYER_BASE].roughness = 1.0;
    transmissive_lighting_input.P = diffuse_transmissive_lobe_world_position.xyz;
    transmissive_lighting_input.V = -in.V;
    transmissive_lighting_input.diffuse_color = diffuse_transmissive_color;
    transmissive_lighting_input.F0_ = vec3(0.0);
    transmissive_lighting_input.F_ab = vec2(0.1);
#ifdef STANDARD_MATERIAL_CLEARCOAT
    transmissive_lighting_input.layers[LAYER_CLEARCOAT].NdotV = 0.0;
    transmissive_lighting_input.layers[LAYER_CLEARCOAT].N = vec3(0.0);
    transmissive_lighting_input.layers[LAYER_CLEARCOAT].R = vec3(0.0);
    transmissive_lighting_input.layers[LAYER_CLEARCOAT].perceptual_roughness = 0.0;
    transmissive_lighting_input.layers[LAYER_CLEARCOAT].roughness = 0.0;
    transmissive_lighting_input.clearcoat_strength = 0.0;
#endif  // STANDARD_MATERIAL_CLEARCOAT
#ifdef STANDARD_MATERIAL_ANISOTROPY
    lighting_input.anisotropy = in.anisotropy_strength;
    lighting_input.Ta = in.anisotropy_T;
    lighting_input.Ba = in.anisotropy_B;
#endif  // STANDARD_MATERIAL_ANISOTROPY
#endif  // STANDARD_MATERIAL_DIFFUSE_TRANSMISSION

    let view_z = dot(vec4<f32>(
        view_bindings::view.view_from_world[0].z,
        view_bindings::view.view_from_world[1].z,
        view_bindings::view.view_from_world[2].z,
        view_bindings::view.view_from_world[3].z
    ), in.world_position);
    let cluster_index = clustering::fragment_cluster_index(in.frag_coord.xy, view_z, in.is_orthographic);
    let offset_and_counts = clustering::unpack_offset_and_counts(cluster_index);

    // Point lights (direct)
    for (var i: u32 = offset_and_counts[0]; i < offset_and_counts[0] + offset_and_counts[1]; i = i + 1u) {
        let light_id = clustering::get_clusterable_object_id(i);
        var shadow: f32 = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && (view_bindings::clusterable_objects.data[light_id].flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_point_shadow(light_id, in.world_position, in.world_normal);
        }

        let light_contrib = lighting::point_light(light_id, &lighting_input);
        direct_light += light_contrib * shadow;

#ifdef STANDARD_MATERIAL_DIFFUSE_TRANSMISSION
        // NOTE: We use the diffuse transmissive color, the second Lambertian lobe's calculated
        // world position, inverted normal and view vectors, and the following simplified
        // values for a fully diffuse transmitted light contribution approximation:
        //
        // roughness = 1.0;
        // NdotV = 1.0;
        // R = vec3<f32>(0.0) // doesn't really matter
        // F_ab = vec2<f32>(0.1)
        // F0 = vec3<f32>(0.0)
        var transmitted_shadow: f32 = 1.0;
        if ((in.flags & (MESH_FLAGS_SHADOW_RECEIVER_BIT | MESH_FLAGS_TRANSMITTED_SHADOW_RECEIVER_BIT)) == (MESH_FLAGS_SHADOW_RECEIVER_BIT | MESH_FLAGS_TRANSMITTED_SHADOW_RECEIVER_BIT)
                && (view_bindings::clusterable_objects.data[light_id].flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            transmitted_shadow = shadows::fetch_point_shadow(light_id, diffuse_transmissive_lobe_world_position, -in.world_normal);
        }

        let transmitted_light_contrib =
            lighting::point_light(light_id, &transmissive_lighting_input);
        transmitted_light += transmitted_light_contrib * transmitted_shadow;
#endif
    }

    // Spot lights (direct)
    for (var i: u32 = offset_and_counts[0] + offset_and_counts[1]; i < offset_and_counts[0] + offset_and_counts[1] + offset_and_counts[2]; i = i + 1u) {
        let light_id = clustering::get_clusterable_object_id(i);

        var shadow: f32 = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && (view_bindings::clusterable_objects.data[light_id].flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_spot_shadow(light_id, in.world_position, in.world_normal);
        }

        let light_contrib = lighting::spot_light(light_id, &lighting_input);
        direct_light += light_contrib * shadow;

#ifdef STANDARD_MATERIAL_DIFFUSE_TRANSMISSION
        // NOTE: We use the diffuse transmissive color, the second Lambertian lobe's calculated
        // world position, inverted normal and view vectors, and the following simplified
        // values for a fully diffuse transmitted light contribution approximation:
        //
        // roughness = 1.0;
        // NdotV = 1.0;
        // R = vec3<f32>(0.0) // doesn't really matter
        // F_ab = vec2<f32>(0.1)
        // F0 = vec3<f32>(0.0)
        var transmitted_shadow: f32 = 1.0;
        if ((in.flags & (MESH_FLAGS_SHADOW_RECEIVER_BIT | MESH_FLAGS_TRANSMITTED_SHADOW_RECEIVER_BIT)) == (MESH_FLAGS_SHADOW_RECEIVER_BIT | MESH_FLAGS_TRANSMITTED_SHADOW_RECEIVER_BIT)
                && (view_bindings::clusterable_objects.data[light_id].flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            transmitted_shadow = shadows::fetch_spot_shadow(light_id, diffuse_transmissive_lobe_world_position, -in.world_normal);
        }

        let transmitted_light_contrib =
            lighting::spot_light(light_id, &transmissive_lighting_input);
        transmitted_light += transmitted_light_contrib * transmitted_shadow;
#endif
    }

    // directional lights (direct)
    let n_directional_lights = view_bindings::lights.n_directional_lights;
    for (var i: u32 = 0u; i < n_directional_lights; i = i + 1u) {
        // check if this light should be skipped, which occurs if this light does not intersect with the view
        // note point and spot lights aren't skippable, as the relevant lights are filtered in `assign_lights_to_clusters`
        let light = &view_bindings::lights.directional_lights[i];
        if (*light).skip != 0u {
            continue;
        }

        var shadow: f32 = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && (view_bindings::lights.directional_lights[i].flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_directional_shadow(i, in.world_position, in.world_normal, view_z);
        }

        var light_contrib = lighting::directional_light(i, &lighting_input);

#ifdef DIRECTIONAL_LIGHT_SHADOW_MAP_DEBUG_CASCADES
        light_contrib = shadows::cascade_debug_visualization(light_contrib, i, view_z);
#endif
        direct_light += light_contrib * shadow;

#ifdef STANDARD_MATERIAL_DIFFUSE_TRANSMISSION
        // NOTE: We use the diffuse transmissive color, the second Lambertian lobe's calculated
        // world position, inverted normal and view vectors, and the following simplified
        // values for a fully diffuse transmitted light contribution approximation:
        //
        // roughness = 1.0;
        // NdotV = 1.0;
        // R = vec3<f32>(0.0) // doesn't really matter
        // F_ab = vec2<f32>(0.1)
        // F0 = vec3<f32>(0.0)
        var transmitted_shadow: f32 = 1.0;
        if ((in.flags & (MESH_FLAGS_SHADOW_RECEIVER_BIT | MESH_FLAGS_TRANSMITTED_SHADOW_RECEIVER_BIT)) == (MESH_FLAGS_SHADOW_RECEIVER_BIT | MESH_FLAGS_TRANSMITTED_SHADOW_RECEIVER_BIT)
                && (view_bindings::lights.directional_lights[i].flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            transmitted_shadow = shadows::fetch_directional_shadow(i, diffuse_transmissive_lobe_world_position, -in.world_normal, view_z);
        }

        let transmitted_light_contrib =
            lighting::directional_light(i, &transmissive_lighting_input);
        transmitted_light += transmitted_light_contrib * transmitted_shadow;
#endif
    }

#ifdef STANDARD_MATERIAL_DIFFUSE_TRANSMISSION
    // NOTE: We use the diffuse transmissive color, the second Lambertian lobe's calculated
    // world position, inverted normal and view vectors, and the following simplified
    // values for a fully diffuse transmitted light contribution approximation:
    //
    // perceptual_roughness = 1.0;
    // NdotV = 1.0;
    // F0 = vec3<f32>(0.0)
    // diffuse_occlusion = vec3<f32>(1.0)
    transmitted_light += ambient::ambient_light(diffuse_transmissive_lobe_world_position, -in.N, -in.V, 1.0, diffuse_transmissive_color, vec3<f32>(0.0), 1.0, vec3<f32>(1.0));
#endif

    // Diffuse indirect lighting can come from a variety of sources. The
    // priority goes like this:
    //
    // 1. Lightmap (highest)
    // 2. Irradiance volume
    // 3. Environment map (lowest)
    //
    // When we find a source of diffuse indirect lighting, we stop accumulating
    // any more diffuse indirect light. This avoids double-counting if, for
    // example, both lightmaps and irradiance volumes are present.

    var indirect_light = vec3(0.0f);

#ifdef LIGHTMAP
    if (all(indirect_light == vec3(0.0f))) {
        indirect_light += in.lightmap_light * diffuse_color;
    }
#endif

#ifdef IRRADIANCE_VOLUME {
    // Irradiance volume light (indirect)
    if (all(indirect_light == vec3(0.0f))) {
        let irradiance_volume_light = irradiance_volume::irradiance_volume_light(
            in.world_position.xyz, in.N);
        indirect_light += irradiance_volume_light * diffuse_color * diffuse_occlusion;
    }
#endif

    // Environment map light (indirect)
#ifdef ENVIRONMENT_MAP

#ifdef STANDARD_MATERIAL_ANISOTROPY
    var bent_normal_lighting_input = lighting_input;
    bend_normal_for_anisotropy(&bent_normal_lighting_input);
    let environment_map_lighting_input = &bent_normal_lighting_input;
#else   // STANDARD_MATERIAL_ANISOTROPY
    let environment_map_lighting_input = &lighting_input;
#endif  // STANDARD_MATERIAL_ANISOTROPY

    let environment_light = environment_map::environment_map_light(
        environment_map_lighting_input,
        any(indirect_light != vec3(0.0f))
    );

    // If screen space reflections are going to be used for this material, don't
    // accumulate environment map light yet. The SSR shader will do it.
#ifdef SCREEN_SPACE_REFLECTIONS
    let use_ssr = perceptual_roughness <=
        view_bindings::ssr_settings.perceptual_roughness_threshold;
#else   // SCREEN_SPACE_REFLECTIONS
    let use_ssr = false;
#endif  // SCREEN_SPACE_REFLECTIONS

    if (!use_ssr) {
        let environment_light = environment_map::environment_map_light(
            &lighting_input,
            any(indirect_light != vec3(0.0f))
        );

        indirect_light += environment_light.diffuse * diffuse_occlusion +
            environment_light.specular * specular_occlusion;
    }

#endif  // ENVIRONMENT_MAP

    // Ambient light (indirect)
    indirect_light += ambient::ambient_light(in.world_position, in.N, in.V, NdotV, diffuse_color, F0, perceptual_roughness, diffuse_occlusion);

    // we'll use the specular component of the transmitted environment
    // light in the call to `specular_transmissive_light()` below
    var specular_transmitted_environment_light = vec3<f32>(0.0);

#ifdef ENVIRONMENT_MAP

#ifdef STANDARD_MATERIAL_DIFFUSE_OR_SPECULAR_TRANSMISSION
    // NOTE: We use the diffuse transmissive color, inverted normal and view vectors,
    // and the following simplified values for the transmitted environment light contribution
    // approximation:
    //
    // diffuse_color = vec3<f32>(1.0) // later we use `diffuse_transmissive_color` and `specular_transmissive_color`
    // NdotV = 1.0;
    // R = T // see definition below
    // F0 = vec3<f32>(1.0)
    // diffuse_occlusion = 1.0
    //
    // (This one is slightly different from the other light types above, because the environment
    // map light returns both diffuse and specular components separately, and we want to use both)

    let T = -normalize(
        in.V + // start with view vector at entry point
        refract(in.V, -in.N, 1.0 / ior) * thickness // add refracted vector scaled by thickness, towards exit point
    ); // normalize to find exit point view vector

    var transmissive_environment_light_input: lighting::LightingInput;
    transmissive_environment_light_input.diffuse_color = vec3(1.0);
    transmissive_environment_light_input.layers[LAYER_BASE].NdotV = 1.0;
    transmissive_environment_light_input.P = in.world_position.xyz;
    transmissive_environment_light_input.layers[LAYER_BASE].N = -in.N;
    transmissive_environment_light_input.V = in.V;
    transmissive_environment_light_input.layers[LAYER_BASE].R = T;
    transmissive_environment_light_input.layers[LAYER_BASE].perceptual_roughness = perceptual_roughness;
    transmissive_environment_light_input.layers[LAYER_BASE].roughness = roughness;
    transmissive_environment_light_input.F0_ = vec3<f32>(1.0);
    transmissive_environment_light_input.F_ab = vec2(0.1);
#ifdef STANDARD_MATERIAL_CLEARCOAT
    // No clearcoat.
    transmissive_environment_light_input.clearcoat_strength = 0.0;
    transmissive_environment_light_input.layers[LAYER_CLEARCOAT].NdotV = 0.0;
    transmissive_environment_light_input.layers[LAYER_CLEARCOAT].N = in.N;
    transmissive_environment_light_input.layers[LAYER_CLEARCOAT].R = vec3(0.0);
    transmissive_environment_light_input.layers[LAYER_CLEARCOAT].perceptual_roughness = 0.0;
    transmissive_environment_light_input.layers[LAYER_CLEARCOAT].roughness = 0.0;
#endif  // STANDARD_MATERIAL_CLEARCOAT

    let transmitted_environment_light =
        environment_map::environment_map_light(&transmissive_environment_light_input, false);

#ifdef STANDARD_MATERIAL_DIFFUSE_TRANSMISSION
    transmitted_light += transmitted_environment_light.diffuse * diffuse_transmissive_color;
#endif  // STANDARD_MATERIAL_DIFFUSE_TRANSMISSION
#ifdef STANDARD_MATERIAL_SPECULAR_TRANSMISSION
    specular_transmitted_environment_light = transmitted_environment_light.specular * specular_transmissive_color;
#endif  // STANDARD_MATERIAL_SPECULAR_TRANSMISSION

#endif  // STANDARD_MATERIAL_SPECULAR_OR_DIFFUSE_TRANSMISSION

#endif  // ENVIRONMENT_MAP

    var emissive_light = emissive.rgb * output_color.a;

    // "The clearcoat layer is on top of emission in the layering stack.
    // Consequently, the emission is darkened by the Fresnel term."
    //
    // <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_clearcoat/README.md#emission>
#ifdef STANDARD_MATERIAL_CLEARCOAT
    emissive_light = emissive_light * (0.04 + (1.0 - 0.04) * pow(1.0 - clearcoat_NdotV, 5.0));
#endif

    emissive_light = emissive_light * mix(1.0, view_bindings::view.exposure, emissive.a);

#ifdef STANDARD_MATERIAL_SPECULAR_TRANSMISSION
    transmitted_light += transmission::specular_transmissive_light(in.world_position, in.frag_coord.xyz, view_z, in.N, in.V, F0, ior, thickness, perceptual_roughness, specular_transmissive_color, specular_transmitted_environment_light).rgb;

    if (in.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ATTENUATION_ENABLED_BIT) != 0u {
        // We reuse the `atmospheric_fog()` function here, as it's fundamentally
        // equivalent to the attenuation that takes place inside the material volume,
        // and will allow us to eventually hook up subsurface scattering more easily
        var attenuation_fog: mesh_view_types::Fog;
        attenuation_fog.base_color.a = 1.0;
        attenuation_fog.be = pow(1.0 - in.material.attenuation_color.rgb, vec3<f32>(E)) / in.material.attenuation_distance;
        // TODO: Add the subsurface scattering factor below
        // attenuation_fog.bi = /* ... */
        transmitted_light = bevy_pbr::fog::atmospheric_fog(
            attenuation_fog, vec4<f32>(transmitted_light, 1.0), thickness,
            vec3<f32>(0.0) // TODO: Pass in (pre-attenuated) scattered light contribution here
        ).rgb;
    }
#endif

    // Total light
    output_color = vec4<f32>(
        (view_bindings::view.exposure * (transmitted_light + direct_light + indirect_light)) + emissive_light,
        output_color.a
    );

    output_color = clustering::cluster_debug_visualization(
        output_color,
        view_z,
        in.is_orthographic,
        offset_and_counts,
        cluster_index,
    );

    return output_color;
}
#endif // PREPASS_FRAGMENT

fn apply_fog(fog_params: mesh_view_types::Fog, input_color: vec4<f32>, fragment_world_position: vec3<f32>, view_world_position: vec3<f32>) -> vec4<f32> {
    let view_to_world = fragment_world_position.xyz - view_world_position.xyz;

    // `length()` is used here instead of just `view_to_world.z` since that produces more
    // high quality results, especially for denser/smaller fogs. we get a "curved"
    // fog shape that remains consistent with camera rotation, instead of a "linear"
    // fog shape that looks a bit fake
    let distance = length(view_to_world);

    var scattering = vec3<f32>(0.0);
    if fog_params.directional_light_color.a > 0.0 {
        let view_to_world_normalized = view_to_world / distance;
        let n_directional_lights = view_bindings::lights.n_directional_lights;
        for (var i: u32 = 0u; i < n_directional_lights; i = i + 1u) {
            let light = view_bindings::lights.directional_lights[i];
            scattering += pow(
                max(
                    dot(view_to_world_normalized, light.direction_to_light),
                    0.0
                ),
                fog_params.directional_light_exponent
            ) * light.color.rgb * view_bindings::view.exposure;
        }
    }

    if fog_params.mode == mesh_view_types::FOG_MODE_LINEAR {
        return bevy_pbr::fog::linear_fog(fog_params, input_color, distance, scattering);
    } else if fog_params.mode == mesh_view_types::FOG_MODE_EXPONENTIAL {
        return bevy_pbr::fog::exponential_fog(fog_params, input_color, distance, scattering);
    } else if fog_params.mode == mesh_view_types::FOG_MODE_EXPONENTIAL_SQUARED {
        return bevy_pbr::fog::exponential_squared_fog(fog_params, input_color, distance, scattering);
    } else if fog_params.mode == mesh_view_types::FOG_MODE_ATMOSPHERIC {
        return bevy_pbr::fog::atmospheric_fog(fog_params, input_color, distance, scattering);
    } else {
        return input_color;
    }
}

#ifdef PREMULTIPLY_ALPHA
fn premultiply_alpha(standard_material_flags: u32, color: vec4<f32>) -> vec4<f32> {
// `Blend`, `Premultiplied` and `Alpha` all share the same `BlendState`. Depending
// on the alpha mode, we premultiply the color channels by the alpha channel value,
// (and also optionally replace the alpha value with 0.0) so that the result produces
// the desired blend mode when sent to the blending operation.
#ifdef BLEND_PREMULTIPLIED_ALPHA
    // For `BlendState::PREMULTIPLIED_ALPHA_BLENDING` the blend function is:
    //
    //     result = 1 * src_color + (1 - src_alpha) * dst_color
    let alpha_mode = standard_material_flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD {
        // Here, we premultiply `src_color` by `src_alpha`, and replace `src_alpha` with 0.0:
        //
        //     src_color *= src_alpha
        //     src_alpha = 0.0
        //
        // We end up with:
        //
        //     result = 1 * (src_alpha * src_color) + (1 - 0) * dst_color
        //     result = src_alpha * src_color + 1 * dst_color
        //
        // Which is the blend operation for additive blending
        return vec4<f32>(color.rgb * color.a, 0.0);
    } else {
        // Here, we don't do anything, so that we get premultiplied alpha blending. (As expected)
        return color.rgba;
    }
#endif
// `Multiply` uses its own `BlendState`, but we still need to premultiply here in the
// shader so that we get correct results as we tweak the alpha channel
#ifdef BLEND_MULTIPLY
    // The blend function is:
    //
    //     result = dst_color * src_color + (1 - src_alpha) * dst_color
    //
    // We premultiply `src_color` by `src_alpha`:
    //
    //     src_color *= src_alpha
    //
    // We end up with:
    //
    //     result = dst_color * (src_color * src_alpha) + (1 - src_alpha) * dst_color
    //     result = src_alpha * (src_color * dst_color) + (1 - src_alpha) * dst_color
    //
    // Which is the blend operation for multiplicative blending with arbitrary mixing
    // controlled by the source alpha channel
    return vec4<f32>(color.rgb * color.a, color.a);
#endif
}
#endif

// fog, alpha premultiply
// for non-hdr cameras, tonemapping and debanding
fn main_pass_post_lighting_processing(
    pbr_input: pbr_types::PbrInput,
    input_color: vec4<f32>,
) -> vec4<f32> {
    var output_color = input_color;

    // fog
    if (view_bindings::fog.mode != mesh_view_types::FOG_MODE_OFF && (pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT) != 0u) {
        output_color = apply_fog(view_bindings::fog, output_color, pbr_input.world_position.xyz, view_bindings::view.world_position.xyz);
    }

#ifdef TONEMAP_IN_SHADER
    output_color = tone_mapping(output_color, view_bindings::view.color_grading);
#ifdef DEBAND_DITHER
    var output_rgb = output_color.rgb;
    output_rgb = powsafe(output_rgb, 1.0 / 2.2);
    output_rgb += screen_space_dither(pbr_input.frag_coord.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb, 2.2);
    output_color = vec4(output_rgb, output_color.a);
#endif
#endif
#ifdef PREMULTIPLY_ALPHA
    output_color = premultiply_alpha(pbr_input.material.flags, output_color);
#endif
    return output_color;
}
