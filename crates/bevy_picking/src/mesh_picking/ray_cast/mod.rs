//! Ray casting for meshes.
//!
//! See the [`MeshRayCast`] system parameter for more information.

mod intersections;

use bevy_derive::{Deref, DerefMut};

use bevy_camera::{
    primitives::Aabb,
    visibility::{InheritedVisibility, ViewVisibility},
};
use bevy_math::{bounding::Aabb3d, Ray3d};
use bevy_mesh::{Mesh, Mesh2d, Mesh3d};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

use intersections::*;
pub use intersections::{ray_aabb_intersection_3d, ray_mesh_intersection, RayMeshHit};

use bevy_asset::{Assets, Handle};
use bevy_ecs::{prelude::*, system::lifetimeless::Read, system::SystemParam};
use bevy_math::FloatOrd;
use bevy_transform::components::GlobalTransform;
use tracing::*;

/// How a ray cast should handle [`Visibility`](bevy_camera::visibility::Visibility).
#[derive(Clone, Copy, Reflect)]
#[reflect(Clone)]
pub enum RayCastVisibility {
    /// Completely ignore visibility checks. Hidden items can still be ray cast against.
    Any,
    /// Only cast rays against entities that are visible in the hierarchy. See [`Visibility`](bevy_camera::visibility::Visibility).
    Visible,
    /// Only cast rays against entities that are visible in the hierarchy and visible to a camera or
    /// light. See [`Visibility`](bevy_camera::visibility::Visibility).
    VisibleInView,
}

/// Settings for a ray cast.
#[derive(Clone)]
pub struct MeshRayCastSettings<'a> {
    /// Determines how ray casting should consider [`Visibility`](bevy_camera::visibility::Visibility).
    pub visibility: RayCastVisibility,
    /// A predicate that is applied for every entity that ray casts are performed against.
    /// Only entities that return `true` will be considered.
    pub filter: &'a dyn Fn(Entity) -> bool,
    /// A function that is run every time a hit is found. Ray casting will continue to check for hits
    /// along the ray as long as this returns `false`.
    pub early_exit_test: &'a dyn Fn(Entity) -> bool,
}

impl<'a> MeshRayCastSettings<'a> {
    /// Set the filter to apply to the ray cast.
    pub fn with_filter(mut self, filter: &'a impl Fn(Entity) -> bool) -> Self {
        self.filter = filter;
        self
    }

    /// Set the early exit test to apply to the ray cast.
    pub fn with_early_exit_test(mut self, early_exit_test: &'a impl Fn(Entity) -> bool) -> Self {
        self.early_exit_test = early_exit_test;
        self
    }

