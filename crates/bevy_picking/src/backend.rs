//! This module provides a simple interface for implementing a picking backend.
//!
//! Don't be dissuaded by terminology like "backend"; the idea is dead simple. `bevy_picking`
//! will tell you where pointers are, all you have to do is send an event if the pointers are
//! hitting something. That's it. The rest of this documentation explains the requirements in more
//! detail.
//!
//! Because `bevy_picking` is very loosely coupled with its backends, you can mix and match as
//! many backends as you want. For example, You could use the `rapier` backend to raycast against
//! physics objects, a picking shader backend to pick non-physics meshes, and the `bevy_ui` backend
//! for your UI. The [`PointerHits`]s produced by these various backends will be combined, sorted,
//! and used as a homogeneous input for the picking systems that consume these events.
//!
//! ## Implementation
//!
//! - A picking backend only has one job: read [`PointerLocation`](crate::pointer::PointerLocation)
//!   components and produce [`PointerHits`] events. In plain English, a backend is provided the
//!   location of pointers, and is asked to provide a list of entities under those pointers.
//!
//! - The [`PointerHits`] events produced by a backend do **not** need to be sorted or filtered, all
//!   that is needed is an unordered list of entities and their [`HitData`].
//!
//! - Backends do not need to consider the [`Pickable`](crate::Pickable) component, though they may
//!   use it for optimization purposes. For example, a backend that traverses a spatial hierarchy
//!   may want to early exit if it intersects an entity that blocks lower entities from being
//!   picked.
//!
//! ### Raycasting Backends
//!
//! Backends that require a ray to cast into the scene should use [`ray::RayMap`]. This
//! automatically constructs rays in world space for all cameras and pointers, handling details like
//! viewports and DPI for you.

use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_reflect::Reflect;

/// Common imports for implementing a picking backend.
pub mod prelude {
    pub use super::{ray::RayMap, HitData, PointerHits};
    pub use crate::{
        pointer::{PointerId, PointerLocation},
        PickSet, Pickable,
    };
}

/// An event produced by a picking backend after it has run its hit tests, describing the entities
/// under a pointer.
///
/// Some backends may only support providing the topmost entity; this is a valid limitation of some
/// backends. For example, a picking shader might only have data on the topmost rendered output from
/// its buffer.
#[derive(Event, Debug, Clone)]
pub struct PointerHits {
    /// The pointer associated with this hit test.
    pub pointer: prelude::PointerId,
    /// An unordered collection of entities and their distance (depth) from the cursor.
    pub picks: Vec<(Entity, HitData)>,
    /// Set the order of this group of picks. Normally, this is the
    /// [`bevy_render::camera::Camera::order`].
    ///
    /// Used to allow multiple `PointerHits` submitted for the same pointer to be ordered.
    /// `PointerHits` with a higher `order` will be checked before those with a lower `order`,
    /// regardless of the depth of each entity pick.
    ///
    /// In other words, when pick data is coalesced across all backends, the data is grouped by
    /// pointer, then sorted by order, and checked sequentially, sorting each `PointerHits` by
    /// entity depth. Events with a higher `order` are effectively on top of events with a lower
    /// order.
    ///
    /// ### Why is this an `f32`???
    ///
    /// Bevy UI is special in that it can share a camera with other things being rendered. in order
    /// to properly sort them, we need a way to make `bevy_ui`'s order a tiny bit higher, like adding
    /// 0.5 to the order. We can't use integers, and we want users to be using camera.order by
    /// default, so this is the best solution at the moment.
    pub order: f32,
}

impl PointerHits {
    #[allow(missing_docs)]
    pub fn new(pointer: prelude::PointerId, picks: Vec<(Entity, HitData)>, order: f32) -> Self {
        Self {
            pointer,
            picks,
            order,
        }
    }
}

/// Holds data from a successful pointer hit test. See [`HitData::depth`] for important details.
#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct HitData {
    /// The camera entity used to detect this hit. Useful when you need to find the ray that was
    /// casted for this hit when using a raycasting backend.
    pub camera: Entity,
    /// `depth` only needs to be self-consistent with other [`PointerHits`]s using the same
    /// [`RenderTarget`](bevy_render::camera::RenderTarget). However, it is recommended to use the
    /// distance from the pointer to the hit, measured from the near plane of the camera, to the
    /// point, in world space.
    pub depth: f32,
    /// The position of the intersection in the world, if the data is available from the backend.
    pub position: Option<Vec3>,
    /// The normal vector of the hit test, if the data is available from the backend.
    pub normal: Option<Vec3>,
}

