#define_import_path bevy_pbr::clustered_forward

#import bevy_pbr::{
    mesh_view_bindings as bindings,
    utils::rand_f,
}

#import bevy_render::{
   color_operations::hsv_to_rgb,
   maths::PI_2,
}

// Offsets within the `cluster_offsets_and_counts` buffer for a single cluster.
//
// These offsets must be monotonically nondecreasing. That is, indices are
// always sorted into the following order: point lights, spot lights, reflection
// probes, irradiance volumes.
struct ClusterableObjectIndexRanges {
    // The offset of the index of the first point light.
    first_point_light_index_offset: u32,
    // The offset of the index of the first spot light, which also terminates
    // the list of point lights.
    first_spot_light_index_offset: u32,
    // The offset of the index of the first reflection probe, which also
    // terminates the list of spot lights.
    first_reflection_probe_index_offset: u32,
    // The offset of the index of the first irradiance volumes, which also
    // terminates the list of reflection probes.
    first_irradiance_volume_index_offset: u32,
    first_decal_offset: u32,
    // One past the offset of the index of the final clusterable object for this
    // cluster.
    last_clusterable_object_index_offset: u32,
}

// NOTE: Keep in sync with bevy_pbr/src/light.rs
fn view_z_to_z_slice(view_z: f32, is_orthographic: bool) -> u32 {
    var z_slice: u32 = 0u;
    if is_orthographic {
        // NOTE: view_z is correct in the orthographic case
        z_slice = u32(floor((view_z - bindings::lights.cluster_factors.z) * bindings::lights.cluster_factors.w));
    } else {
        // NOTE: had to use -view_z to make it positive else log(negative) is nan
        z_slice = u32(log(-view_z) * bindings::lights.cluster_factors.z - bindings::lights.cluster_factors.w + 1.0);
    }
    // NOTE: We use min as we may limit the far z plane used for clustering to be closer than
    // the furthest thing being drawn. This means that we need to limit to the maximum cluster.
    return min(z_slice, bindings::lights.cluster_dimensions.z - 1u);
}

fn fragment_cluster_index(frag_coord: vec2<f32>, view_z: f32, is_orthographic: bool) -> u32 {
    let xy = vec2<u32>(floor((frag_coord - bindings::view.viewport.xy) * bindings::lights.cluster_factors.xy));
    let z_slice = view_z_to_z_slice(view_z, is_orthographic);
    // NOTE: Restricting cluster index to avoid undefined behavior when accessing uniform buffer
    // arrays based on the cluster index.
    return min(
        (xy.y * bindings::lights.cluster_dimensions.x + xy.x) * bindings::lights.cluster_dimensions.z + z_slice,
        bindings::lights.cluster_dimensions.w - 1u
    );
}

// this must match CLUSTER_COUNT_SIZE in light.rs
const CLUSTER_COUNT_SIZE = 9u;

// Returns the indices of clusterable objects belonging to the given cluster.
//
// Note that if fewer than 3 SSBO bindings are available (in WebGL 2,
// primarily), light probes aren't clustered, and therefore both light probe
// index ranges will be empty.
fn unpack_clusterable_object_index_ranges(cluster_index: u32) -> ClusterableObjectIndexRanges {
#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3

    let offset_and_counts_a = bindings::cluster_offsets_and_counts.data[cluster_index][0];
    let offset_and_counts_b = bindings::cluster_offsets_and_counts.data[cluster_index][1];

    // Sum up the counts to produce the range brackets.
    //
    // We could have stored the range brackets in `cluster_offsets_and_counts`
    // directly, but doing it this way makes the logic in this path more
    // consistent with the WebGL 2 path below.
    let point_light_offset = offset_and_counts_a.x;
    let spot_light_offset = point_light_offset + offset_and_counts_a.y;
    let reflection_probe_offset = spot_light_offset + offset_and_counts_a.z;
    let irradiance_volume_offset = reflection_probe_offset + offset_and_counts_a.w;
    let decal_offset = irradiance_volume_offset + offset_and_counts_b.x;
    let last_clusterable_offset = decal_offset + offset_and_counts_b.y;
    return ClusterableObjectIndexRanges(
        point_light_offset,
        spot_light_offset,
        reflection_probe_offset,
        irradiance_volume_offset,
        decal_offset,
        last_clusterable_offset
    );

#else   // AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3

    let raw_offset_and_counts = bindings::cluster_offsets_and_counts.data[cluster_index >> 2u][cluster_index & ((1u << 2u) - 1u)];
    //  [ 31     ..     18 | 17      ..      9 | 8       ..     0 ]
    //  [      offset      | point light count | spot light count ]
    let offset_and_counts = vec3<u32>(
        (raw_offset_and_counts >> (CLUSTER_COUNT_SIZE * 2u)) & ((1u << (32u - (CLUSTER_COUNT_SIZE * 2u))) - 1u),
        (raw_offset_and_counts >> CLUSTER_COUNT_SIZE)        & ((1u << CLUSTER_COUNT_SIZE) - 1u),
        raw_offset_and_counts                                & ((1u << CLUSTER_COUNT_SIZE) - 1u),
    );

    // We don't cluster reflection probes or irradiance volumes on this
    // platform, as there's no room in the UBO. Thus, those offset ranges
    // (corresponding to `offset_d` and `offset_e` above) are empty and are
    // simply copies of `offset_c`.

    let offset_a = offset_and_counts.x;
    let offset_b = offset_a + offset_and_counts.y;
    let offset_c = offset_b + offset_and_counts.z;

    return ClusterableObjectIndexRanges(offset_a, offset_b, offset_c, offset_c, offset_c, offset_c);

#endif  // AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3
}

