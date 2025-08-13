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
//! - This backend does not provide `position` or `normal` in `HitData`.

use core::f32;

use bevy_camera::NormalizedRenderTarget;
use bevy_ecs::prelude::*;

use crate::{
    backend::{HitData, PointerHits},
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
    mut output_events: EventWriter<PointerHits>,
) {
    for (pointer_id, pointer_location) in pointers.iter() {
        if let Some(Location {
            target: NormalizedRenderTarget::Window(window_ref),
            ..
        }) = pointer_location.location
        {
            let entity = window_ref.entity();
            let hit_data = HitData::new(entity, 0.0, None, None);
            output_events.write(PointerHits::new(
                *pointer_id,
                vec![(entity, hit_data)],
                f32::NEG_INFINITY,
            ));
        }
    }
}
