//! # Immediate Mode Raycasting API
//!
//! See the `minimal` example for reference.
//!
//! This is the simplest way to get started. Add the [`Raycast`] [`SystemParam`] to your system, and
//! call [`Raycast::cast_ray`], to get a list of intersections. Raycasts are performed immediately
//! when you call the `cast_ray` method. See the [`Raycast`] documentation for more details. You
//! don't even need to add a plugin to your application.

use bevy_asset::{Assets, Handle};
use bevy_ecs::{prelude::*, system::lifetimeless::Read, system::SystemParam};
use bevy_math::{FloatOrd, Ray3d};
use bevy_reflect::Reflect;
use bevy_render::{prelude::*, primitives::Aabb};
use bevy_transform::components::GlobalTransform;
use bevy_utils::tracing::*;

use super::{
    markers::{NoBackfaceCulling, SimplifiedMesh},
    primitives::{intersects_aabb, IntersectionData},
    raycast::{ray_intersection_over_mesh, Backfaces},
};

#[cfg(feature = "debug")]
use {
    bevy_gizmos::gizmos::Gizmos,
    bevy_math::{Quat, Vec3},
};

use crate::prelude::*;

/// How a raycast should handle visibility
#[derive(Clone, Copy, Reflect)]
pub enum RaycastVisibility {
    /// Completely ignore visibility checks. Hidden items can still be raycasted against.
    Ignore,
    /// Only raycast against entities that are visible in the hierarchy; see [`Visibility`].
    MustBeVisible,
    /// Only raycast against entities that are visible in the hierarchy and visible to a camera or
    /// light; see [`Visibility`].
    MustBeVisibleAndInView,
}

/// Settings for a raycast.
#[derive(Clone)]
pub struct RaycastSettings<'a> {
    /// Determines how raycasting should consider entity visibility.
    pub visibility: RaycastVisibility,
    /// A filtering function that is applied to every entity that is raycasted. Only entities that
    /// return `true` will be considered.
    pub filter: &'a dyn Fn(Entity) -> bool,
    /// A function that is run every time a hit is found. Raycasting will continue to check for hits
    /// along the ray as long as this returns false.
    pub early_exit_test: &'a dyn Fn(Entity) -> bool,
}

impl<'a> RaycastSettings<'a> {
    /// Set the filter to apply to the raycast.
    pub fn with_filter(mut self, filter: &'a impl Fn(Entity) -> bool) -> Self {
        self.filter = filter;
        self
    }

    /// Set the early exit test to apply to the raycast.
    pub fn with_early_exit_test(mut self, early_exit_test: &'a impl Fn(Entity) -> bool) -> Self {
        self.early_exit_test = early_exit_test;
        self
    }

    /// Set the [`RaycastVisibility`] setting to apply to the raycast.
    pub fn with_visibility(mut self, visibility: RaycastVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// This raycast should exit as soon as the nearest hit is found.
    pub fn always_early_exit(self) -> Self {
        self.with_early_exit_test(&|_| true)
    }

    /// This raycast should check all entities whose AABB intersects the ray and return all hits.
    pub fn never_early_exit(self) -> Self {
        self.with_early_exit_test(&|_| false)
    }
}

impl<'a> Default for RaycastSettings<'a> {
    fn default() -> Self {
        Self {
            visibility: RaycastVisibility::MustBeVisibleAndInView,
            filter: &|_| true,
            early_exit_test: &|_| true,
        }
    }
}

#[cfg(feature = "2d")]
type MeshFilter = Or<(With<Handle<Mesh>>, With<bevy_sprite::Mesh2dHandle>)>;
#[cfg(not(feature = "2d"))]
type MeshFilter = With<Handle<Mesh>>;

/// Add this raycasting [`SystemParam`] to your system to raycast into the world with an
/// immediate-mode API. Call `cast_ray` to immediately perform a raycast and get a result. Under the
/// hood, this is a collection of regular bevy queries, resources, and locals that are added to your
/// system.
///
/// ## Usage
///
/// The following system raycasts into the world with a ray positioned at the origin, pointing in
/// the x-direction, and returns a list of intersections:
///
/// ```
/// # use bevy_mod_raycast::prelude::*;
/// # use bevy::prelude::*;
/// fn raycast_system(mut raycast: Raycast) {
///     let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
///     let hits = raycast.cast_ray(ray, &RaycastSettings::default());
/// }
/// ```
/// ## Configuration
///
/// You can specify behavior of the raycast using [`RaycastSettings`]. This allows you to filter out
/// entities, configure early-out, and set whether the [`Visibility`] of an entity should be
/// considered.
///
/// ```
/// # use bevy_mod_raycast::prelude::*;
/// # use bevy::prelude::*;
/// # #[derive(Component)]
/// # struct Foo;
/// fn raycast_system(mut raycast: Raycast, foo_query: Query<(), With<Foo>>) {
///     let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
///
///     // Only raycast against entities with the `Foo` component.
///     let filter = |entity| foo_query.contains(entity);
///     // Never early-exit. Note that you can change behavior per-entity.
///     let early_exit_test = |_entity| false;
///     // Ignore the visibility of entities. This allows raycasting hidden entities.
///     let visibility = RaycastVisibility::Ignore;
///
///     let settings = RaycastSettings::default()
///         .with_filter(&filter)
///         .with_early_exit_test(&early_exit_test)
///         .with_visibility(visibility);
///
///     let hits = raycast.cast_ray(ray, &settings);
/// }
/// ```
#[derive(SystemParam)]
pub struct Raycast<'w, 's> {
    #[doc(hidden)]
    pub meshes: Res<'w, Assets<Mesh>>,
    #[doc(hidden)]
    pub hits: Local<'s, Vec<(FloatOrd, (Entity, IntersectionData))>>,
    #[doc(hidden)]
    pub output: Local<'s, Vec<(Entity, IntersectionData)>>,
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
            Read<Handle<Mesh>>,
            Option<Read<SimplifiedMesh>>,
            Option<Read<NoBackfaceCulling>>,
            Read<GlobalTransform>,
        ),
    >,
    #[cfg(feature = "2d")]
    #[doc(hidden)]
    pub mesh2d_query: Query<
        'w,
        's,
        (
            Read<bevy_sprite::Mesh2dHandle>,
            Option<Read<SimplifiedMesh>>,
            Read<GlobalTransform>,
        ),
    >,
}

