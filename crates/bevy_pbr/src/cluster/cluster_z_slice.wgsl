#import bevy_pbr::cluster::{
    CLUSTERABLE_OBJECT_TYPE_DECAL, CLUSTERABLE_OBJECT_TYPE_IRRADIANCE_VOLUME,
    CLUSTERABLE_OBJECT_TYPE_POINT_LIGHT, CLUSTERABLE_OBJECT_TYPE_REFLECTION_PROBE,
    CLUSTERABLE_OBJECT_TYPE_SPOT_LIGHT, ClusterMetadata, ClusterableObjectZSlice,
    calculate_sphere_cluster_bounds, compute_view_from_world_scale
}
#import bevy_pbr::mesh_view_types::{
    ClusteredDecals, ClusteredLights, LightProbes, Lights, POINT_LIGHT_FLAGS_SPOT_LIGHT_BIT
}
#import bevy_render::view::View

// The shader that divides clusterable objects into Z slices.
//
// Treating the cluster froxel space as a grid of size WxHxD, for each
// clusterable object, we seek to rasterize D overlapping quads into a viewport
// of size WxH. Each quad represents a *Z slice* of the object. This shader
// calculates the number of Z slices needed for each object in parallel and
// prepares the `ClusterableObjectZSlice` data that the rasterization passes
// will consume. It also updates the indirect draw parameters for the
// rasterization pass and calculates statistics about the farthest Z value
// encountered that the CPU can later read back for dynamic froxel range tuning.

// Metadata, including the indirect draw parameters that we write to.
@group(0) @binding(0) var<storage, read_write> cluster_metadata: ClusterMetadata;
// The list of clusterable object Z slices that we write to.
@group(0) @binding(1) var<storage, read_write> z_slices: array<ClusterableObjectZSlice>;
// Information about each light.
@group(0) @binding(2) var<storage> clustered_lights: ClusteredLights;
// Information about each light probe (reflection probe or irradiance volume).
@group(0) @binding(3) var<uniform> light_probes: LightProbes;
// Information about each clustered decal.
@group(0) @binding(4) var<storage> clustered_decals: ClusteredDecals;
// Information about the clusters as a whole, including the dimensions of the
// cluster grid.
@group(0) @binding(5) var<uniform> lights: Lights;
// Information about the view.
@group(0) @binding(6) var<uniform> view: View;

// A temporary workgroup-local buffer used to accelerate the "farthest depth of
// any object" calculation.
var<workgroup> shared_farthest_z: array<f32, 64>;

// The shader entry point.
//
// We have one invocation per clusterable object.
@compute @workgroup_size(64, 1, 1)
fn z_slice_main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(local_invocation_id) local_invocation_id: vec3<u32>
) {
    let id = global_invocation_id.x;
    let local_id = local_invocation_id.x;

    var object_index: u32 = 0u;
    var object_type: u32 = 0xffffffffu;
    var position: vec3<f32> = vec3<f32>(0.0);
    var radius: f32 = 0.0;

    // Figure out what the bounds are for each type of clusterable object, in
    // preparation to determining which kind of object we're clustering.
    // In thread order, the threads are assigned to cluster clusterable lights,
    // reflection probes, irradiance volumes, and decals, in that order.
    // It might look like we should have done prefix sum on
    // `clustered_light_count`, `reflection_probe_count`, etc. to avoid all
    // this ID math. But that would make life harder for plugins that want to
    // add clustered objects in compute shaders, because they might have to
    // update multiple fields (atomically!) when adding, for example, a light.
    let last_clustered_light_id = cluster_metadata.clustered_light_count;
    let last_reflection_probe_id = last_clustered_light_id +
        cluster_metadata.reflection_probe_count;
    let last_irradiance_volume_id = last_reflection_probe_id +
        cluster_metadata.irradiance_volume_count;
    let last_decal_id = last_irradiance_volume_id + cluster_metadata.decal_count;

    // Figure out which type of object we are, and calculate our position and range.
    // We use a sphere to conservatively construct our AABB.
    if (id < last_clustered_light_id) {
        // We're a light (either point light or spot light).
        object_index = id;
        let flags = clustered_lights.data[object_index].flags;
        object_type = select(
            CLUSTERABLE_OBJECT_TYPE_POINT_LIGHT,
            CLUSTERABLE_OBJECT_TYPE_SPOT_LIGHT,
            (flags & POINT_LIGHT_FLAGS_SPOT_LIGHT_BIT) != 0u
        );
        position = clustered_lights.data[object_index].position_radius.xyz;
        radius = clustered_lights.data[object_index].range;
    } else if (id < last_reflection_probe_id) {
        // We're a reflection probe.
        object_index = id - last_clustered_light_id;
        object_type = CLUSTERABLE_OBJECT_TYPE_REFLECTION_PROBE;
        position = light_probes.reflection_probes[object_index].world_position;
        radius = light_probes.reflection_probes[object_index].bounding_sphere_radius;
    } else if (id < last_irradiance_volume_id) {
        // We're an irradiance volume.
        object_index = id - last_reflection_probe_id;
        object_type = CLUSTERABLE_OBJECT_TYPE_IRRADIANCE_VOLUME;
        position = light_probes.irradiance_volumes[object_index].world_position;
        radius = light_probes.irradiance_volumes[object_index].bounding_sphere_radius;
    } else if (id < last_decal_id) {
        // We're a clustered decal.
        object_index = id - last_irradiance_volume_id;
        object_type = CLUSTERABLE_OBJECT_TYPE_DECAL;
        position = clustered_decals.decals[object_index].world_position;
        radius = clustered_decals.decals[object_index].bounding_sphere_radius;
    }

    let view_from_world_scale = compute_view_from_world_scale(view.world_from_view);
    let is_orthographic = view.clip_from_view[3].w == 1.0;

    // Gather the farthest Z value among all clusters in this workgroup.
    // We want to do this *before* bailing out below so that all threads hit the
    // same workgroup barriers, which this function uses.
    accumulate_farthest_z_value(local_id, position, radius, view_from_world_scale, is_orthographic);

    // Bail out if we have no clusterable object to work on.
    if (object_type == 0xffffffffu) {
        return;
    }

    let cluster_bounds = calculate_sphere_cluster_bounds(
        position,
        radius,
        view.view_from_world,
        view.clip_from_view,
        view_from_world_scale,
        lights.cluster_dimensions.xyz,
        lights.cluster_factors.zw,
        is_orthographic
    );

    // Write out our Z slices.
    for (var z_slice = cluster_bounds.min.z; z_slice <= cluster_bounds.max.z; z_slice += 1u) {
        try_write_z_slice(object_index, object_type, z_slice);
    }
}

