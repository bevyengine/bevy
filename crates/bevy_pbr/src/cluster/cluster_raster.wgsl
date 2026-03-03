#import bevy_pbr::cluster::{
    Aabb, CLUSTERABLE_OBJECT_TYPE_DECAL, CLUSTERABLE_OBJECT_TYPE_IRRADIANCE_VOLUME,
    CLUSTERABLE_OBJECT_TYPE_POINT_LIGHT, CLUSTERABLE_OBJECT_TYPE_REFLECTION_PROBE,
    CLUSTERABLE_OBJECT_TYPE_SPOT_LIGHT, ClusterableObjectZSlice,
    calculate_sphere_cluster_bounds, compute_view_from_world_scale
}
#import bevy_pbr::clustered_forward
#import bevy_pbr::light_probes::transpose_affine_matrix
#import bevy_pbr::mesh_view_types::{
    ClusterOffsetsAndCounts, ClusterableObjectIndexLists, ClusteredDecals, ClusteredLights,
    LightProbes, Lights, POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE
}
#import bevy_render::view::View

// The shader that performs the cluster-object intersection tests and assigns
// objects to clusters as appropriate.
//
// Assuming the froxel grid has size WxHxD, this shader is expected to run on a
// viewport of size WxH. It draws each *Z slice* as an axis aligned quad such
// that each fragment shader invocation represents a single cluster-object pair
// in the froxel grid. The fragment shader performs a finer test to see if the
// object might intersects the cluster and, if it succeeds, records the result.
// Because the result is written to storage buffers, color writes are disabled
// for this shader invocation.
//
// This shader runs twice: once to accumulate the *count* of each object type in
// each cluster, and once to *populate* the actual IDs. Both invocations of the
// shader must compute the exact same visibility results.

struct Vertex {
    @builtin(instance_index) instance_id: u32,
    @location(0) position: vec2<f32>,
}

// Data output from the vertex shader and input to the fragment shader.
struct Varyings {
    @builtin(position) position: vec4<f32>,
    // The index of the Z slice we're rasterizing.
    @location(0) @interpolate(flat) instance_id: u32,
    // The view-space center of the bounding sphere of the object.
    @location(1) @interpolate(flat) sphere_position: vec3<f32>,
    // The view-space radius of the bounding sphere of the object.
    @location(2) @interpolate(flat) sphere_radius: f32,
}

// The same as the `ClusterOffsetsAndCounts` structure, but with atomic fields
// so that we can write to it.
struct ClusterOffsetsAndCountsAtomic {
    data: array<ClusterOffsetsAndCountsElementAtomic>,
}

// The same as the `ClusterOffsetsAndCountsElement` structure, but with atomic
// fields so that we can write to it.
struct ClusterOffsetsAndCountsElementAtomic {
    offset: atomic<u32>,
    point_lights: atomic<u32>,
    spot_lights: atomic<u32>,
    reflection_probes: atomic<u32>,
    irradiance_volumes: atomic<u32>,
    decals: atomic<u32>,
    pad_a: u32,
    pad_b: u32,
}

// The list of clusterable object Z slices that we read from to.
@group(0) @binding(0) var<storage> z_slices: array<ClusterableObjectZSlice>;
// The list of indices per cluster that we write to in the populate pass.
@group(0) @binding(1) var<storage, read_write> index_lists: ClusterableObjectIndexLists;
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
#ifdef POPULATE_PASS
// The number of objects in each cluster, and the offset of each list.
@group(0) @binding(7) var<storage> offsets_and_counts: ClusterOffsetsAndCounts;
// For each cluster, the counts of objects written *so far* to it.
//
// We use this during the populate phase in order to write the ID of each object
// to the correct spot.
@group(0) @binding(8) var<storage, read_write> scratchpad_offsets_and_counts:
    ClusterOffsetsAndCountsAtomic;
