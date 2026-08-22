//! This module contains a basic backend that implements picking for window
//! entities.
//!
//! Pointers can exist on windows, images, and gpu texture views. With
//! [`update_window_hits`] enabled, when a pointer hovers over a window that
//! window will be inserted as a pointer hit, listed behind all other pointer
//! hits. This means that when the pointer isn't hovering any other entities,
//! the picking events will be routed to the window.
//!
//! ## Implementation Notes
//!
//! - This backend does not provide `normal` in `HitData`.

use bevy_camera::NormalizedRenderTarget;
use bevy_ecs::prelude::*;

use crate::{
    backend::{ray::RayMap, HitData, PointerHits},
    pointer::{Location, PointerId, PointerLocation},
};

/// Generates pointer hit events for window entities.
///
/// A pointer is treated as hitting a window when it is located on that window. The order
/// of the hit event is negative infinity, meaning it should appear behind all other entities.
///
/// The depth of the hit will be listed as zero.
pub fn update_window_hits(
    pointers: Query<(&PointerId, &PointerLocation)>,
    mut pointer_hits_writer: MessageWriter<PointerHits>,
    ray_map: Res<RayMap>,
) {
    for (&ray_id, _) in ray_map.iter() {
        if let Some((position, window_entity)) = pointers.iter().find_map(|(id, loc)| {
            if *id == ray_id.pointer
                && let Some(Location {
                    target: NormalizedRenderTarget::Window(window_ref),
                    position,
                    ..
                }) = loc.location
            {
                return Some((position, window_ref.entity()));
            }
            None
        }) {
            let hit_data = HitData::new(ray_id.camera, 0.0, Some(position.extend(0.0)), None);
            pointer_hits_writer.write(PointerHits::new(
                ray_id.pointer,
                vec![(window_entity, hit_data)],
                f32::NEG_INFINITY,
            ));
        }
    }
}