    /// Set the [`RayCastVisibility`] setting to apply to the ray cast.
    pub fn with_visibility(mut self, visibility: RayCastVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// This ray cast should exit as soon as the nearest hit is found.
    pub fn always_early_exit(self) -> Self {
        self.with_early_exit_test(&|_| true)
    }

    /// This ray cast should check all entities whose AABB intersects the ray and return all hits.
    pub fn never_early_exit(self) -> Self {
        self.with_early_exit_test(&|_| false)
    }
}

impl<'a> Default for MeshRayCastSettings<'a> {
    fn default() -> Self {
        Self {
            visibility: RayCastVisibility::VisibleInView,
            filter: &|_| true,
            early_exit_test: &|_| true,
        }
    }
}

/// Determines whether backfaces should be culled or included in ray intersection tests.
///
/// By default, backfaces are culled.
#[derive(Copy, Clone, Default, Reflect)]
#[reflect(Default, Clone)]
pub enum Backfaces {
    /// Cull backfaces.
    #[default]
    Cull,
    /// Include backfaces.
    Include,
}

/// Disables backface culling for [ray casts](MeshRayCast) on this entity.
#[derive(Component, Copy, Clone, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct RayCastBackfaces;

/// A simplified mesh component that can be used for [ray casting](super::MeshRayCast).
///
/// Consider using this component for complex meshes that don't need perfectly accurate ray casting.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct SimplifiedMesh(pub Handle<Mesh>);

type MeshFilter = Or<(With<Mesh3d>, With<Mesh2d>, With<SimplifiedMesh>)>;

/// Add this ray casting [`SystemParam`] to your system to cast rays into the world with an
/// immediate-mode API. Call `cast_ray` to immediately perform a ray cast and get a result.
///
/// Under the hood, this is a collection of regular bevy queries, resources, and local parameters
/// that are added to your system.
///
/// ## Usage
///
/// The following system casts a ray into the world with the ray positioned at the origin, pointing in
/// the X-direction, and returns a list of intersections:
///
/// ```
/// # use bevy_math::prelude::*;
/// # use bevy_picking::prelude::*;
/// fn ray_cast_system(mut ray_cast: MeshRayCast) {
///     let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
///     let hits = ray_cast.cast_ray(ray, &MeshRayCastSettings::default());
/// }
/// ```
///
/// ## Configuration
///
/// You can specify the behavior of the ray cast using [`MeshRayCastSettings`]. This allows you to filter out
/// entities, configure early-out behavior, and set whether the [`Visibility`](bevy_camera::visibility::Visibility)
/// of an entity should be considered.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_math::prelude::*;
/// # use bevy_picking::prelude::*;
/// # #[derive(Component)]
/// # struct Foo;
/// fn ray_cast_system(mut ray_cast: MeshRayCast, foo_query: Query<(), With<Foo>>) {
///     let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
///
///     // Only ray cast against entities with the `Foo` component.
///     let filter = |entity| foo_query.contains(entity);
///
///     // Never early-exit. Note that you can change behavior per-entity.
///     let early_exit_test = |_entity| false;
///
///     // Ignore the visibility of entities. This allows ray casting hidden entities.
///     let visibility = RayCastVisibility::Any;
///
///     let settings = MeshRayCastSettings::default()
///         .with_filter(&filter)
///         .with_early_exit_test(&early_exit_test)
///         .with_visibility(visibility);
///
///     // Cast the ray with the settings, returning a list of intersections.
///     let hits = ray_cast.cast_ray(ray, &settings);
/// }
/// ```
#[derive(SystemParam)]
pub struct MeshRayCast<'w, 's> {
    #[doc(hidden)]
    pub meshes: Res<'w, Assets<Mesh>>,
    #[doc(hidden)]
    pub hits: Local<'s, Vec<(FloatOrd, (Entity, RayMeshHit))>>,
    #[doc(hidden)]
    pub output: Local<'s, Vec<(Entity, RayMeshHit)>>,
    #[doc(hidden)]
    pub culled_list: Local<'s, Vec<(FloatOrd, Entity)>>,
    #[doc(hidden)]
    pub culling_query: Query<
        'w,
        's,
        (
            Read<InheritedVisibility>,
            Read<ViewVisibility>,
            Read<Aabb>,
            Read<GlobalTransform>,
            Entity,
        ),
        MeshFilter,
    >,
    #[doc(hidden)]
    pub mesh_query: Query<
        'w,
        's,
        (
            Option<Read<Mesh2d>>,
            Option<Read<Mesh3d>>,
            Option<Read<SimplifiedMesh>>,
            Has<RayCastBackfaces>,
            Read<GlobalTransform>,
        ),
        MeshFilter,
    >,
}

impl<'w, 's> MeshRayCast<'w, 's> {
    /// Casts the `ray` into the world and returns a sorted list of intersections, nearest first.
    pub fn cast_ray(
        &mut self,
        ray: Ray3d,
        settings: &MeshRayCastSettings,
    ) -> &[(Entity, RayMeshHit)] {
        let ray_cull = info_span!("ray culling");
        let ray_cull_guard = ray_cull.enter();

        self.hits.clear();
        self.culled_list.clear();
        self.output.clear();

        // Check all entities to see if the ray intersects the AABB. Use this to build a short list
        // of entities that are in the path of the ray.
        let (aabb_hits_tx, aabb_hits_rx) = crossbeam_channel::unbounded::<(FloatOrd, Entity)>();
        let visibility_setting = settings.visibility;
        self.culling_query.par_iter().for_each(
            |(inherited_visibility, view_visibility, aabb, transform, entity)| {
                let should_ray_cast = match visibility_setting {
                    RayCastVisibility::Any => true,
                    RayCastVisibility::Visible => inherited_visibility.get(),
                    RayCastVisibility::VisibleInView => view_visibility.get(),
                };
                if should_ray_cast
                    && let Some(distance) = ray_aabb_intersection_3d(
                        ray,
                        &Aabb3d::new(aabb.center, aabb.half_extents),
                        &transform.affine(),
                    )
                {
                    aabb_hits_tx.send((FloatOrd(distance), entity)).ok();
                }
            },
        );
        *self.culled_list = aabb_hits_rx.try_iter().collect();

        // Sort by the distance along the ray.
        self.culled_list.sort_by_key(|(aabb_near, _)| *aabb_near);

        drop(ray_cull_guard);

        // Perform ray casts against the culled entities.
        let mut nearest_blocking_hit = FloatOrd(f32::INFINITY);
        let ray_cast_guard = debug_span!("ray_cast");
        self.culled_list
            .iter()
            .filter(|(_, entity)| (settings.filter)(*entity))
            .for_each(|(aabb_near, entity)| {
                // Get the mesh components and transform.
                let Ok((mesh2d, mesh3d, simplified_mesh, has_backfaces, transform)) =
                    self.mesh_query.get(*entity)
                else {
                    return;
                };

                // Get the underlying mesh handle. One of these will always be `Some` because of the query filters.
                let Some(mesh_handle) = simplified_mesh
                    .map(|m| &m.0)
                    .or(mesh3d.map(|m| &m.0).or(mesh2d.map(|m| &m.0)))
                else {
                    return;
                };

                // Is it even possible the mesh could be closer than the current best?
                if *aabb_near > nearest_blocking_hit {
                    return;
                }

                // Does the mesh handle resolve?
                let Some(mesh) = self.meshes.get(mesh_handle) else {
                    return;
                };

                // Backfaces of 2d meshes are never culled, unlike 3d meshes.
                let backfaces = match (has_backfaces, mesh2d.is_some()) {
                    (false, false) => Backfaces::Cull,
                    _ => Backfaces::Include,
                };

                // Perform the actual ray cast.
                let _ray_cast_guard = ray_cast_guard.enter();
                let transform = transform.affine();
                let intersection = ray_intersection_over_mesh(mesh, &transform, ray, backfaces);

                if let Some(intersection) = intersection {
                    let distance = FloatOrd(intersection.distance);
                    if (settings.early_exit_test)(*entity) && distance < nearest_blocking_hit {
                        // The reason we don't just return here is because right now we are
                        // going through the AABBs in order, but that doesn't mean that an
                        // AABB that starts further away can't end up with a closer hit than
                        // an AABB that starts closer. We need to keep checking AABBs that
                        // could possibly contain a nearer hit.
                        nearest_blocking_hit = distance.min(nearest_blocking_hit);
                    }
                    self.hits.push((distance, (*entity, intersection)));
                };
            });

        self.hits.retain(|(dist, _)| *dist <= nearest_blocking_hit);
        self.hits.sort_by_key(|(k, _)| *k);
        let hits = self.hits.iter().map(|(_, (e, i))| (*e, i.to_owned()));
        self.output.extend(hits);
        self.output.as_ref()
    }
}