#else   // POPULATE_PASS
// The number of objects in each cluster.
//
// During the count pass, we write to this.
@group(0) @binding(7) var<storage, read_write> offsets_and_counts: ClusterOffsetsAndCountsAtomic;
#endif  // POPULATE_PASS

// The vertex entry point.
@vertex
fn vertex_main(vertex: Vertex) -> Varyings {
    let instance_id = vertex.instance_id;
    let object_index = z_slices[instance_id].object_index;
    let object_type = z_slices[instance_id].object_type;

    // Look up the world space bounding sphere of the object.
    let bounding_sphere = get_object_bounding_sphere(object_index, object_type);
    let position = bounding_sphere.xyz;
    let radius = bounding_sphere.w;

    let view_from_world_scale = compute_view_from_world_scale(view.world_from_view);
    let max_view_from_world_scale = max(view_from_world_scale.x,
        max(view_from_world_scale.y, view_from_world_scale.z));
    let is_orthographic = view.clip_from_view[3].w == 1.0;

    // Calculate an approximate AABB of the cluster by computing its bounding
    // sphere and converting that to the AABB.
    // It's possible to do better, as the CPU version of
    // `assign_objects_to_clusters` does with its *iterative sphere refinement*
    // algorithm. However, that's sequential. I believe that this simple
    // approach is the standard way to do cluster assignment on GPU.
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
    let cluster_bounds_xy = vec4<u32>(cluster_bounds.min.xy, cluster_bounds.max.xy + vec2<u32>(1u));

    // Calculate the bounding sphere's center and radius in view space.
    let view_position = (view.view_from_world * vec4(position, 1.0)).xyz;
    let view_radius = max_view_from_world_scale * radius;

    return Varyings(
        calculate_vertex_position(vertex, cluster_bounds_xy),
        instance_id,
        view_position,
        view_radius
    );
}

// Returns the position of the quad vertex necessary to enclose all the
// fragments that represent the cluster AABB.
// The cluster bounds are supplied as `vec4(min X, min Y, max X, max Y)`.
fn calculate_vertex_position(vertex: Vertex, cluster_bounds: vec4<u32>) -> vec4<f32> {
    let framebuffer_position = vec2<u32>(
        select(cluster_bounds.x, cluster_bounds.z, vertex.position.x == 1.0),
        select(cluster_bounds.y, cluster_bounds.w, vertex.position.y == 1.0)
    );
    let vertex_position =
        vec2<f32>(framebuffer_position) / vec2<f32>(lights.cluster_dimensions.xy);
    return vec4(mix(vec2(-1.0, 1.0), vec2(1.0, -1.0), vertex_position), 0.0, 1.0);
}