impl HitData {
    #[allow(missing_docs)]
    pub fn new(camera: Entity, depth: f32, position: Option<Vec3>, normal: Option<Vec3>) -> Self {
        Self {
            camera,
            depth,
            position,
            normal,
        }
    }
}

pub mod ray {
    //! Types and systems for constructing rays from cameras and pointers.

    use crate::backend::prelude::{PointerId, PointerLocation};
    use bevy_ecs::prelude::*;
    use bevy_math::Ray3d;
    use bevy_reflect::Reflect;
    use bevy_render::camera::Camera;
    use bevy_transform::prelude::GlobalTransform;
    use bevy_utils::{hashbrown::hash_map::Iter, HashMap};
    use bevy_window::PrimaryWindow;

    /// Identifies a ray constructed from some (pointer, camera) combination. A pointer can be over
    /// multiple cameras, which is why a single pointer may have multiple rays.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Reflect)]
    pub struct RayId {
        /// The camera whose projection was used to calculate the ray.
        pub camera: Entity,
        /// The pointer whose pixel coordinates were used to calculate the ray.
        pub pointer: PointerId,
    }

    impl RayId {
        /// Construct a [`RayId`].
        pub fn new(camera: Entity, pointer: PointerId) -> Self {
            Self { camera, pointer }
        }
    }

    /// A map from [`RayId`] to [`Ray3d`].
    ///
    /// This map is cleared and re-populated every frame before any backends run. Ray-based picking
    /// backends should use this when possible, as it automatically handles viewports, DPI, and
    /// other details of building rays from pointer locations.
    ///
    /// ## Usage
    ///
    /// Iterate over each [`Ray3d`] and its [`RayId`] with [`RayMap::iter`].
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_picking::backend::ray::RayMap;
    /// # use bevy_picking::backend::PointerHits;
    /// // My raycasting backend
    /// pub fn update_hits(ray_map: Res<RayMap>, mut output_events: EventWriter<PointerHits>,) {
    ///     for (&ray_id, &ray) in ray_map.iter() {
    ///         // Run a raycast with each ray, returning any `PointerHits` found.
    ///     }
    /// }
    /// ```
    #[derive(Clone, Debug, Default, Resource)]
    pub struct RayMap {
        map: HashMap<RayId, Ray3d>,
    }

    impl RayMap {
        /// Iterates over all world space rays for every picking pointer.
        pub fn iter(&self) -> Iter<'_, RayId, Ray3d> {
            self.map.iter()
        }

        /// The hash map of all rays cast in the current frame.
        pub fn map(&self) -> &HashMap<RayId, Ray3d> {
            &self.map
        }

        /// Clears the [`RayMap`] and re-populates it with one ray for each
        /// combination of pointer entity and camera entity where the pointer
        /// intersects the camera's viewport.
        pub fn repopulate(
            mut ray_map: ResMut<Self>,
            primary_window_entity: Query<Entity, With<PrimaryWindow>>,
            cameras: Query<(Entity, &Camera, &GlobalTransform)>,
            pointers: Query<(&PointerId, &PointerLocation)>,
        ) {
            ray_map.map.clear();

            for (camera_entity, camera, camera_tfm) in &cameras {
                if !camera.is_active {
                    continue;
                }

                for (&pointer_id, pointer_loc) in &pointers {
                    if let Some(ray) =
                        make_ray(&primary_window_entity, camera, camera_tfm, pointer_loc)
                    {
                        ray_map
                            .map
                            .insert(RayId::new(camera_entity, pointer_id), ray);
                    }
                }
            }
        }
    }

    fn make_ray(
        primary_window_entity: &Query<Entity, With<PrimaryWindow>>,
        camera: &Camera,
        camera_tfm: &GlobalTransform,
        pointer_loc: &PointerLocation,
    ) -> Option<Ray3d> {
        let pointer_loc = pointer_loc.location()?;
        if !pointer_loc.is_in_viewport(camera, primary_window_entity) {
            return None;
        }
        let mut viewport_pos = pointer_loc.position;
        if let Some(viewport) = &camera.viewport {
            let viewport_logical = camera.to_logical(viewport.physical_position)?;
            viewport_pos -= viewport_logical;
        }
        camera.viewport_to_world(camera_tfm, viewport_pos)
    }
}
