#define_import_path bevy_pbr::light_probe

#import bevy_pbr::clustered_forward
#import bevy_pbr::clustered_forward::ClusterableObjectIndexRanges
#import bevy_pbr::mesh_view_bindings::light_probes
#import bevy_pbr::mesh_view_types::{
    LightProbe, LIGHT_PROBE_FLAG_AFFECTS_LIGHTMAPPED_MESH_DIFFUSE,
    LIGHT_PROBE_FLAG_PARALLAX_CORRECT
}

// The result of searching for a light probe.
//
// Light probe iterators yield values of this type. Note that multiple light
// probes can affect a single fragment.
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
    // The weight of this light probe, determined by the position of the
    // fragment within the falloff range. The sum of the weights of all light
    // probes affecting a fragment need not be 1.
    weight: f32,
    // The flags that the light probe has: a combination of
    // `LIGHT_PROBE_FLAG_*`.
    flags: u32,
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

// A type that allows iterating through the list of light probes that overlap
// the current fragment.
//
// This is the version used when light probes are clustered.
struct LightProbeIterator {
    // The current offset in the light probes list.
    current_offset: u32,
    // The last offset in the light probes list.
    end_offset: u32,
    // The world-space position of the current fragment.
    world_position: vec3<f32>,
    // True if we're searching for an irradiance volume; false if we're
    // searching for a reflection probe.
    is_irradiance_volume: bool,
}

// Creates a new light probe iterator ready to iterate through light probes in
// the froxel containing the `world_position`.
fn light_probe_iterator_new(
    world_position: vec3<f32>,
    is_irradiance_volume: bool,
    clusterable_object_index_ranges: ptr<function, ClusterableObjectIndexRanges>,
) -> LightProbeIterator {
    // Reflection probe indices are followed by irradiance volume indices in the
    // cluster index list. Use this fact to create our bracketing range of
    // indices.

    if is_irradiance_volume {
        return LightProbeIterator(
            (*clusterable_object_index_ranges).first_irradiance_volume_index_offset,
            (*clusterable_object_index_ranges).first_decal_offset,
            world_position,
            true
        );
    }

    return LightProbeIterator(
        (*clusterable_object_index_ranges).first_reflection_probe_index_offset,
        (*clusterable_object_index_ranges).first_irradiance_volume_index_offset,
        world_position,
        false
    );
}

// Searches for a light probe that contains the fragment and returns the next
// such probe.
//
// Note that multiple light probes can affect a fragment. The caller is
// generally expected to blend their influences together in a weighted sum.
fn light_probe_iterator_next(iterator: ptr<function, LightProbeIterator>) -> LightProbeQueryResult {
    let world_position = (*iterator).world_position;

    var result: LightProbeQueryResult;
    result.texture_index = -1;
    result.weight = 0.0;

    while ((*iterator).current_offset < (*iterator).end_offset) {
        let light_probe_index = i32(clustered_forward::get_clusterable_object_id(
            (*iterator).current_offset));
        (*iterator).current_offset += 1u;

        // FIXME: This happens when one or more images for the light probe
        // aren't loaded yet. Really, though, we shouldn't be clustering such
        // objects at all.
        if (light_probe_index < 0) {
            continue;
        }

        var light_probe: LightProbe;
        if (*iterator).is_irradiance_volume {
            light_probe = light_probes.irradiance_volumes[light_probe_index];
        } else {
            light_probe = light_probes.reflection_probes[light_probe_index];
        }

        // Unpack the inverse transform.
        let light_from_world =
            transpose_affine_matrix(light_probe.light_from_world_transposed);

        // Transform the point into local space, with the cube edges at ±0.5 on
        // each axis.
        let probe_space_pos = (light_from_world * vec4<f32>(world_position, 1.0f)).xyz;
        // Avoid division by zero.
        let falloff = max(light_probe.falloff, vec3(0.0001));
        // Calculate the per-axis weight by doing a linear ramp from 0.0 at the
        // inside of the falloff region to 1.0 at the outside of the falloff
        // region.
        let axis_weights = saturate((1.0 - 2.0 * abs(probe_space_pos)) / (2.0 * falloff));
        // The actual weight is the minimum of all the per-axis weights.
        let weight = min(min(axis_weights.x, axis_weights.y), axis_weights.z);
        // If the resulting weight is zero, we're outside the light probe
        // entirely. Bail.
        if (weight == 0.0) {
            continue;
        }

        result.texture_index = light_probe.cubemap_index;
        result.intensity = light_probe.intensity;
        result.light_from_world = light_from_world;
        result.flags = light_probe.flags;
        result.weight = weight;
        return result;
    }

    return result;
}