// Performs a fine-grained test to ensure that the object intersects a single
// froxel and records the result.
@fragment
fn fragment_main(varyings: Varyings) -> @location(0) vec4<f32> {
    let instance_id = varyings.instance_id;
    let object_index = z_slices[instance_id].object_index;
    let object_type = z_slices[instance_id].object_type;
    let z_slice = z_slices[instance_id].z_slice;

    let is_orthographic = view.clip_from_view[3].w == 1.0;
    let screen_size = view.viewport.zw;
    let tile_size = screen_size / vec2<f32>(lights.cluster_dimensions.xy);

    let z_near_far = compute_z_near_and_z_far(is_orthographic);
    let z_near = z_near_far.x;
    let z_far = z_near_far.y;

    // Determine the AABB of the cluster this fragment represents.
    let cluster_position = vec3<u32>(vec2<u32>(floor(varyings.position.xy)), z_slice);
    let cluster_aabb = compute_aabb_for_cluster(
        z_near,
        z_far,
        tile_size,
        screen_size,
        view.view_from_clip,
        is_orthographic,
        lights.cluster_dimensions.xyz,
        cluster_position
    );
    let cluster_aabb_center = (cluster_aabb.max + cluster_aabb.min) * 0.5;
    let cluster_aabb_half_size = (cluster_aabb.max - cluster_aabb.min) * 0.5;

    // See if the object sphere intersects the AABB. If it doesn't, cull the
    // object.
    let object_intersects_cluster_aabb = sphere_intersects_aabb(
        varyings.sphere_position,
        varyings.sphere_radius,
        cluster_aabb_center,
        cluster_aabb_half_size
    );
    if (!object_intersects_cluster_aabb) {
        return vec4<f32>(0.0);
    }

    // Do further, more precise culling for spot lights.
    if (object_type == CLUSTERABLE_OBJECT_TYPE_SPOT_LIGHT && cull_spot_light(
        object_index,
        cluster_aabb_center,
        length(cluster_aabb_half_size),
        varyings.sphere_position,
        varyings.sphere_radius
    )) {
        return vec4<f32>(0.0);
    }

    let cluster_index =
        clustered_forward::fragment_cluster_index(cluster_position, lights.cluster_dimensions);

    // If this is the populate pass, reserve a slot and write in the actual
    // object index. Otherwise, if this is the count pass, just bump the
    // appropriate counter.
#ifdef POPULATE_PASS
    let output_index = allocate_list_entry(cluster_index, object_type);
    if (output_index < arrayLength(&index_lists.data)) {
        index_lists.data[output_index] = object_index;
    }
#else   // POPULATE_PASS
    increment_object_count(cluster_index, object_type);
#endif  // POPULATE_PASS

    return vec4<f32>(0.0);
}

// Returns true if the given sphere intersects the AABB with the given
// boundaries.
fn sphere_intersects_aabb(
    sphere_center: vec3<f32>,
    sphere_radius: f32,
    aabb_center: vec3<f32>,
    aabb_half_size: vec3<f32>
) -> bool {
    let delta = max(vec3(0.0), abs(aabb_center - sphere_center) - aabb_half_size);
    let dist_sq = dot(delta, delta);
    return dist_sq <= sphere_radius * sphere_radius;
}

// See `bevy_light::cluster::assign::compute_aabb_for_cluster`.
fn compute_aabb_for_cluster(
    z_near: f32,
    z_far: f32,
    tile_size: vec2<f32>,
    screen_size: vec2<f32>,
    view_from_clip: mat4x4<f32>,
    is_orthographic: bool,
    cluster_dimensions: vec3<u32>,
    ijk_u: vec3<u32>
) -> Aabb {
    let ijk = vec3<f32>(ijk_u);

    // Calculate the minimum and maximum points in screen space
    let p_min = ijk.xy * tile_size;
    let p_max = p_min + tile_size;

    var cluster_min: vec3<f32>;
    var cluster_max: vec3<f32>;
    if (is_orthographic) {
        // Use linear depth slicing for orthographic

        // Convert to view space at the cluster near and far planes
        // NOTE: 1.0 is the near plane due to using reverse z projections
        var p_min = screen_to_view(screen_size, view_from_clip, p_min, 0.0).xyz;
        var p_max = screen_to_view(screen_size, view_from_clip, p_max, 0.0).xyz;

        // calculate cluster depth using z_near and z_far
        p_min.z = -z_near + (z_near - z_far) * ijk.z / f32(cluster_dimensions.z);
        p_max.z = -z_near + (z_near - z_far) * (ijk.z + 1.0) / f32(cluster_dimensions.z);

        cluster_min = min(p_min, p_max);
        cluster_max = max(p_min, p_max);
    } else {
        // Convert to view space at the near plane
        // NOTE: 1.0 is the near plane due to using reverse z projections
        let p_min = screen_to_view(screen_size, view_from_clip, p_min, 1.0);
        let p_max = screen_to_view(screen_size, view_from_clip, p_max, 1.0);

        let z_far_over_z_near = -z_far / -z_near;
        var cluster_near = 0.0;
        if (ijk.z != 0.0) {
            cluster_near = -z_near *
                pow(z_far_over_z_near, (ijk.z - 1.0) / f32(cluster_dimensions.z - 1u));
        }
        // NOTE: This could be simplified to:
        // cluster_far = cluster_near * z_far_over_z_near;
        var cluster_far: f32;
        if (cluster_dimensions.z == 1u) {
            cluster_far = -z_far;
        } else {
            cluster_far = -z_near * pow(z_far_over_z_near, ijk.z / f32(cluster_dimensions.z - 1u));
        }

        // Calculate the four intersection points of the min and max points with the cluster near and far planes
        let p_min_near = line_intersection_to_z_plane(vec3(0.0), p_min.xyz, cluster_near);
        let p_min_far = line_intersection_to_z_plane(vec3(0.0), p_min.xyz, cluster_far);
        let p_max_near = line_intersection_to_z_plane(vec3(0.0), p_max.xyz, cluster_near);
        let p_max_far = line_intersection_to_z_plane(vec3(0.0), p_max.xyz, cluster_far);

        cluster_min = min(min(p_min_near, p_min_far), min(p_max_near, p_max_far));
        cluster_max = max(max(p_min_near, p_min_far), max(p_max_near, p_max_far));
    }

    return Aabb(cluster_min, cluster_max);
}

