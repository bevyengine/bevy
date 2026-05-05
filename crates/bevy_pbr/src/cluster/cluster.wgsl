#define_import_path bevy_pbr::cluster
#import bevy_pbr::clustered_forward::view_z_to_z_slice

// Valid values for the `object_type` field.
const CLUSTERABLE_OBJECT_TYPE_POINT_LIGHT: u32 = 0u;
const CLUSTERABLE_OBJECT_TYPE_SPOT_LIGHT: u32 = 1u;
const CLUSTERABLE_OBJECT_TYPE_REFLECTION_PROBE: u32 = 2u;
const CLUSTERABLE_OBJECT_TYPE_IRRADIANCE_VOLUME: u32 = 3u;
const CLUSTERABLE_OBJECT_TYPE_DECAL: u32 = 4u;

const NDC_MIN: vec2<f32> = vec2<f32>(-1.0);
const NDC_MAX: vec2<f32> = vec2<f32>(1.0);

// Metadata stored on GPU that's global to all clusters for a view.
//
// See the comments in `bevy_pbr/src/cluster/gpu.rs` for information on the
// fields.
struct ClusterMetadata {
    indirect_draw_params: ClusterRasterIndirectDrawParams,

    clustered_light_count: u32,
    reflection_probe_count: u32,
    irradiance_volume_count: u32,
    decal_count: u32,

    z_slice_list_capacity: u32,
    index_list_size: u32,

    farthest_z: atomic<u32>,
};

// Indirect draw parameters in the format required by the WebGPU specification.
struct ClusterRasterIndirectDrawParams {
    index_count: u32,
    instance_count: atomic<u32>,
    first_index: u32,
    base_vertex: u32,
    first_instance: u32,
}

// The GPU representation of a single Z-slice of a clusterable object.
//
// See the comments in `bevy_pbr/src/cluster/gpu.rs` for information on the
// fields.
struct ClusterableObjectZSlice {
    object_index: u32,
    object_type: u32,
    z_slice: u32,
};

// An axis-aligned bounding box.
struct Aabb {
    // The minimum extents of the box.
    min: vec3<f32>,
    // The maximum extents of the box.
    max: vec3<f32>,
};

// An axis-aligned bounding box using unsigned integer coordinates.
//
// This is used for cluster bounds.
struct AabbU {
    // The minimum extents of the box.
    min: vec3<u32>,
    // The maximum extents of the box, plus one.
    //
    // We add 1 here so that 0-size AABBs can be expressed.
    max: vec3<u32>,
}

// Returns the AABB of an object suitable for conversion into an AABB of
// clusters.
//
// See `bevy_light::cluster::assign::cluster_space_clusterable_object_aabb`.
fn cluster_space_object_aabb(
    position: vec3<f32>,
    radius: f32,
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    view_from_world_scale: vec3<f32>
) -> Aabb {
    let position_view = (view_from_world * vec4(position, 1.0)).xyz;
    let half_extents = radius * abs(view_from_world_scale);

    var view_min = position_view - half_extents;
    var view_max = position_view + half_extents;

    // Constrain view z to be negative - i.e. in front of the camera
    // When view z is >= 0.0 and we're using a perspective projection, bad
    // things happen.  At view z == 0.0, ndc x,y are mathematically undefined.
    // At view z > 0.0, i.e. behind the camera, the perspective projection flips
    // the directions of the axes. This breaks assumptions about use of min/max
    // operations as something that was to the left in view space is now
    // returning a coordinate that for view z in front of the camera would be on
    // the right, but at view z behind the camera is on the left. So, we just
    // constrain view z to be < 0.0 and necessarily in front of the camera.
    view_min.z = min(view_min.z, -0.00001);
    view_max.z = min(view_max.z, -0.00001);

    // Is there a cheaper way to do this? The problem is that because of
    // perspective the point at max z but min xy may be less xy in screenspace,
    // and similar. As such, projecting the min and max xy at both the closer
    // and further z and taking the min and max of those projected points
    // addresses this.
    let view_xymin_near = view_min;
    let view_xymin_far = vec3(view_min.xy, view_max.z);
    let view_xymax_near = vec3(view_max.xy, view_min.z);
    let view_xymax_far = view_max;

    let clip_xymin_near = clip_from_view * vec4(view_xymin_near, 1.0);
    let clip_xymin_far = clip_from_view * vec4(view_xymin_far, 1.0);
    let clip_xymax_near = clip_from_view * vec4(view_xymax_near, 1.0);
    let clip_xymax_far = clip_from_view * vec4(view_xymax_far, 1.0);

    let ndc_xymin_near = clip_xymin_near.xyz / clip_xymin_near.w;
    let ndc_xymin_far = clip_xymin_far.xyz / clip_xymin_far.w;
    let ndc_xymax_near = clip_xymax_near.xyz / clip_xymax_near.w;
    let ndc_xymax_far = clip_xymax_far.xyz / clip_xymax_far.w;

    var ndc_min = min(min(ndc_xymin_near, ndc_xymin_far), min(ndc_xymax_near, ndc_xymax_far));
    var ndc_max = max(max(ndc_xymin_near, ndc_xymin_far), max(ndc_xymax_near, ndc_xymax_far));

    // clamp to ndc coords without depth
    ndc_min = vec3(clamp(ndc_min.xy, NDC_MIN, NDC_MAX), ndc_min.z);
    ndc_max = vec3(clamp(ndc_max.xy, NDC_MIN, NDC_MAX), ndc_max.z);

    // pack unadjusted z depth into the vecs
    return Aabb(vec3(ndc_min.xy, view_min.z), vec3(ndc_max.xy, view_max.z));
}

