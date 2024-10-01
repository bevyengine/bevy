//! A raycasting backend for `bevy_mod_picking` that uses `bevy_mod_raycast` for raycasting.
//!
//! # Usage
//!
//! If a pointer passes through this camera's render target, it will automatically shoot rays into
//! the scene and will be able to pick things.
//!
//! To ignore an entity, you can add [`Pickable::IGNORE`] to it, and it will be ignored during
//! raycasting.
//!
//! For fine-grained control, see the [`RaycastBackendSettings::require_markers`] setting.
//!

#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![warn(missing_docs)]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_picking::{
    backend::{ray::RayMap, HitData, PointerHits},
    prelude::*,
    PickSet,
};
use bevy_reflect::prelude::*;
use bevy_render::{prelude::*, view::RenderLayers};
use raycast::immediate::{Raycast, RaycastSettings, RaycastVisibility};

pub mod raycast;

/// Commonly used imports for the [`bevy_picking_raycast`](crate) crate.
pub mod prelude {
    pub use crate::RaycastBackend;
}

/// Runtime settings for the [`RaycastBackend`].
#[derive(Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct RaycastBackendSettings {
    /// When set to `true` raycasting will only happen between cameras and entities marked with
    /// [`RaycastPickable`]. Off by default. This setting is provided to give you fine-grained
    /// control over which cameras and entities should be used by the raycast backend at runtime.
    pub require_markers: bool,
    /// When set to Ignore, hidden items can be raycasted against.
    /// See [`RaycastSettings::visibility`] for more information.
    pub raycast_visibility: RaycastVisibility,
}

impl Default for RaycastBackendSettings {
    fn default() -> Self {
        Self {
            require_markers: false,
            raycast_visibility: RaycastVisibility::MustBeVisibleAndInView,
        }
    }
}

/// Optional. Marks cameras and target entities that should be used in the raycast picking backend.
/// Only needed if [`RaycastBackendSettings::require_markers`] is set to true.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RaycastPickable;

/// Adds the raycasting picking backend to your app.
#[derive(Clone)]
pub struct RaycastBackend;
impl Plugin for RaycastBackend {
    fn build(&self, app: &mut App) {
        app.init_resource::<RaycastBackendSettings>()
            .add_systems(PreUpdate, update_hits.in_set(PickSet::Backend))
            .register_type::<RaycastPickable>()
            .register_type::<RaycastBackendSettings>();
    }
}

/// Raycasts into the scene using [`RaycastBackendSettings`] and [`PointerLocation`]s, then outputs
/// [`PointerHits`].
pub fn update_hits(
    backend_settings: Res<RaycastBackendSettings>,
    ray_map: Res<RayMap>,
    picking_cameras: Query<(&Camera, Option<&RaycastPickable>, Option<&RenderLayers>)>,
    pickables: Query<&Pickable>,
    marked_targets: Query<&RaycastPickable>,
    layers: Query<&RenderLayers>,
    mut raycast: Raycast,
    mut output_events: EventWriter<PointerHits>,
) {
    for (&ray_id, &ray) in ray_map.map().iter() {
        let Ok((camera, cam_pickable, cam_layers)) = picking_cameras.get(ray_id.camera) else {
            continue;
        };
        if backend_settings.require_markers && cam_pickable.is_none() {
            continue;
        }

        let cam_layers = cam_layers.to_owned().unwrap_or_default();

        let settings = RaycastSettings {
            visibility: backend_settings.raycast_visibility,
            filter: &|entity| {
                let marker_requirement =
                    !backend_settings.require_markers || marked_targets.get(entity).is_ok();

                // Other entities missing render layers are on the default layer 0
                let entity_layers = layers.get(entity).cloned().unwrap_or_default();
                let render_layers_match = cam_layers.intersects(&entity_layers);

                let is_pickable = pickables
                    .get(entity)
                    .map(|p| p.is_hoverable)
                    .unwrap_or(true);

                marker_requirement && render_layers_match && is_pickable
            },
            early_exit_test: &|entity_hit| {
                pickables
                    .get(entity_hit)
                    .is_ok_and(|pickable| pickable.should_block_lower)
            },
        };
        let picks = raycast
            .cast_ray(ray, &settings)
            .iter()
            .map(|(entity, hit)| {
                let hit_data = HitData::new(
                    ray_id.camera,
                    hit.distance(),
                    Some(hit.position()),
                    Some(hit.normal()),
                );
                (*entity, hit_data)
            })
            .collect::<Vec<_>>();
        let order = camera.order as f32;
        if !picks.is_empty() {
            output_events.send(PointerHits::new(ray_id.pointer, picks, order));
        }
    }
}