// Converts a screen-space position to a view-space position.
// See `bevy_light::cluster::assign::screen_to_view`.
fn screen_to_view(
    screen_size: vec2<f32>,
    view_from_clip: mat4x4<f32>,
    screen: vec2<f32>,
    ndc_z: f32
) -> vec4<f32> {
    let tex_coord = screen / screen_size;
    let clip = vec4(
        tex_coord.x * 2.0 - 1.0,
        (1.0 - tex_coord.y) * 2.0 - 1.0,
        ndc_z,
        1.0
    );
    return clip_to_view(view_from_clip, clip);
}

// Converts a clip-space position to a view-space position.
// See `bevy_light::cluster::assign::clip_to_view`.
fn clip_to_view(view_from_clip: mat4x4<f32>, clip: vec4<f32>) -> vec4<f32> {
    let view = view_from_clip * clip;
    return view / view.w;
}

// Calculate the intersection of a ray from the eye through the view space
// position to a z plane
// See `bevy_light::cluster::assign::line_intersection_to_z_plane`.
fn line_intersection_to_z_plane(origin: vec3<f32>, p: vec3<f32>, z: f32) -> vec3<f32> {
    let v = p - origin;
    let t = (z - dot(vec3(0.0, 0.0, 1.0), origin)) / dot(vec3(0.0, 0.0, 1.0), v);
    return origin + t * v;
}

// Computes the near and far extents of the cluster grid.
fn compute_z_near_and_z_far(is_orthographic: bool) -> vec2<f32> {
    let cluster_dimensions = vec3<f32>(lights.cluster_dimensions.xyz);
    let cluster_factors = lights.cluster_factors;
    var z_near: f32;
    var z_far: f32;
    if (is_orthographic) {
        z_near = -cluster_factors.z;
        z_far = -(cluster_dimensions.z + cluster_factors.w * cluster_factors.z) / cluster_factors.w;
    } else {
        z_near = exp(cluster_factors.w / cluster_factors.z);
        z_far = exp((cluster_dimensions.z + cluster_factors.w - 1.0) / cluster_factors.z);
    }
    return vec2<f32>(z_near, z_far);
}