// Computes the scale of the camera from the view matrix.
fn compute_view_from_world_scale(world_from_view: mat4x4<f32>) -> vec3<f32> {
    let world_from_view_3x3 = mat3x3<f32>(
        world_from_view[0].xyz,
        world_from_view[1].xyz,
        world_from_view[2].xyz
    );
    let det = determinant(world_from_view_3x3);
    let scale = vec3<f32>(
        length(world_from_view_3x3[0]) * sign(det),
        length(world_from_view_3x3[1]),
        length(world_from_view_3x3[2])
    );
    return vec3<f32>(1.0) / scale;
}

// Returns the cluster coordinates corresponding to a position in normalized
// device coordinates.
// See `bevy_light::cluster::assign::ndc_position_to_cluster`.
fn ndc_position_to_cluster(
    cluster_dimensions: vec3<u32>,
    cluster_factors: vec2<f32>,
    is_orthographic: bool,
    ndc_p: vec3<f32>,
    view_z: f32
) -> vec3<u32> {
    let frag_coord = clamp(ndc_p.xy * vec2(0.5, -0.5) + vec2(0.5), vec2(0.0), vec2(1.0));
    let xy = vec2<u32>(floor(frag_coord * vec2<f32>(cluster_dimensions.xy)));
    let z_slice = view_z_to_z_slice(cluster_factors, cluster_dimensions.z, view_z, is_orthographic);
    return clamp(vec3<u32>(xy, z_slice), vec3(0u), cluster_dimensions - vec3(1u));
}

// Returns the AABB encompassing all clusters that intersect a sphere.
fn calculate_sphere_cluster_bounds(
    position: vec3<f32>,
    radius: f32,
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    view_from_world_scale: vec3<f32>,
    cluster_dimensions: vec3<u32>,
    cluster_factors: vec2<f32>,
    is_orthographic: bool,
) -> AabbU {
    let aabb_ndc = cluster_space_object_aabb(
        position,
        radius,
        view_from_world,
        clip_from_view,
        view_from_world_scale
    );

    let temp_min_cluster = ndc_position_to_cluster(
        cluster_dimensions,
        cluster_factors,
        is_orthographic,
        aabb_ndc.min,
        aabb_ndc.min.z
    );
    let temp_max_cluster = ndc_position_to_cluster(
        cluster_dimensions,
        cluster_factors,
        is_orthographic,
        aabb_ndc.max,
        aabb_ndc.max.z
    );

    let min_cluster = min(temp_min_cluster, temp_max_cluster);
    let max_cluster = max(temp_min_cluster, temp_max_cluster);

    return AabbU(min_cluster, max_cluster);
}