#else   // AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3

// A type that allows iterating through the list of light probes that overlap
// the current fragment.
//
// This is the version that's used when sufficient storage buffers aren't
// available and consequently when light probes aren't clustered. It simply does
// a brute force search of all light probes. Because platforms without
// sufficient SSBO bindings typically lack bindless shaders, there will usually
// only be one of each type of light probe present anyway.
struct LightProbeIterator {
    // The current index in the list of light probes for this cluster.
    current_index: u32,
    // The last index in the list.
    end_index: u32,
    // The position of the current fragment.
    world_position: vec3<f32>,
    // True if we're searching for irradiance volumes; false if we're searching
    // for reflection probes.
    is_irradiance_volume: bool,
}

// Creates a new light probe iterator ready to search through light probes.
fn light_probe_iterator_new(
    world_position: vec3<f32>,
    is_irradiance_volume: bool,
    clusterable_object_index_ranges: ptr<function, ClusterableObjectIndexRanges>,
) -> LightProbeIterator {
    return LightProbeIterator(
        0,
        select(
            light_probes.reflection_probe_count,
            light_probes.irradiance_volume_count,
            is_irradiance_volume
        ),
        world_position,
        is_irradiance_volume
    );
}

// Searches for a light probe that contains the fragment and returns the next
// such probe.
//
// Note that, theoretically, multiple light probes can affect a fragment, and
// the caller is generally expected to blend their influences together in a
// weighted sum. In practice, this version of `light_probe_iterator_next` is
// used on platforms that lack bindless shaders, so there will only be at most
// one light probe that affects the current fragment in the first place.
fn light_probe_iterator_next(iterator: ptr<function, LightProbeIterator>) -> LightProbeQueryResult {
    var result: LightProbeQueryResult;
    result.texture_index = -1;
    result.weight = 0.0;

    while (true) {
        let light_probe_index = (*iterator).current_index;

        var light_probe: LightProbe;
        if is_irradiance_volume {
            light_probe = light_probes.irradiance_volumes[light_probe_index];
        } else {
            light_probe = light_probes.reflection_probes[light_probe_index];
        }

        // Unpack the inverse transform.
        let light_from_world =
            transpose_affine_matrix(light_probe.light_from_world_transposed);

        // Transform the point into local space, with the cube edges at ±0.5 on
        // each axis.
        let probe_space_pos = (light_from_world * vec4<f32>(world_position, 1.0f)).xyz;
        // Avoid division by zero.
        let falloff = max(light_probe.falloff, vec3(0.0001));
        // Calculate the per-axis weight by doing a linear ramp from 0.0 at the
        // inside of the falloff region to 1.0 at the outside of the falloff
        // region.
        let axis_weights = saturate((1.0 - 2.0 * abs(probe_space_pos)) / (2.0 * falloff));
        // The actual weight is the minimum of all the per-axis weights.
        let weight = min(min(axis_weights.x, axis_weights.y), axis_weights.z);
        // If the resulting weight is zero, we're outside the light probe
        // entirely. Bail.
        if (weight == 0.0) {
            continue;
        }

        result.texture_index = light_probe.cubemap_index;
        result.intensity = light_probe.intensity;
        result.light_from_world = light_from_world;
        result.flags = light_probe.flags;
        result.weight = weight;
        return result;
    }

    return result;
}

#endif  // AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3