// Returns true if a spot light should be culled.
// See `assign_objects_to_clusters` in `bevy_light/src/cluster/assign.rs`.
fn cull_spot_light(
    object_index: u32,
    cluster_aabb_sphere_center: vec3<f32>,
    cluster_aabb_sphere_radius: f32,
    sphere_position: vec3<f32>,
    sphere_radius: f32
) -> bool {
    let light_custom_data = clustered_lights.data[object_index].light_custom_data;
    let light_flags = clustered_lights.data[object_index].flags;
    let light_tan_angle = clustered_lights.data[object_index].spot_light_tan_angle;

    // `spot_light_dir_sin_cos` in `assign_objects_to_clusters` uses
    // `normalize(view_from_world *
    // vec4(clusterable_object.transform.back(), 0.0).xyz)`.
    // What we have is the XZ value of
    // `clusterable_object.transform.forward()`. So we have to first
    // calculate the missing Y value, then flip it to go from forward to
    // back, then transform by the view-from-world matrix to get
    // `view_light_direction`.
    let world_light_direction_rev_xz = light_custom_data.xy;
    let world_light_direction_rev_y_sign = select(
        1.0,
        -1.0,
        (light_flags & POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u
    );
    let world_light_direction_rev = vec3(
        world_light_direction_rev_xz.x,
        world_light_direction_rev_y_sign * sqrt(
            1.0 -
                world_light_direction_rev_xz.x * world_light_direction_rev_xz.x -
                world_light_direction_rev_xz.y * world_light_direction_rev_xz.y
        ),
        world_light_direction_rev_xz.y
    );
    let world_light_direction = -world_light_direction_rev;
    let view_light_direction = normalize((view.view_from_world *
        vec4(world_light_direction, 0.0)).xyz);

    let angle_cos = cos_atan(light_tan_angle);
    let angle_sin = sin_atan(light_tan_angle);

    // test -- based on https://bartwronski.com/2017/04/13/cull-that-cone/
    let spot_light_offset = sphere_position - cluster_aabb_sphere_center;
    let spot_light_dist_sq = dot(spot_light_offset, spot_light_offset);
    let v1_len = dot(spot_light_offset, view_light_direction);

    let distance_closest_point = angle_cos * sqrt(spot_light_dist_sq - v1_len * v1_len) -
        v1_len * angle_sin;
    let angle_cull = distance_closest_point > cluster_aabb_sphere_radius;

    let front_cull = v1_len > cluster_aabb_sphere_radius + sphere_radius;
    let back_cull = v1_len < -cluster_aabb_sphere_radius;

    return angle_cull || front_cull || back_cull;
}

// Computes `cos(atan(x))` cheaply.
// See https://en.wikipedia.org/wiki/List_of_trigonometric_identities
fn cos_atan(tan_theta: f32) -> f32 {
    return inverseSqrt(1.0 + tan_theta * tan_theta);
}

// Computes `sin(atan(x))` cheaply.
// See https://en.wikipedia.org/wiki/List_of_trigonometric_identities
fn sin_atan(tan_theta: f32) -> f32 {
    return tan_theta * inverseSqrt(1.0 + tan_theta * tan_theta);
}

#ifdef POPULATE_PASS
// Allocates space in the appropriate list and returns the global index that the
// object index should be written to.
fn allocate_list_entry(cluster_index: u32, object_type: u32) -> u32 {
    switch (object_type) {
        case CLUSTERABLE_OBJECT_TYPE_POINT_LIGHT: {
            return offsets_and_counts.data[cluster_index][0u].x +
                atomicAdd(&scratchpad_offsets_and_counts.data[cluster_index].point_lights, 1u);
        }
        case CLUSTERABLE_OBJECT_TYPE_SPOT_LIGHT: {
            return offsets_and_counts.data[cluster_index][0u].x +
                offsets_and_counts.data[cluster_index][0u].y +
                atomicAdd(&scratchpad_offsets_and_counts.data[cluster_index].spot_lights, 1u);
        }
        case CLUSTERABLE_OBJECT_TYPE_REFLECTION_PROBE: {
            return offsets_and_counts.data[cluster_index][0u].x +
                offsets_and_counts.data[cluster_index][0u].y +
                offsets_and_counts.data[cluster_index][0u].z +
                atomicAdd(&scratchpad_offsets_and_counts.data[cluster_index].reflection_probes, 1u);
        }
        case CLUSTERABLE_OBJECT_TYPE_IRRADIANCE_VOLUME: {
            return offsets_and_counts.data[cluster_index][0u].x +
                offsets_and_counts.data[cluster_index][0u].y +
                offsets_and_counts.data[cluster_index][0u].z +
                offsets_and_counts.data[cluster_index][0u].w +
                atomicAdd(
                    &scratchpad_offsets_and_counts.data[cluster_index].irradiance_volumes,
                    1u
                );
        }
        case CLUSTERABLE_OBJECT_TYPE_DECAL: {
            return offsets_and_counts.data[cluster_index][0u].x +
                offsets_and_counts.data[cluster_index][0u].y +
                offsets_and_counts.data[cluster_index][0u].z +
                offsets_and_counts.data[cluster_index][0u].w +
                offsets_and_counts.data[cluster_index][1u].x +
                atomicAdd(&scratchpad_offsets_and_counts.data[cluster_index].decals, 1u);
        }
        default: {}
    }
    return 0xffffffffu;
}
#else   // POPULATE_PASS
// Increments the count of objects of the given type for the given cluster.
fn increment_object_count(cluster_index: u32, object_type: u32) {
    switch (object_type) {
        case CLUSTERABLE_OBJECT_TYPE_POINT_LIGHT: {
            atomicAdd(&offsets_and_counts.data[cluster_index].point_lights, 1u);
        }
        case CLUSTERABLE_OBJECT_TYPE_SPOT_LIGHT: {
            atomicAdd(&offsets_and_counts.data[cluster_index].spot_lights, 1u);
        }
        case CLUSTERABLE_OBJECT_TYPE_REFLECTION_PROBE: {
            atomicAdd(&offsets_and_counts.data[cluster_index].reflection_probes, 1u);
        }
        case CLUSTERABLE_OBJECT_TYPE_IRRADIANCE_VOLUME: {
            atomicAdd(&offsets_and_counts.data[cluster_index].irradiance_volumes, 1u);
        }
        case CLUSTERABLE_OBJECT_TYPE_DECAL: {
            atomicAdd(&offsets_and_counts.data[cluster_index].decals, 1u);
        }
        default: {}
    }
}
#endif  // POPULATE_PASS

// Looks up and returns the world-space center and radius of the bounding sphere
// for the object with the given index and type.
// Returns a 4-vector with the fields `vec4(center X, center Y, center Z, radius)`.
fn get_object_bounding_sphere(object_index: u32, object_type: u32) -> vec4<f32> {
    var position = vec3<f32>(0.0);
    var radius = 0.0;
    switch (object_type) {
        case CLUSTERABLE_OBJECT_TYPE_POINT_LIGHT: {
            position = clustered_lights.data[object_index].position_radius.xyz;
            radius = clustered_lights.data[object_index].range;
        }
        case CLUSTERABLE_OBJECT_TYPE_SPOT_LIGHT: {
            position = clustered_lights.data[object_index].position_radius.xyz;
            radius = clustered_lights.data[object_index].range;
        }
        case CLUSTERABLE_OBJECT_TYPE_REFLECTION_PROBE: {
            position = light_probes.reflection_probes[object_index].world_position;
            radius = light_probes.reflection_probes[object_index].bounding_sphere_radius;
        }
        case CLUSTERABLE_OBJECT_TYPE_IRRADIANCE_VOLUME: {
            position = light_probes.irradiance_volumes[object_index].world_position;
            radius = light_probes.irradiance_volumes[object_index].bounding_sphere_radius;
        }
        case CLUSTERABLE_OBJECT_TYPE_DECAL: {
            position = clustered_decals.decals[object_index].world_position;
            radius = clustered_decals.decals[object_index].bounding_sphere_radius;
        }
        default: {}
    }
    return vec4(position, radius);
}
