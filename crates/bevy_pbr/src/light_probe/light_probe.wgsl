#define_import_path bevy_pbr::light_probe

#import bevy_pbr::clustered_forward
#import bevy_pbr::clustered_forward::ClusterableObjectIndexRanges
#import bevy_pbr::mesh_view_bindings::light_probes
#import bevy_pbr::mesh_view_types::LightProbe

// The result of searching for a light probe.
struct LightProbeQueryResult {
    // The index of the light probe texture or textures in the binding array or
    // arrays.
    texture_index: i32,
    // A scale factor that's applied to the diffuse and specular light from the
    // light probe. This is in units of cd/m² (candela per square meter).
    intensity: f32,
    // Transform from world space to the light probe model space. In light probe
    // model space, the light probe is a 1×1×1 cube centered on the origin.
    light_from_world: mat4x4<f32>,
    // Whether this light probe contributes diffuse light to lightmapped meshes.
    affects_lightmapped_mesh_diffuse: bool,
};

fn transpose_affine_matrix(matrix: mat3x4<f32>) -> mat4x4<f32> {
    let matrix4x4 = mat4x4<f32>(
        matrix[0],
        matrix[1],
        matrix[2],
        vec4<f32>(0.0, 0.0, 0.0, 1.0));
    return transpose(matrix4x4);
}

#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3

// Searches for a light probe that contains the fragment.
//
// This is the version that's used when storage buffers are available and
// light probes are clustered.
//
// TODO: Interpolate between multiple light probes.
fn query_light_probe(
    world_position: vec3<f32>,
    is_irradiance_volume: bool,
    clusterable_object_index_ranges: ptr<function, ClusterableObjectIndexRanges>,
) -> LightProbeQueryResult {
    var result: LightProbeQueryResult;
    result.texture_index = -1;

    // Reflection probe indices are followed by irradiance volume indices in the
    // cluster index list. Use this fact to create our bracketing range of
    // indices.
    var start_offset: u32;
    var end_offset: u32;
    if is_irradiance_volume {
        start_offset = (*clusterable_object_index_ranges).first_irradiance_volume_index_offset;
        end_offset = (*clusterable_object_index_ranges).first_decal_offset;
    } else {
        start_offset = (*clusterable_object_index_ranges).first_reflection_probe_index_offset;
        end_offset = (*clusterable_object_index_ranges).first_irradiance_volume_index_offset;
    }

    for (var light_probe_index_offset: u32 = start_offset;
            light_probe_index_offset < end_offset && result.texture_index < 0;
            light_probe_index_offset += 1u) {
        let light_probe_index = i32(clustered_forward::get_clusterable_object_id(
            light_probe_index_offset));

        var light_probe: LightProbe;
        if is_irradiance_volume {
            light_probe = light_probes.irradiance_volumes[light_probe_index];
        } else {
            light_probe = light_probes.reflection_probes[light_probe_index];
        }

        // Unpack the inverse transform.
        let light_from_world =
            transpose_affine_matrix(light_probe.light_from_world_transposed);

        // Check to see if the transformed point is inside the unit cube
        // centered at the origin.
        let probe_space_pos = (light_from_world * vec4<f32>(world_position, 1.0f)).xyz;
        if (all(abs(probe_space_pos) <= vec3(0.5f))) {
            result.texture_index = light_probe.cubemap_index;
            result.intensity = light_probe.intensity;
            result.light_from_world = light_from_world;
            result.affects_lightmapped_mesh_diffuse =
                light_probe.affects_lightmapped_mesh_diffuse != 0u;
            break;
        }
    }

    return result;
}

#else   // AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3

// Searches for a light probe that contains the fragment.
//
// This is the version that's used when storage buffers aren't available and
// light probes aren't clustered. It simply does a brute force search of all
// light probes. Because platforms without sufficient SSBO bindings typically
// lack bindless shaders, there will usually only be one of each type of light
// probe present anyway.
fn query_light_probe(
    world_position: vec3<f32>,
    is_irradiance_volume: bool,
    clusterable_object_index_ranges: ptr<function, ClusterableObjectIndexRanges>,
) -> LightProbeQueryResult {
    var result: LightProbeQueryResult;
    result.texture_index = -1;

    var light_probe_count: i32;
    if is_irradiance_volume {
        light_probe_count = light_probes.irradiance_volume_count;
    } else {
        light_probe_count = light_probes.reflection_probe_count;
    }

    for (var light_probe_index: i32 = 0;
            light_probe_index < light_probe_count && result.texture_index < 0;
            light_probe_index += 1) {
        var light_probe: LightProbe;
        if is_irradiance_volume {
            light_probe = light_probes.irradiance_volumes[light_probe_index];
        } else {
            light_probe = light_probes.reflection_probes[light_probe_index];
        }

        // Unpack the inverse transform.
        let light_from_world =
            transpose_affine_matrix(light_probe.light_from_world_transposed);

        // Check to see if the transformed point is inside the unit cube
        // centered at the origin.
        let probe_space_pos = (light_from_world * vec4<f32>(world_position, 1.0f)).xyz;
        if (all(abs(probe_space_pos) <= vec3(0.5f))) {
            result.texture_index = light_probe.cubemap_index;
            result.intensity = light_probe.intensity;
            result.light_from_world = light_from_world;
            result.affects_lightmapped_mesh_diffuse =
                light_probe.affects_lightmapped_mesh_diffuse != 0u;

            // TODO: Workaround for ICE in DXC https://github.com/microsoft/DirectXShaderCompiler/issues/6183
            // We can't use `break` here because of the ICE.
            // So instead we rely on the fact that we set `result.texture_index`
            // above and check its value in the `for` loop header before
            // looping.
            // break;
        }
    }

    return result;
}

#endif  // AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3
