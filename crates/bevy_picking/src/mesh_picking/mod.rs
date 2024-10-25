//! A [mesh ray casting](ray_cast) backend for [`bevy_picking`](crate).
//!
//! By default, all meshes are pickable. Picking can be disabled for individual entities
//! by adding [`PickingBehavior::IGNORE`].
//!
//! To make mesh picking entirely opt-in, set [`MeshPickingSettings::require_markers`]
//! to `true` and add a [`RayCastPickable`] component to the desired camera and target entities.
//!
//! To manually perform mesh ray casts independent of picking, use the [`MeshRayCast`] system parameter.

pub mod ray_cast;

use crate::{
    backend::{ray::RayMap, HitData, PointerHits},
    prelude::*,
    PickSet,
};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_render::{prelude::*, view::RenderLayers};
use ray_cast::{MeshRayCast, RayCastSettings, RayCastVisibility, SimplifiedMesh};

/// Runtime settings for the [`MeshPickingPlugin`].
#[derive(Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct MeshPickingSettings {
    /// When set to `true` ray casting will only happen between cameras and entities marked with
    /// [`RayCastPickable`]. `false` by default.
    ///
    /// This setting is provided to give you fine-grained control over which cameras and entities
    /// should be used by the mesh picking backend at runtime.
    pub require_markers: bool,

    /// Determines how mesh picking should consider [`Visibility`]. When set to [`RayCastVisibility::Any`],
    /// ray casts can be performed against both visible and hidden entities.
    ///
    /// Defaults to [`RayCastVisibility::VisibleInView`], only performing picking against visible entities
    /// that are in the view of a camera.
    pub ray_cast_visibility: RayCastVisibility,
}

impl Default for MeshPickingSettings {
    fn default() -> Self {
        Self {
            require_markers: false,
            ray_cast_visibility: RayCastVisibility::VisibleInView,
        }
    }
}

/// An optional component that marks cameras and target entities that should be used in the [`MeshPickingPlugin`].
/// Only needed if [`MeshPickingSettings::require_markers`] is set to `true`, and ignored otherwise.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RayCastPickable;

/// Adds the mesh picking backend to your app.
#[derive(Clone, Default)]
pub struct MeshPickingPlugin;

impl Plugin for MeshPickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MeshPickingSettings>()
            .register_type::<(RayCastPickable, MeshPickingSettings, SimplifiedMesh)>()
            .add_systems(PreUpdate, update_hits.in_set(PickSet::Backend));
    }
}

/// Casts rays into the scene using [`MeshPickingSettings`] and sends [`PointerHits`] events.
#[allow(clippy::too_many_arguments)]
pub fn update_hits(
    backend_settings: Res<MeshPickingSettings>,
    ray_map: Res<RayMap>,
    picking_cameras: Query<(&Camera, Option<&RayCastPickable>, Option<&RenderLayers>)>,
    pickables: Query<&PickingBehavior>,
    marked_targets: Query<&RayCastPickable>,
    layers: Query<&RenderLayers>,
    mut ray_cast: MeshRayCast,
    mut output: EventWriter<PointerHits>,
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
            visibility: backend_settings.ray_cast_visibility,
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
                    hit.distance,
                    Some(hit.point),
                    Some(hit.normal),
                );
                (*entity, hit_data)
            })
            .collect::<Vec<_>>();
        let order = camera.order as f32;
        if !picks.is_empty() {
            output.send(PointerHits::new(ray_id.pointer, picks, order));
        }
    }
}
