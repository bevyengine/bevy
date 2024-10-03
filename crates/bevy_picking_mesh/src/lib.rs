//! A ray casting backend for `bevy_picking`.
//!
//! # Usage
//!
//! If a pointer passes through this camera's render target, it will automatically shoot rays into
//! the scene and will be able to pick things.
//!
//! To ignore an entity, you can add [`Pickable::IGNORE`] to it, and it will be ignored during
//! ray casting.
//!
//! For fine-grained control, see the [`RayCastBackendSettings::require_markers`] setting.

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
use ray_cast::{Backfaces, MeshRayCast, RayCastSettings, RayCastVisibility};

pub mod ray_cast;

/// Commonly used imports for the [`bevy_picking_raycast`](crate) crate.
pub mod prelude {
    pub use crate::{
        ray_cast::{Backfaces, MeshRayCast, RayCastSettings, RayCastVisibility, RayTriangleHit},
        RayCastBackend,
    };
}

/// Runtime settings for the [`RayCastBackend`].
#[derive(Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct RayCastBackendSettings {
    /// When set to `true` ray casting will only happen between cameras and entities marked with
    /// [`RayCastPickable`]. Off by default. This setting is provided to give you fine-grained
    /// control over which cameras and entities should be used by the ray cast backend at runtime.
    pub require_markers: bool,
    /// When set to Ignore, hidden items can be raycasted against.
    /// See [`RayCastSettings::visibility`] for more information.
    pub raycast_visibility: RayCastVisibility,
    /// When set to [`Backfaces::Cull`], backfaces will be ignored during ray casting.
    pub backfaces: Backfaces,
}

impl Default for RayCastBackendSettings {
    fn default() -> Self {
        Self {
            require_markers: false,
            raycast_visibility: RayCastVisibility::VisibleAndInView,
            backfaces: Backfaces::default(),
        }
    }
}

/// Optional. Marks cameras and target entities that should be used in the ray cast picking backend.
/// Only needed if [`RayCastBackendSettings::require_markers`] is set to true.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RayCastPickable;

/// Adds the ray casting picking backend to your app.
#[derive(Clone, Default)]
pub struct RayCastBackend;
impl Plugin for RayCastBackend {
    fn build(&self, app: &mut App) {
        app.init_resource::<RayCastBackendSettings>()
            .add_systems(PreUpdate, update_hits.in_set(PickSet::Backend))
            .register_type::<RayCastPickable>()
            .register_type::<RayCastBackendSettings>();
    }
}

/// Raycasts into the scene using [`RayCastBackendSettings`] and [`PointerLocation`]s, then outputs
/// [`PointerHits`].
pub fn update_hits(
    backend_settings: Res<RayCastBackendSettings>,
    ray_map: Res<RayMap>,
    picking_cameras: Query<(&Camera, Option<&RayCastPickable>, Option<&RenderLayers>)>,
    pickables: Query<&Pickable>,
    marked_targets: Query<&RayCastPickable>,
    layers: Query<&RenderLayers>,
    mut ray_cast: MeshRayCast,
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

        let settings = RayCastSettings {
            visibility: backend_settings.raycast_visibility,
            backfaces: backend_settings.backfaces,
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
        let picks = ray_cast
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
