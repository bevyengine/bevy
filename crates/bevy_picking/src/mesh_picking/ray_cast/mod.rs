//! Ray casting on meshes.

mod intersections;
mod simplified_mesh;

pub use simplified_mesh::*;

use bevy_math::Ray3d;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::mesh::Mesh;

pub use intersections::RayMeshHit;
use intersections::*;

use bevy_asset::{Assets, Handle};
use bevy_ecs::{prelude::*, system::lifetimeless::Read, system::SystemParam};
use bevy_math::FloatOrd;
use bevy_render::{prelude::*, primitives::Aabb};
use bevy_transform::components::GlobalTransform;
use bevy_utils::tracing::*;

/// How a ray cast should handle visibility.
#[derive(Clone, Copy, Reflect)]
pub enum RayCastVisibility {
    /// Completely ignore visibility checks. Hidden items can still be ray casted against.
    Any,
    /// Only ray cast against entities that are visible in the hierarchy. See [`Visibility`].
    Visible,
    /// Only ray cast against entities that are visible in the hierarchy and visible to a camera or
    /// light. See [`Visibility`].
    VisibleAndInView,
}

/// Settings for a ray cast.
#[derive(Clone)]
pub struct RayCastSettings<'a> {
    /// Determines how ray casting should consider entity visibility.
    pub visibility: RayCastVisibility,
    /// Determines how ray casting should handle backfaces.
    pub backfaces: Backfaces,
    /// A filtering function that is applied to every entity that is ray casted. Only entities that
    /// return `true` will be considered.
    pub filter: &'a dyn Fn(Entity) -> bool,
    /// A function that is run every time a hit is found. Ray casting will continue to check for hits
    /// along the ray as long as this returns false.
    pub early_exit_test: &'a dyn Fn(Entity) -> bool,
}

impl<'a> RayCastSettings<'a> {
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

impl<'a> Default for RayCastSettings<'a> {
    fn default() -> Self {
        Self {
            visibility: RayCastVisibility::VisibleAndInView,
            backfaces: Backfaces::default(),
            filter: &|_| true,
            early_exit_test: &|_| true,
        }
    }
}

/// Determines whether backfaces should be culled or included in intersection checks.
#[derive(Copy, Clone, Default, Reflect)]
#[reflect(Default)]
pub enum Backfaces {
    /// Cull backfaces.
    #[default]
    Cull,
    /// Include backfaces.
    Include,
}

type MeshFilter = Or<(With<Mesh3d>, With<Mesh2d>)>;

/// Add this raycasting [`SystemParam`] to your system to ray cast into the world with an
/// immediate-mode API. Call `cast_ray` to immediately perform a ray cast and get a result. Under the
/// hood, this is a collection of regular bevy queries, resources, and locals that are added to your
/// system.
///
/// ## Usage
///
/// The following system ray casts into the world with a ray positioned at the origin, pointing in
/// the x-direction, and returns a list of intersections:
///
/// ```
/// # use bevy::prelude::*;
/// fn ray_cast_system(mut raycast: MeshRayCast) {
///     let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
///     let hits = raycast.cast_ray(ray, &RayCastSettings::default());
/// }
/// ```
///
/// ## Configuration
///
/// You can specify behavior of the ray cast using [`RayCastSettings`]. This allows you to filter out
/// entities, configure early-out, and set whether the [`Visibility`] of an entity should be
/// considered.
///
/// ```
/// # use bevy::prelude::*;
/// # #[derive(Component)]
/// # struct Foo;
/// fn ray_cast_system(mut ray_cast: MeshRayCast, foo_query: Query<(), With<Foo>>) {
///     let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
///
///     // Only ray cast against entities with the `Foo` component.
///     let filter = |entity| foo_query.contains(entity);
///     // Never early-exit. Note that you can change behavior per-entity.
///     let early_exit_test = |_entity| false;
///     // Ignore the visibility of entities. This allows ray casting hidden entities.
///     let visibility = RayCastVisibility::Ignore;
///
///     let settings = RayCastSettings::default()
///         .with_filter(&filter)
///         .with_early_exit_test(&early_exit_test)
///         .with_visibility(visibility);
///
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
            Read<Mesh3d>,
            Option<Read<SimplifiedMesh>>,
            Read<GlobalTransform>,
        ),
    >,
    #[doc(hidden)]
    pub mesh2d_query: Query<
        'w,
        's,
        (
            Read<Mesh2d>,
            Option<Read<SimplifiedMesh>>,
            Read<GlobalTransform>,
        ),
    >,
}

