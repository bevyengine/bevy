#define_import_path bevy_pbr::clustered_forward

#import bevy_pbr::mesh_view_bindings as bindings
#import bevy_pbr::utils hsv2rgb

// NOTE: Keep in sync with bevy_pbr/src/light.rs
fn view_z_to_z_slice(view_z: f32, is_orthographic: bool) -> u32 {
    var z_slice: u32 = 0u;
    if (is_orthographic) {
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
fn unpack_offset_and_counts(cluster_index: u32) -> vec3<u32> {
#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3
        return bindings::cluster_offsets_and_counts.data[cluster_index].xyz;
#else
    let offset_and_counts = bindings::cluster_offsets_and_counts.data[cluster_index >> 2u][cluster_index & ((1u << 2u) - 1u)];
    //  [ 31     ..     18 | 17      ..      9 | 8       ..     0 ]
    //  [      offset      | point light count | spot light count ]
    return vec3<u32>(
        (offset_and_counts >> (CLUSTER_COUNT_SIZE * 2u)) & ((1u << (32u - (CLUSTER_COUNT_SIZE * 2u))) - 1u),
        (offset_and_counts >> CLUSTER_COUNT_SIZE)        & ((1u << CLUSTER_COUNT_SIZE) - 1u),
        offset_and_counts                                & ((1u << CLUSTER_COUNT_SIZE) - 1u),
    );
#endif
}

fn get_light_id(index: u32) -> u32 {
#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3
    return bindings::cluster_light_index_lists.data[index];
#else
    // The index is correct but in cluster_light_index_lists we pack 4 u8s into a u32
    // This means the index into cluster_light_index_lists is index / 4
    let indices = bindings::cluster_light_index_lists.data[index >> 4u][(index >> 2u) & ((1u << 2u) - 1u)];
    // And index % 4 gives the sub-index of the u8 within the u32 so we shift by 8 * sub-index
    return (indices >> (8u * (index & ((1u << 2u) - 1u)))) & ((1u << 8u) - 1u);
#endif
}

fn cluster_debug_visualization(
    output_color: vec4<f32>,
    view_z: f32,
    is_orthographic: bool,
    offset_and_counts: vec3<u32>,
    cluster_index: u32,
) -> vec4<f32> {
    // Cluster allocation debug (using 'over' alpha blending)
#ifdef CLUSTERED_FORWARD_DEBUG_Z_SLICES
    // NOTE: This debug mode visualises the z-slices
    let cluster_overlay_alpha = 0.1;
    var z_slice: u32 = view_z_to_z_slice(view_z, is_orthographic);
    // A hack to make the colors alternate a bit more
    if ((z_slice & 1u) == 1u) {
        z_slice = z_slice + bindings::lights.cluster_dimensions.z / 2u;
    }
    let slice_color = hsv2rgb(f32(z_slice) / f32(bindings::lights.cluster_dimensions.z + 1u), 1.0, 0.5);
    output_color = vec4<f32>(
        (1.0 - cluster_overlay_alpha) * output_color.rgb + cluster_overlay_alpha * slice_color,
        output_color.a
    );
#endif // CLUSTERED_FORWARD_DEBUG_Z_SLICES
#ifdef CLUSTERED_FORWARD_DEBUG_CLUSTER_LIGHT_COMPLEXITY
    // NOTE: This debug mode visualises the number of lights within the cluster that contains
    // the fragment. It shows a sort of lighting complexity measure.
    let cluster_overlay_alpha = 0.1;
    let max_light_complexity_per_cluster = 64.0;
    output_color.r = (1.0 - cluster_overlay_alpha) * output_color.r
        + cluster_overlay_alpha * smoothStep(0.0, max_light_complexity_per_cluster, f32(offset_and_counts[1] + offset_and_counts[2]));
    output_color.g = (1.0 - cluster_overlay_alpha) * output_color.g
        + cluster_overlay_alpha * (1.0 - smoothStep(0.0, max_light_complexity_per_cluster, f32(offset_and_counts[1] + offset_and_counts[2])));
#endif // CLUSTERED_FORWARD_DEBUG_CLUSTER_LIGHT_COMPLEXITY
#ifdef CLUSTERED_FORWARD_DEBUG_CLUSTER_COHERENCY
    // NOTE: Visualizes the cluster to which the fragment belongs
    let cluster_overlay_alpha = 0.1;
    let cluster_color = hsv2rgb(random1D(f32(cluster_index)), 1.0, 0.5);
    output_color = vec4<f32>(
        (1.0 - cluster_overlay_alpha) * output_color.rgb + cluster_overlay_alpha * cluster_color,
        output_color.a
    );
#endif // CLUSTERED_FORWARD_DEBUG_CLUSTER_COHERENCY

    return output_color;
}