// Returns the index of the clusterable object at the given offset.
//
// Note that, in the case of a light probe, the index refers to an element in
// one of the two `light_probes` sublists, not the `clusterable_objects` list.
fn get_clusterable_object_id(index: u32) -> u32 {
#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3
    return bindings::clusterable_object_index_lists.data[index];
#else
    // The index is correct but in clusterable_object_index_lists we pack 4 u8s into a u32
    // This means the index into clusterable_object_index_lists is index / 4
    let indices = bindings::clusterable_object_index_lists.data[index >> 4u][(index >> 2u) &
        ((1u << 2u) - 1u)];
    // And index % 4 gives the sub-index of the u8 within the u32 so we shift by 8 * sub-index
    return (indices >> (8u * (index & ((1u << 2u) - 1u)))) & ((1u << 8u) - 1u);
#endif
}

fn cluster_debug_visualization(
    input_color: vec4<f32>,
    view_z: f32,
    is_orthographic: bool,
    clusterable_object_index_ranges: ClusterableObjectIndexRanges,
    cluster_index: u32,
) -> vec4<f32> {
    var output_color = input_color;

    // Cluster allocation debug (using 'over' alpha blending)
#ifdef CLUSTERED_FORWARD_DEBUG_Z_SLICES
    // NOTE: This debug mode visualizes the z-slices
    let cluster_overlay_alpha = 0.1;
    var z_slice: u32 = view_z_to_z_slice(view_z, is_orthographic);
    // A hack to make the colors alternate a bit more
    if (z_slice & 1u) == 1u {
        z_slice = z_slice + bindings::lights.cluster_dimensions.z / 2u;
    }
    let slice_color_hsv = vec3(
        f32(z_slice) / f32(bindings::lights.cluster_dimensions.z + 1u) * PI_2,
        1.0,
        0.5
    );
    let slice_color = hsv_to_rgb(slice_color_hsv);
    output_color = vec4<f32>(
        (1.0 - cluster_overlay_alpha) * output_color.rgb + cluster_overlay_alpha * slice_color,
        output_color.a
    );
#endif // CLUSTERED_FORWARD_DEBUG_Z_SLICES
#ifdef CLUSTERED_FORWARD_DEBUG_CLUSTER_COMPLEXITY
    // NOTE: This debug mode visualizes the number of clusterable objects within
    // the cluster that contains the fragment. It shows a sort of cluster
    // complexity measure.
    let cluster_overlay_alpha = 0.1;
    let max_complexity_per_cluster = 64.0;
    let object_count = clusterable_object_index_ranges.first_reflection_probe_index_offset -
        clusterable_object_index_ranges.first_point_light_index_offset;
    output_color.r = (1.0 - cluster_overlay_alpha) * output_color.r + cluster_overlay_alpha *
        smoothstep(0.0, max_complexity_per_cluster, f32(object_count));
    output_color.g = (1.0 - cluster_overlay_alpha) * output_color.g + cluster_overlay_alpha *
        (1.0 - smoothstep(0.0, max_complexity_per_cluster, f32(object_count)));
#endif // CLUSTERED_FORWARD_DEBUG_CLUSTER_COMPLEXITY
#ifdef CLUSTERED_FORWARD_DEBUG_CLUSTER_COHERENCY
    // NOTE: Visualizes the cluster to which the fragment belongs
    let cluster_overlay_alpha = 0.1;
    var rng = cluster_index;
    let cluster_color_hsv = vec3(rand_f(&rng) * PI_2, 1.0, 0.5);
    let cluster_color = hsv_to_rgb(cluster_color_hsv);
    output_color = vec4<f32>(
        (1.0 - cluster_overlay_alpha) * output_color.rgb + cluster_overlay_alpha * cluster_color,
        output_color.a
    );
#endif // CLUSTERED_FORWARD_DEBUG_CLUSTER_COHERENCY

    return output_color;
}