impl<'w, 's> MeshRayCast<'w, 's> {
    /// Casts the `ray` into the world and returns a sorted list of intersections, nearest first.
    pub fn cast_ray(&mut self, ray: Ray3d, settings: &RayCastSettings) -> &[(Entity, RayMeshHit)] {
        let ray_cull = info_span!("ray culling");
        let ray_cull_guard = ray_cull.enter();

        self.hits.clear();
        self.culled_list.clear();
        self.output.clear();

        // Check all entities to see if the ray intersects the AABB, use this to build a short list
        // of entities that are in the path of the ray.
        let (aabb_hits_tx, aabb_hits_rx) = crossbeam_channel::unbounded::<(FloatOrd, Entity)>();
        let visibility_setting = settings.visibility;
        self.culling_query.par_iter().for_each(
            |(inherited_visibility, view_visibility, aabb, transform, entity)| {
                let should_ray_cast = match visibility_setting {
                    RayCastVisibility::Any => true,
                    RayCastVisibility::Visible => inherited_visibility.get(),
                    RayCastVisibility::VisibleAndInView => view_visibility.get(),
                };
                if should_ray_cast {
                    if let Some([near, _]) =
                        ray_aabb_intersection_3d(ray, aabb, &transform.compute_matrix())
                            .filter(|[_, far]| *far >= 0.0)
                    {
                        aabb_hits_tx.send((FloatOrd(near), entity)).ok();
                    }
                }
            },
        );
        *self.culled_list = aabb_hits_rx.try_iter().collect();
        self.culled_list.sort_by_key(|(aabb_near, _)| *aabb_near);
        drop(ray_cull_guard);

        let mut nearest_blocking_hit = FloatOrd(f32::INFINITY);
        let ray_cast_guard = debug_span!("ray_cast");
        self.culled_list
            .iter()
            .filter(|(_, entity)| (settings.filter)(*entity))
            .for_each(|(aabb_near, entity)| {
                let mut ray_cast_mesh =
                    |mesh_handle: &Handle<Mesh>,
                     simplified_mesh: Option<&SimplifiedMesh>,
                     transform: &GlobalTransform| {
                        // Is it even possible the mesh could be closer than the current best?
                        if *aabb_near > nearest_blocking_hit {
                            return;
                        }

                        // Does the mesh handle resolve?
                        let mesh_handle = simplified_mesh.map(|m| &m.0).unwrap_or(mesh_handle);
                        let Some(mesh) = self.meshes.get(mesh_handle) else {
                            return;
                        };

                        let _ray_cast_guard = ray_cast_guard.enter();
                        let transform = transform.compute_matrix();
                        let intersection =
                            ray_intersection_over_mesh(mesh, &transform, ray, settings.backfaces);
                        if let Some(intersection) = intersection {
                            let distance = FloatOrd(intersection.distance);
                            if (settings.early_exit_test)(*entity)
                                && distance < nearest_blocking_hit
                            {
                                // The reason we don't just return here is because right now we are
                                // going through the AABBs in order, but that doesn't mean that an
                                // AABB that starts further away cant end up with a closer hit than
                                // an AABB that starts closer. We need to keep checking AABBs that
                                // could possibly contain a nearer hit.
                                nearest_blocking_hit = distance.min(nearest_blocking_hit);
                            }
                            self.hits.push((distance, (*entity, intersection)));
                        };
                    };

                if let Ok((mesh, simplified_mesh, transform)) = self.mesh_query.get(*entity) {
                    ray_cast_mesh(mesh, simplified_mesh, transform);
                }

                if let Ok((mesh, simplified_mesh, transform)) = self.mesh2d_query.get(*entity) {
                    ray_cast_mesh(&mesh.0, simplified_mesh, transform);
                }
            });

        self.hits.retain(|(dist, _)| *dist <= nearest_blocking_hit);
        self.hits.sort_by_key(|(k, _)| *k);
        let hits = self.hits.iter().map(|(_, (e, i))| (*e, i.to_owned()));
        *self.output = hits.collect();
        self.output.as_ref()
    }
}
