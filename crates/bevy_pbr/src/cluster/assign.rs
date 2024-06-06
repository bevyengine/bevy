//! Assigning objects to clusters.

use bevy_ecs::{
    entity::Entity,
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_math::{Mat4, UVec3, Vec2, Vec3, Vec3A, Vec3Swizzles as _, Vec4, Vec4Swizzles as _};
use bevy_render::{
    camera::Camera,
    primitives::{Aabb, Frustum, HalfSpace, Sphere},
    render_resource::BufferBindingType,
    renderer::RenderDevice,
    view::{RenderLayers, ViewVisibility},
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::tracing::warn;

use crate::{
    ClusterConfig, ClusterFarZMode, Clusters, GlobalVisibleClusterableObjects, PointLight,
    SpotLight, ViewClusterBindings, VisibleClusterableObjects,
    CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT, MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS,
};

const NDC_MIN: Vec2 = Vec2::NEG_ONE;
const NDC_MAX: Vec2 = Vec2::ONE;

const VEC2_HALF: Vec2 = Vec2::splat(0.5);
const VEC2_HALF_NEGATIVE_Y: Vec2 = Vec2::new(0.5, -0.5);

#[derive(Clone)]
// data required for assigning objects to clusters
pub(crate) struct ClusterableObjectAssignmentData {
    entity: Entity,
    transform: GlobalTransform,
    range: f32,
    shadows_enabled: bool,
    spot_light_angle: Option<f32>,
    render_layers: RenderLayers,
}

impl ClusterableObjectAssignmentData {
    pub fn sphere(&self) -> Sphere {
        Sphere {
            center: self.transform.translation_vec3a(),
            radius: self.range,
        }
    }
}

// NOTE: Run this before update_point_light_frusta!
#[allow(clippy::too_many_arguments)]
pub(crate) fn assign_objects_to_clusters(
    mut commands: Commands,
    mut global_clusterable_objects: ResMut<GlobalVisibleClusterableObjects>,
    mut views: Query<(
        Entity,
        &GlobalTransform,
        &Camera,
        &Frustum,
        &ClusterConfig,
        &mut Clusters,
        Option<&RenderLayers>,
        Option<&mut VisibleClusterableObjects>,
    )>,
    point_lights_query: Query<(
        Entity,
        &GlobalTransform,
        &PointLight,
        Option<&RenderLayers>,
        &ViewVisibility,
    )>,
    spot_lights_query: Query<(
        Entity,
        &GlobalTransform,
        &SpotLight,
        Option<&RenderLayers>,
        &ViewVisibility,
    )>,
    mut clusterable_objects: Local<Vec<ClusterableObjectAssignmentData>>,
    mut cluster_aabb_spheres: Local<Vec<Option<Sphere>>>,
    mut max_clusterable_objects_warning_emitted: Local<bool>,
    render_device: Option<Res<RenderDevice>>,
) {
    let Some(render_device) = render_device else {
        return;
    };

    global_clusterable_objects.entities.clear();
    clusterable_objects.clear();
    // collect just the relevant query data into a persisted vec to avoid reallocating each frame
    clusterable_objects.extend(
        point_lights_query
            .iter()
            .filter(|(.., visibility)| visibility.get())
            .map(
                |(entity, transform, point_light, maybe_layers, _visibility)| {
                    ClusterableObjectAssignmentData {
                        entity,
                        transform: GlobalTransform::from_translation(transform.translation()),
                        shadows_enabled: point_light.shadows_enabled,
                        range: point_light.range,
                        spot_light_angle: None,
                        render_layers: maybe_layers.unwrap_or_default().clone(),
                    }
                },
            ),
    );
    clusterable_objects.extend(
        spot_lights_query
            .iter()
            .filter(|(.., visibility)| visibility.get())
            .map(
                |(entity, transform, spot_light, maybe_layers, _visibility)| {
                    ClusterableObjectAssignmentData {
                        entity,
                        transform: *transform,
                        shadows_enabled: spot_light.shadows_enabled,
                        range: spot_light.range,
                        spot_light_angle: Some(spot_light.outer_angle),
                        render_layers: maybe_layers.unwrap_or_default().clone(),
                    }
                },
            ),
    );

    let clustered_forward_buffer_binding_type =
        render_device.get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);
    let supports_storage_buffers = matches!(
        clustered_forward_buffer_binding_type,
        BufferBindingType::Storage { .. }
    );
    if clusterable_objects.len() > MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS
        && !supports_storage_buffers
    {
        clusterable_objects.sort_by(|clusterable_object_1, clusterable_object_2| {
            crate::clusterable_object_order(
                (
                    &clusterable_object_1.entity,
                    &clusterable_object_1.shadows_enabled,
                    &clusterable_object_1.spot_light_angle.is_some(),
                ),
                (
                    &clusterable_object_2.entity,
                    &clusterable_object_2.shadows_enabled,
                    &clusterable_object_2.spot_light_angle.is_some(),
                ),
            )
        });

        // check each clusterable object against each view's frustum, keep only
        // those that affect at least one of our views
        let frusta: Vec<_> = views
            .iter()
            .map(|(_, _, _, frustum, _, _, _, _)| *frustum)
            .collect();
        let mut clusterable_objects_in_view_count = 0;
        clusterable_objects.retain(|clusterable_object| {
            // take one extra clusterable object to check if we should emit the warning
            if clusterable_objects_in_view_count == MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS + 1 {
                false
            } else {
                let clusterable_object_sphere = clusterable_object.sphere();
                let clusterable_object_in_view = frusta
                    .iter()
                    .any(|frustum| frustum.intersects_sphere(&clusterable_object_sphere, true));

                if clusterable_object_in_view {
                    clusterable_objects_in_view_count += 1;
                }

                clusterable_object_in_view
            }
        });

        if clusterable_objects.len() > MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS
            && !*max_clusterable_objects_warning_emitted
        {
            warn!(
                "MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS ({}) exceeded",
                MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS
            );
            *max_clusterable_objects_warning_emitted = true;
        }

        clusterable_objects.truncate(MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS);
    }

    for (
        view_entity,
        camera_transform,
        camera,
        frustum,
        config,
        clusters,
        maybe_layers,
        mut visible_clusterable_objects,
    ) in &mut views
    {
        let view_layers = maybe_layers.unwrap_or_default();
        let clusters = clusters.into_inner();

        if matches!(config, ClusterConfig::None) {
            if visible_clusterable_objects.is_some() {
                commands
                    .entity(view_entity)
                    .remove::<VisibleClusterableObjects>();
            }
            clusters.clear();
            continue;
        }

        let Some(screen_size) = camera.physical_viewport_size() else {
            clusters.clear();
            continue;
        };

        let mut requested_cluster_dimensions = config.dimensions_for_screen_size(screen_size);

        let world_from_view = camera_transform.compute_matrix();
        let view_from_world_scale = camera_transform.compute_transform().scale.recip();
        let view_from_world_scale_max = view_from_world_scale.abs().max_element();
        let view_from_world = world_from_view.inverse();
        let is_orthographic = camera.clip_from_view().w_axis.w == 1.0;

        let far_z = match config.far_z_mode() {
            ClusterFarZMode::MaxClusterableObjectRange => {
                let view_from_world_row_2 = view_from_world.row(2);
                clusterable_objects
                    .iter()
                    .map(|object| {
                        -view_from_world_row_2.dot(object.transform.translation().extend(1.0))
                            + object.range * view_from_world_scale.z
                    })
                    .reduce(f32::max)
                    .unwrap_or(0.0)
            }
            ClusterFarZMode::Constant(far) => far,
        };
        let first_slice_depth = match (is_orthographic, requested_cluster_dimensions.z) {
            (true, _) => {
                // NOTE: Based on glam's Mat4::orthographic_rh(), as used to calculate the orthographic projection
                // matrix, we can calculate the projection's view-space near plane as follows:
                // component 3,2 = r * near and 2,2 = r where r = 1.0 / (near - far)
                // There is a caveat here that when calculating the projection matrix, near and far were swapped to give
                // reversed z, consistent with the perspective projection. So,
                // 3,2 = r * far and 2,2 = r where r = 1.0 / (far - near)
                // rearranging r = 1.0 / (far - near), r * (far - near) = 1.0, r * far - 1.0 = r * near, near = (r * far - 1.0) / r
                // = (3,2 - 1.0) / 2,2
                (camera.clip_from_view().w_axis.z - 1.0) / camera.clip_from_view().z_axis.z
            }
            (false, 1) => config.first_slice_depth().max(far_z),
            _ => config.first_slice_depth(),
        };
        let first_slice_depth = first_slice_depth * view_from_world_scale.z;

        // NOTE: Ensure the far_z is at least as far as the first_depth_slice to avoid clustering problems.
        let far_z = far_z.max(first_slice_depth);
        let cluster_factors = crate::calculate_cluster_factors(
            first_slice_depth,
            far_z,
            requested_cluster_dimensions.z as f32,
            is_orthographic,
        );

        if config.dynamic_resizing() {
            let mut cluster_index_estimate = 0.0;
            for clusterable_object in &clusterable_objects {
                let clusterable_object_sphere = clusterable_object.sphere();

                // Check if the clusterable object is within the view frustum
                if !frustum.intersects_sphere(&clusterable_object_sphere, true) {
                    continue;
                }

                // calculate a conservative aabb estimate of number of clusters affected by this light
                // this overestimates index counts by at most 50% (and typically much less) when the whole light range is in view
                // it can overestimate more significantly when light ranges are only partially in view
                let (clusterable_object_aabb_min, clusterable_object_aabb_max) =
                    cluster_space_clusterable_object_aabb(
                        view_from_world,
                        view_from_world_scale,
                        camera.clip_from_view(),
                        &clusterable_object_sphere,
                    );

                // since we won't adjust z slices we can calculate exact number of slices required in z dimension
                let z_cluster_min = view_z_to_z_slice(
                    cluster_factors,
                    requested_cluster_dimensions.z,
                    clusterable_object_aabb_min.z,
                    is_orthographic,
                );
                let z_cluster_max = view_z_to_z_slice(
                    cluster_factors,
                    requested_cluster_dimensions.z,
                    clusterable_object_aabb_max.z,
                    is_orthographic,
                );
                let z_count =
                    z_cluster_min.max(z_cluster_max) - z_cluster_min.min(z_cluster_max) + 1;

                // calculate x/y count using floats to avoid overestimating counts due to large initial tile sizes
                let xy_min = clusterable_object_aabb_min.xy();
                let xy_max = clusterable_object_aabb_max.xy();
                // multiply by 0.5 to move from [-1,1] to [-0.5, 0.5], max extent of 1 in each dimension
                let xy_count = (xy_max - xy_min)
                    * 0.5
                    * Vec2::new(
                        requested_cluster_dimensions.x as f32,
                        requested_cluster_dimensions.y as f32,
                    );

                // add up to 2 to each axis to account for overlap
                let x_overlap = if xy_min.x <= -1.0 { 0.0 } else { 1.0 }
                    + if xy_max.x >= 1.0 { 0.0 } else { 1.0 };
                let y_overlap = if xy_min.y <= -1.0 { 0.0 } else { 1.0 }
                    + if xy_max.y >= 1.0 { 0.0 } else { 1.0 };
                cluster_index_estimate +=
                    (xy_count.x + x_overlap) * (xy_count.y + y_overlap) * z_count as f32;
            }

            if cluster_index_estimate > ViewClusterBindings::MAX_INDICES as f32 {
                // scale x and y cluster count to be able to fit all our indices

                // we take the ratio of the actual indices over the index estimate.
                // this is not guaranteed to be small enough due to overlapped tiles, but
                // the conservative estimate is more than sufficient to cover the
                // difference
                let index_ratio = ViewClusterBindings::MAX_INDICES as f32 / cluster_index_estimate;
                let xy_ratio = index_ratio.sqrt();

                requested_cluster_dimensions.x =
                    ((requested_cluster_dimensions.x as f32 * xy_ratio).floor() as u32).max(1);
                requested_cluster_dimensions.y =
                    ((requested_cluster_dimensions.y as f32 * xy_ratio).floor() as u32).max(1);
            }
        }

        clusters.update(screen_size, requested_cluster_dimensions);
        clusters.near = first_slice_depth;
        clusters.far = far_z;

        // NOTE: Maximum 4096 clusters due to uniform buffer size constraints
        debug_assert!(
            clusters.dimensions.x * clusters.dimensions.y * clusters.dimensions.z <= 4096
        );

        let view_from_clip = camera.clip_from_view().inverse();

        for clusterable_objects in &mut clusters.clusterable_objects {
            clusterable_objects.entities.clear();
            clusterable_objects.point_light_count = 0;
            clusterable_objects.spot_light_count = 0;
        }
        let cluster_count =
            (clusters.dimensions.x * clusters.dimensions.y * clusters.dimensions.z) as usize;
        clusters
            .clusterable_objects
            .resize_with(cluster_count, VisibleClusterableObjects::default);

        // initialize empty cluster bounding spheres
        cluster_aabb_spheres.clear();
        cluster_aabb_spheres.extend(std::iter::repeat(None).take(cluster_count));

        // Calculate the x/y/z cluster frustum planes in view space
        let mut x_planes = Vec::with_capacity(clusters.dimensions.x as usize + 1);
        let mut y_planes = Vec::with_capacity(clusters.dimensions.y as usize + 1);
        let mut z_planes = Vec::with_capacity(clusters.dimensions.z as usize + 1);

        if is_orthographic {
            let x_slices = clusters.dimensions.x as f32;
            for x in 0..=clusters.dimensions.x {
                let x_proportion = x as f32 / x_slices;
                let x_pos = x_proportion * 2.0 - 1.0;
                let view_x = clip_to_view(view_from_clip, Vec4::new(x_pos, 0.0, 1.0, 1.0)).x;
                let normal = Vec3::X;
                let d = view_x * normal.x;
                x_planes.push(HalfSpace::new(normal.extend(d)));
            }

            let y_slices = clusters.dimensions.y as f32;
            for y in 0..=clusters.dimensions.y {
                let y_proportion = 1.0 - y as f32 / y_slices;
                let y_pos = y_proportion * 2.0 - 1.0;
                let view_y = clip_to_view(view_from_clip, Vec4::new(0.0, y_pos, 1.0, 1.0)).y;
                let normal = Vec3::Y;
                let d = view_y * normal.y;
                y_planes.push(HalfSpace::new(normal.extend(d)));
            }
        } else {
            let x_slices = clusters.dimensions.x as f32;
            for x in 0..=clusters.dimensions.x {
                let x_proportion = x as f32 / x_slices;
                let x_pos = x_proportion * 2.0 - 1.0;
                let nb = clip_to_view(view_from_clip, Vec4::new(x_pos, -1.0, 1.0, 1.0)).xyz();
                let nt = clip_to_view(view_from_clip, Vec4::new(x_pos, 1.0, 1.0, 1.0)).xyz();
                let normal = nb.cross(nt);
                let d = nb.dot(normal);
                x_planes.push(HalfSpace::new(normal.extend(d)));
            }

            let y_slices = clusters.dimensions.y as f32;
            for y in 0..=clusters.dimensions.y {
                let y_proportion = 1.0 - y as f32 / y_slices;
                let y_pos = y_proportion * 2.0 - 1.0;
                let nl = clip_to_view(view_from_clip, Vec4::new(-1.0, y_pos, 1.0, 1.0)).xyz();
                let nr = clip_to_view(view_from_clip, Vec4::new(1.0, y_pos, 1.0, 1.0)).xyz();
                let normal = nr.cross(nl);
                let d = nr.dot(normal);
                y_planes.push(HalfSpace::new(normal.extend(d)));
            }
        }

        let z_slices = clusters.dimensions.z;
        for z in 0..=z_slices {
            let view_z = z_slice_to_view_z(first_slice_depth, far_z, z_slices, z, is_orthographic);
            let normal = -Vec3::Z;
            let d = view_z * normal.z;
            z_planes.push(HalfSpace::new(normal.extend(d)));
        }

        let mut update_from_object_intersections = |visible_clusterable_objects: &mut Vec<
            Entity,
        >| {
            for clusterable_object in &clusterable_objects {
                // check if the clusterable light layers overlap the view layers
                if !view_layers.intersects(&clusterable_object.render_layers) {
                    continue;
                }

                let clusterable_object_sphere = clusterable_object.sphere();

                // Check if the clusterable object is within the view frustum
                if !frustum.intersects_sphere(&clusterable_object_sphere, true) {
                    continue;
                }

                // NOTE: The clusterable object intersects the frustum so it
                // must be visible and part of the global set
                global_clusterable_objects
                    .entities
                    .insert(clusterable_object.entity);
                visible_clusterable_objects.push(clusterable_object.entity);

                // note: caching seems to be slower than calling twice for this aabb calculation
                let (
                    clusterable_object_aabb_xy_ndc_z_view_min,
                    clusterable_object_aabb_xy_ndc_z_view_max,
                ) = cluster_space_clusterable_object_aabb(
                    view_from_world,
                    view_from_world_scale,
                    camera.clip_from_view(),
                    &clusterable_object_sphere,
                );

                let min_cluster = ndc_position_to_cluster(
                    clusters.dimensions,
                    cluster_factors,
                    is_orthographic,
                    clusterable_object_aabb_xy_ndc_z_view_min,
                    clusterable_object_aabb_xy_ndc_z_view_min.z,
                );
                let max_cluster = ndc_position_to_cluster(
                    clusters.dimensions,
                    cluster_factors,
                    is_orthographic,
                    clusterable_object_aabb_xy_ndc_z_view_max,
                    clusterable_object_aabb_xy_ndc_z_view_max.z,
                );
                let (min_cluster, max_cluster) =
                    (min_cluster.min(max_cluster), min_cluster.max(max_cluster));

                // What follows is the Iterative Sphere Refinement algorithm from Just Cause 3
                // Persson et al, Practical Clustered Shading
                // http://newq.net/dl/pub/s2015_practical.pdf
                // NOTE: A sphere under perspective projection is no longer a sphere. It gets
                // stretched and warped, which prevents simpler algorithms from being correct
                // as they often assume that the widest part of the sphere under projection is the
                // center point on the axis of interest plus the radius, and that is not true!
                let view_clusterable_object_sphere = Sphere {
                    center: Vec3A::from(
                        view_from_world * clusterable_object_sphere.center.extend(1.0),
                    ),
                    radius: clusterable_object_sphere.radius * view_from_world_scale_max,
                };
                let spot_light_dir_sin_cos = clusterable_object.spot_light_angle.map(|angle| {
                    let (angle_sin, angle_cos) = angle.sin_cos();
                    (
                        (view_from_world * clusterable_object.transform.back().extend(0.0))
                            .truncate()
                            .normalize(),
                        angle_sin,
                        angle_cos,
                    )
                });
                let clusterable_object_center_clip =
                    camera.clip_from_view() * view_clusterable_object_sphere.center.extend(1.0);
                let object_center_ndc =
                    clusterable_object_center_clip.xyz() / clusterable_object_center_clip.w;
                let cluster_coordinates = ndc_position_to_cluster(
                    clusters.dimensions,
                    cluster_factors,
                    is_orthographic,
                    object_center_ndc,
                    view_clusterable_object_sphere.center.z,
                );
                let z_center = if object_center_ndc.z <= 1.0 {
                    Some(cluster_coordinates.z)
                } else {
                    None
                };
                let y_center = if object_center_ndc.y > 1.0 {
                    None
                } else if object_center_ndc.y < -1.0 {
                    Some(clusters.dimensions.y + 1)
                } else {
                    Some(cluster_coordinates.y)
                };
                for z in min_cluster.z..=max_cluster.z {
                    let mut z_object = view_clusterable_object_sphere.clone();
                    if z_center.is_none() || z != z_center.unwrap() {
                        // The z plane closer to the clusterable object has the
                        // larger radius circle where the light sphere
                        // intersects the z plane.
                        let z_plane = if z_center.is_some() && z < z_center.unwrap() {
                            z_planes[(z + 1) as usize]
                        } else {
                            z_planes[z as usize]
                        };
                        // Project the sphere to this z plane and use its radius as the radius of a
                        // new, refined sphere.
                        if let Some(projected) = project_to_plane_z(z_object, z_plane) {
                            z_object = projected;
                        } else {
                            continue;
                        }
                    }
                    for y in min_cluster.y..=max_cluster.y {
                        let mut y_object = z_object.clone();
                        if y_center.is_none() || y != y_center.unwrap() {
                            // The y plane closer to the clusterable object has
                            // the larger radius circle where the light sphere
                            // intersects the y plane.
                            let y_plane = if y_center.is_some() && y < y_center.unwrap() {
                                y_planes[(y + 1) as usize]
                            } else {
                                y_planes[y as usize]
                            };
                            // Project the refined sphere to this y plane and use its radius as the
                            // radius of a new, even more refined sphere.
                            if let Some(projected) =
                                project_to_plane_y(y_object, y_plane, is_orthographic)
                            {
                                y_object = projected;
                            } else {
                                continue;
                            }
                        }
                        // Loop from the left to find the first affected cluster
                        let mut min_x = min_cluster.x;
                        loop {
                            if min_x >= max_cluster.x
                                || -get_distance_x(
                                    x_planes[(min_x + 1) as usize],
                                    y_object.center,
                                    is_orthographic,
                                ) + y_object.radius
                                    > 0.0
                            {
                                break;
                            }
                            min_x += 1;
                        }
                        // Loop from the right to find the last affected cluster
                        let mut max_x = max_cluster.x;
                        loop {
                            if max_x <= min_x
                                || get_distance_x(
                                    x_planes[max_x as usize],
                                    y_object.center,
                                    is_orthographic,
                                ) + y_object.radius
                                    > 0.0
                            {
                                break;
                            }
                            max_x -= 1;
                        }
                        let mut cluster_index = ((y * clusters.dimensions.x + min_x)
                            * clusters.dimensions.z
                            + z) as usize;

                        if let Some((view_light_direction, angle_sin, angle_cos)) =
                            spot_light_dir_sin_cos
                        {
                            for x in min_x..=max_x {
                                // further culling for spot lights
                                // get or initialize cluster bounding sphere
                                let cluster_aabb_sphere = &mut cluster_aabb_spheres[cluster_index];
                                let cluster_aabb_sphere = if let Some(sphere) = cluster_aabb_sphere
                                {
                                    &*sphere
                                } else {
                                    let aabb = compute_aabb_for_cluster(
                                        first_slice_depth,
                                        far_z,
                                        clusters.tile_size.as_vec2(),
                                        screen_size.as_vec2(),
                                        view_from_clip,
                                        is_orthographic,
                                        clusters.dimensions,
                                        UVec3::new(x, y, z),
                                    );
                                    let sphere = Sphere {
                                        center: aabb.center,
                                        radius: aabb.half_extents.length(),
                                    };
                                    *cluster_aabb_sphere = Some(sphere);
                                    cluster_aabb_sphere.as_ref().unwrap()
                                };

                                // test -- based on https://bartwronski.com/2017/04/13/cull-that-cone/
                                let spot_light_offset = Vec3::from(
                                    view_clusterable_object_sphere.center
                                        - cluster_aabb_sphere.center,
                                );
                                let spot_light_dist_sq = spot_light_offset.length_squared();
                                let v1_len = spot_light_offset.dot(view_light_direction);

                                let distance_closest_point = (angle_cos
                                    * (spot_light_dist_sq - v1_len * v1_len).sqrt())
                                    - v1_len * angle_sin;
                                let angle_cull =
                                    distance_closest_point > cluster_aabb_sphere.radius;

                                let front_cull = v1_len
                                    > cluster_aabb_sphere.radius
                                        + clusterable_object.range * view_from_world_scale_max;
                                let back_cull = v1_len < -cluster_aabb_sphere.radius;

                                if !angle_cull && !front_cull && !back_cull {
                                    // this cluster is affected by the spot light
                                    clusters.clusterable_objects[cluster_index]
                                        .entities
                                        .push(clusterable_object.entity);
                                    clusters.clusterable_objects[cluster_index].spot_light_count +=
                                        1;
                                }
                                cluster_index += clusters.dimensions.z as usize;
                            }
                        } else {
                            for _ in min_x..=max_x {
                                // all clusters within range are affected by point lights
                                clusters.clusterable_objects[cluster_index]
                                    .entities
                                    .push(clusterable_object.entity);
                                clusters.clusterable_objects[cluster_index].point_light_count += 1;
                                cluster_index += clusters.dimensions.z as usize;
                            }
                        }
                    }
                }
            }
        };

        // reuse existing visible clusterable objects Vec, if it exists
        if let Some(visible_clusterable_objects) = visible_clusterable_objects.as_mut() {
            visible_clusterable_objects.entities.clear();
            update_from_object_intersections(&mut visible_clusterable_objects.entities);
        } else {
            let mut entities = Vec::new();
            update_from_object_intersections(&mut entities);
            commands
                .entity(view_entity)
                .insert(VisibleClusterableObjects {
                    entities,
                    ..Default::default()
                });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn compute_aabb_for_cluster(
    z_near: f32,
    z_far: f32,
    tile_size: Vec2,
    screen_size: Vec2,
    view_from_clip: Mat4,
    is_orthographic: bool,
    cluster_dimensions: UVec3,
    ijk: UVec3,
) -> Aabb {
    let ijk = ijk.as_vec3();

    // Calculate the minimum and maximum points in screen space
    let p_min = ijk.xy() * tile_size;
    let p_max = p_min + tile_size;

    let cluster_min;
    let cluster_max;
    if is_orthographic {
        // Use linear depth slicing for orthographic

        // Convert to view space at the cluster near and far planes
        // NOTE: 1.0 is the near plane due to using reverse z projections
        let mut p_min = screen_to_view(screen_size, view_from_clip, p_min, 0.0).xyz();
        let mut p_max = screen_to_view(screen_size, view_from_clip, p_max, 0.0).xyz();

        // calculate cluster depth using z_near and z_far
        p_min.z = -z_near + (z_near - z_far) * ijk.z / cluster_dimensions.z as f32;
        p_max.z = -z_near + (z_near - z_far) * (ijk.z + 1.0) / cluster_dimensions.z as f32;

        cluster_min = p_min.min(p_max);
        cluster_max = p_min.max(p_max);
    } else {
        // Convert to view space at the near plane
        // NOTE: 1.0 is the near plane due to using reverse z projections
        let p_min = screen_to_view(screen_size, view_from_clip, p_min, 1.0);
        let p_max = screen_to_view(screen_size, view_from_clip, p_max, 1.0);

        let z_far_over_z_near = -z_far / -z_near;
        let cluster_near = if ijk.z == 0.0 {
            0.0
        } else {
            -z_near * z_far_over_z_near.powf((ijk.z - 1.0) / (cluster_dimensions.z - 1) as f32)
        };
        // NOTE: This could be simplified to:
        // cluster_far = cluster_near * z_far_over_z_near;
        let cluster_far = if cluster_dimensions.z == 1 {
            -z_far
        } else {
            -z_near * z_far_over_z_near.powf(ijk.z / (cluster_dimensions.z - 1) as f32)
        };

        // Calculate the four intersection points of the min and max points with the cluster near and far planes
        let p_min_near = line_intersection_to_z_plane(Vec3::ZERO, p_min.xyz(), cluster_near);
        let p_min_far = line_intersection_to_z_plane(Vec3::ZERO, p_min.xyz(), cluster_far);
        let p_max_near = line_intersection_to_z_plane(Vec3::ZERO, p_max.xyz(), cluster_near);
        let p_max_far = line_intersection_to_z_plane(Vec3::ZERO, p_max.xyz(), cluster_far);

        cluster_min = p_min_near.min(p_min_far).min(p_max_near.min(p_max_far));
        cluster_max = p_min_near.max(p_min_far).max(p_max_near.max(p_max_far));
    }

    Aabb::from_min_max(cluster_min, cluster_max)
}

// NOTE: Keep in sync as the inverse of view_z_to_z_slice above
fn z_slice_to_view_z(
    near: f32,
    far: f32,
    z_slices: u32,
    z_slice: u32,
    is_orthographic: bool,
) -> f32 {
    if is_orthographic {
        return -near - (far - near) * z_slice as f32 / z_slices as f32;
    }

    // Perspective
    if z_slice == 0 {
        0.0
    } else {
        -near * (far / near).powf((z_slice - 1) as f32 / (z_slices - 1) as f32)
    }
}

fn ndc_position_to_cluster(
    cluster_dimensions: UVec3,
    cluster_factors: Vec2,
    is_orthographic: bool,
    ndc_p: Vec3,
    view_z: f32,
) -> UVec3 {
    let cluster_dimensions_f32 = cluster_dimensions.as_vec3();
    let frag_coord = (ndc_p.xy() * VEC2_HALF_NEGATIVE_Y + VEC2_HALF).clamp(Vec2::ZERO, Vec2::ONE);
    let xy = (frag_coord * cluster_dimensions_f32.xy()).floor();
    let z_slice = view_z_to_z_slice(
        cluster_factors,
        cluster_dimensions.z,
        view_z,
        is_orthographic,
    );
    xy.as_uvec2()
        .extend(z_slice)
        .clamp(UVec3::ZERO, cluster_dimensions - UVec3::ONE)
}

/// Calculate bounds for the clusterable object using a view space aabb.
/// Returns a `(Vec3, Vec3)` containing minimum and maximum with
///     `X` and `Y` in normalized device coordinates with range `[-1, 1]`
///     `Z` in view space, with range `[-inf, -f32::MIN_POSITIVE]`
fn cluster_space_clusterable_object_aabb(
    view_from_world: Mat4,
    view_from_world_scale: Vec3,
    clip_from_view: Mat4,
    clusterable_object_sphere: &Sphere,
) -> (Vec3, Vec3) {
    let clusterable_object_aabb_view = Aabb {
        center: Vec3A::from(view_from_world * clusterable_object_sphere.center.extend(1.0)),
        half_extents: Vec3A::from(clusterable_object_sphere.radius * view_from_world_scale.abs()),
    };
    let (mut clusterable_object_aabb_view_min, mut clusterable_object_aabb_view_max) = (
        clusterable_object_aabb_view.min(),
        clusterable_object_aabb_view.max(),
    );

    // Constrain view z to be negative - i.e. in front of the camera
    // When view z is >= 0.0 and we're using a perspective projection, bad things happen.
    // At view z == 0.0, ndc x,y are mathematically undefined. At view z > 0.0, i.e. behind the camera,
    // the perspective projection flips the directions of the axes. This breaks assumptions about
    // use of min/max operations as something that was to the left in view space is now returning a
    // coordinate that for view z in front of the camera would be on the right, but at view z behind the
    // camera is on the left. So, we just constrain view z to be < 0.0 and necessarily in front of the camera.
    clusterable_object_aabb_view_min.z = clusterable_object_aabb_view_min.z.min(-f32::MIN_POSITIVE);
    clusterable_object_aabb_view_max.z = clusterable_object_aabb_view_max.z.min(-f32::MIN_POSITIVE);

    // Is there a cheaper way to do this? The problem is that because of perspective
    // the point at max z but min xy may be less xy in screenspace, and similar. As
    // such, projecting the min and max xy at both the closer and further z and taking
    // the min and max of those projected points addresses this.
    let (
        clusterable_object_aabb_view_xymin_near,
        clusterable_object_aabb_view_xymin_far,
        clusterable_object_aabb_view_xymax_near,
        clusterable_object_aabb_view_xymax_far,
    ) = (
        clusterable_object_aabb_view_min,
        clusterable_object_aabb_view_min
            .xy()
            .extend(clusterable_object_aabb_view_max.z),
        clusterable_object_aabb_view_max
            .xy()
            .extend(clusterable_object_aabb_view_min.z),
        clusterable_object_aabb_view_max,
    );
    let (
        clusterable_object_aabb_clip_xymin_near,
        clusterable_object_aabb_clip_xymin_far,
        clusterable_object_aabb_clip_xymax_near,
        clusterable_object_aabb_clip_xymax_far,
    ) = (
        clip_from_view * clusterable_object_aabb_view_xymin_near.extend(1.0),
        clip_from_view * clusterable_object_aabb_view_xymin_far.extend(1.0),
        clip_from_view * clusterable_object_aabb_view_xymax_near.extend(1.0),
        clip_from_view * clusterable_object_aabb_view_xymax_far.extend(1.0),
    );
    let (
        clusterable_object_aabb_ndc_xymin_near,
        clusterable_object_aabb_ndc_xymin_far,
        clusterable_object_aabb_ndc_xymax_near,
        clusterable_object_aabb_ndc_xymax_far,
    ) = (
        clusterable_object_aabb_clip_xymin_near.xyz() / clusterable_object_aabb_clip_xymin_near.w,
        clusterable_object_aabb_clip_xymin_far.xyz() / clusterable_object_aabb_clip_xymin_far.w,
        clusterable_object_aabb_clip_xymax_near.xyz() / clusterable_object_aabb_clip_xymax_near.w,
        clusterable_object_aabb_clip_xymax_far.xyz() / clusterable_object_aabb_clip_xymax_far.w,
    );
    let (clusterable_object_aabb_ndc_min, clusterable_object_aabb_ndc_max) = (
        clusterable_object_aabb_ndc_xymin_near
            .min(clusterable_object_aabb_ndc_xymin_far)
            .min(clusterable_object_aabb_ndc_xymax_near)
            .min(clusterable_object_aabb_ndc_xymax_far),
        clusterable_object_aabb_ndc_xymin_near
            .max(clusterable_object_aabb_ndc_xymin_far)
            .max(clusterable_object_aabb_ndc_xymax_near)
            .max(clusterable_object_aabb_ndc_xymax_far),
    );

    // clamp to ndc coords without depth
    let (aabb_min_ndc, aabb_max_ndc) = (
        clusterable_object_aabb_ndc_min.xy().clamp(NDC_MIN, NDC_MAX),
        clusterable_object_aabb_ndc_max.xy().clamp(NDC_MIN, NDC_MAX),
    );

    // pack unadjusted z depth into the vecs
    (
        aabb_min_ndc.extend(clusterable_object_aabb_view_min.z),
        aabb_max_ndc.extend(clusterable_object_aabb_view_max.z),
    )
}

// Calculate the intersection of a ray from the eye through the view space position to a z plane
fn line_intersection_to_z_plane(origin: Vec3, p: Vec3, z: f32) -> Vec3 {
    let v = p - origin;
    let t = (z - Vec3::Z.dot(origin)) / Vec3::Z.dot(v);
    origin + t * v
}

// NOTE: Keep in sync with bevy_pbr/src/render/pbr.wgsl
fn view_z_to_z_slice(
    cluster_factors: Vec2,
    z_slices: u32,
    view_z: f32,
    is_orthographic: bool,
) -> u32 {
    let z_slice = if is_orthographic {
        // NOTE: view_z is correct in the orthographic case
        ((view_z - cluster_factors.x) * cluster_factors.y).floor() as u32
    } else {
        // NOTE: had to use -view_z to make it positive else log(negative) is nan
        ((-view_z).ln() * cluster_factors.x - cluster_factors.y + 1.0) as u32
    };
    // NOTE: We use min as we may limit the far z plane used for clustering to be closer than
    // the furthest thing being drawn. This means that we need to limit to the maximum cluster.
    z_slice.min(z_slices - 1)
}

fn clip_to_view(view_from_clip: Mat4, clip: Vec4) -> Vec4 {
    let view = view_from_clip * clip;
    view / view.w
}

fn screen_to_view(screen_size: Vec2, view_from_clip: Mat4, screen: Vec2, ndc_z: f32) -> Vec4 {
    let tex_coord = screen / screen_size;
    let clip = Vec4::new(
        tex_coord.x * 2.0 - 1.0,
        (1.0 - tex_coord.y) * 2.0 - 1.0,
        ndc_z,
        1.0,
    );
    clip_to_view(view_from_clip, clip)
}

// NOTE: This exploits the fact that a x-plane normal has only x and z components
fn get_distance_x(plane: HalfSpace, point: Vec3A, is_orthographic: bool) -> f32 {
    if is_orthographic {
        point.x - plane.d()
    } else {
        // Distance from a point to a plane:
        // signed distance to plane = (nx * px + ny * py + nz * pz + d) / n.length()
        // NOTE: For a x-plane, ny and d are 0 and we have a unit normal
        //                          = nx * px + nz * pz
        plane.normal_d().xz().dot(point.xz())
    }
}

// NOTE: This exploits the fact that a z-plane normal has only a z component
fn project_to_plane_z(z_object: Sphere, z_plane: HalfSpace) -> Option<Sphere> {
    // p = sphere center
    // n = plane normal
    // d = n.p if p is in the plane
    // NOTE: For a z-plane, nx and ny are both 0
    // d = px * nx + py * ny + pz * nz
    //   = pz * nz
    // => pz = d / nz
    let z = z_plane.d() / z_plane.normal_d().z;
    let distance_to_plane = z - z_object.center.z;
    if distance_to_plane.abs() > z_object.radius {
        return None;
    }
    Some(Sphere {
        center: Vec3A::from(z_object.center.xy().extend(z)),
        // hypotenuse length = radius
        // pythagoras = (distance to plane)^2 + b^2 = radius^2
        radius: (z_object.radius * z_object.radius - distance_to_plane * distance_to_plane).sqrt(),
    })
}

// NOTE: This exploits the fact that a y-plane normal has only y and z components
fn project_to_plane_y(
    y_object: Sphere,
    y_plane: HalfSpace,
    is_orthographic: bool,
) -> Option<Sphere> {
    let distance_to_plane = if is_orthographic {
        y_plane.d() - y_object.center.y
    } else {
        -y_object.center.yz().dot(y_plane.normal_d().yz())
    };

    if distance_to_plane.abs() > y_object.radius {
        return None;
    }
    Some(Sphere {
        center: y_object.center + distance_to_plane * y_plane.normal(),
        radius: (y_object.radius * y_object.radius - distance_to_plane * distance_to_plane).sqrt(),
    })
}