// Writes a Z slice to the list.
//
// This silently fails if the list is too small, but it still updates the
// instance count, which the CPU reads back. So, if the list is too small, the
// CPU will end up being notified and can resize the buffer.
fn try_write_z_slice(object_index: u32, object_type: u32, z_slice: u32) {
    let z_slice_offset = atomicAdd(&cluster_metadata.indirect_draw_params.instance_count, 1u);
    if (z_slice_offset >= cluster_metadata.z_slice_list_capacity) {
        return;
    }

    z_slices[z_slice_offset].object_index = object_index;
    z_slices[z_slice_offset].object_type = object_type;
    z_slices[z_slice_offset].z_slice = z_slice;
}

// Records the farthest Z value for clusterable objects in this workgroup for
// the CPU to read back.
fn accumulate_farthest_z_value(
    local_id: u32,
    position: vec3<f32>,
    radius: f32,
    view_from_world_scale: vec3<f32>,
    is_orthographic: bool
) {
    // Compute the maximum Z extent for our clusterable object.
    let view_from_world_row_2 = transpose(view.view_from_world)[2];
    let far_z = dot(-view_from_world_row_2, vec4(position, 1.0)) + radius * view_from_world_scale.z;
    shared_farthest_z[local_id] = far_z;
    workgroupBarrier();

    // Reduce in local memory to quickly find the maximum Z extent of the
    // objects in our workgroup.
    for (var stride = 32u; stride > 0u; stride /= 2u) {
        if (local_id < stride) {
            shared_farthest_z[local_id] = max(
                shared_farthest_z[local_id],
                shared_farthest_z[local_id + stride]
            );
        }
        workgroupBarrier();
    }

    // Only the first thread will continue.
    if (local_id != 0u) {
        return;
    }

    // Have the first thread update the global farthest-Z value.
    // We don't have `atomicMax` for floats in WGSL, so we use CAS instead.
    // Thankfully, we only have a few workgroups, so this shouldn't be terribly
    // slow.
    let this_farthest_z = shared_farthest_z[0u];
    var that_farthest_z = bitcast<f32>(atomicLoad(&cluster_metadata.farthest_z));
    while (this_farthest_z > that_farthest_z) {
        let exchange_result = atomicCompareExchangeWeak(
            &cluster_metadata.farthest_z,
            bitcast<u32>(that_farthest_z),
            bitcast<u32>(this_farthest_z)
        );
        if (exchange_result.exchanged) {
            break;
        }
        that_farthest_z = bitcast<f32>(exchange_result.old_value);
    }
}
