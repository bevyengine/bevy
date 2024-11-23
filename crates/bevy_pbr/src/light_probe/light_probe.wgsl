#define_import_path bevy_pbr::light_probe

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
};

fn transpose_affine_matrix(matrix: mat3x4<f32>) -> mat4x4<f32> {
    let matrix4x4 = mat4x4<f32>(
        matrix[0],
        matrix[1],
        matrix[2],
        vec4<f32>(0.0, 0.0, 0.0, 1.0));
    return transpose(matrix4x4);
}

// Searches for a light probe that contains the fragment.
//
// TODO: Interpolate between multiple light probes.
fn query_light_probe(
    world_position: vec3<f32>,
    is_irradiance_volume: bool,
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