impl<'w, 's> Raycast<'w, 's> {
    #[cfg(feature = "debug")]
    /// Like [`Raycast::cast_ray`], but debug-draws the ray and intersection.
    pub fn debug_cast_ray(
        &mut self,
        ray: Ray3d,
        settings: &RaycastSettings,
        gizmos: &mut Gizmos,
    ) -> &[(Entity, IntersectionData)] {
        use bevy_color::palettes::css;
        use bevy_math::Dir3;

        let orientation = Quat::from_rotation_arc(Vec3::NEG_Z, *ray.direction);
        gizmos.ray(ray.origin, *ray.direction, css::BLUE);
        gizmos.sphere(ray.origin, orientation, 0.1, css::BLUE);

        let hits = self.cast_ray(ray, settings);

        for (is_first, intersection) in hits
            .iter()
            .map(|i| i.1.clone())
            .enumerate()
            .map(|(i, hit)| (i == 0, hit))
        {
            let color = match is_first {
                true => css::GREEN,
                false => css::PINK,
            };
            gizmos.ray(intersection.position(), intersection.normal(), color);
            gizmos.circle(
                intersection.position(),
                Dir3::new_unchecked(intersection.normal().normalize()),
                0.1,
                color,
            );
        }

        if let Some(hit) = hits.first() {
            debug!("{:?}", hit);
        }

        hits
    }

    /// Casts the `ray` into the world and returns a sorted list of intersections, nearest first.
    pub fn cast_ray(
        &mut self,
        ray: Ray3d,
        settings: &RaycastSettings,
    ) -> &[(Entity, IntersectionData)] {
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
                let should_raycast = match visibility_setting {
                    RaycastVisibility::Ignore => true,
                    RaycastVisibility::MustBeVisible => inherited_visibility.get(),
                    RaycastVisibility::MustBeVisibleAndInView => view_visibility.get(),
                };
                if should_raycast {
                    if let Some([near, _]) = intersects_aabb(ray, aabb, &transform.compute_matrix())
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
        let raycast_guard = debug_span!("raycast");
        self.culled_list
            .iter()
            .filter(|(_, entity)| (settings.filter)(*entity))
            .for_each(|(aabb_near, entity)| {
                let mut raycast_mesh =
                    |mesh_handle: &Handle<Mesh>,
                     simplified_mesh: Option<&SimplifiedMesh>,
                     no_backface_culling: Option<&NoBackfaceCulling>,
                     transform: &GlobalTransform| {
                        // Is it even possible the mesh could be closer than the current best?
                        if *aabb_near > nearest_blocking_hit {
                            return;
                        }

                        // Does the mesh handle resolve?
                        let mesh_handle = simplified_mesh.map(|m| &m.mesh).unwrap_or(mesh_handle);
                        let Some(mesh) = self.meshes.get(mesh_handle) else {
                            return;
                        };

                        let _raycast_guard = raycast_guard.enter();
                        let backfaces = match no_backface_culling {
                            Some(_) => Backfaces::Include,
                            None => Backfaces::Cull,
                        };
                        let transform = transform.compute_matrix();
                        let intersection =
                            ray_intersection_over_mesh(mesh, &transform, ray, backfaces);
                        if let Some(intersection) = intersection {
                            let distance = FloatOrd(intersection.distance());
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

                if let Ok((mesh, simp_mesh, culling, transform)) = self.mesh_query.get(*entity) {
                    raycast_mesh(mesh, simp_mesh, culling, transform);
                }

                #[cfg(feature = "2d")]
                if let Ok((mesh, simp_mesh, transform)) = self.mesh2d_query.get(*entity) {
                    raycast_mesh(&mesh.0, simp_mesh, Some(&NoBackfaceCulling), transform);
                }
            });

        self.hits.retain(|(dist, _)| *dist <= nearest_blocking_hit);
        self.hits.sort_by_key(|(k, _)| *k);
        let hits = self.hits.iter().map(|(_, (e, i))| (*e, i.to_owned()));
        *self.output = hits.collect();
        self.output.as_ref()
    }
}
