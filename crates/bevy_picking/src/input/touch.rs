//! Provides sensible defaults for touch picking inputs.

use bevy_ecs::prelude::*;
use bevy_hierarchy::DespawnRecursiveExt;
use bevy_input::touch::{TouchInput, TouchPhase};
use bevy_math::Vec2;
use bevy_render::camera::RenderTarget;
use bevy_utils::{tracing::debug, HashMap, HashSet};
use bevy_window::{PrimaryWindow, WindowRef};

use crate::{
    events::PointerCancel,
    pointer::{InputMove, InputPress, Location, PointerButton, PointerId},
    PointerBundle,
};

/// Sends touch pointer events to be consumed by the core plugin
///
/// IMPORTANT: the commands must be flushed after this system is run because we need spawning to
/// happen immediately to prevent issues with missed events needed for drag and drop.
pub fn touch_pick_events(
    // Input
    mut touches: EventReader<TouchInput>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    // Local
    mut location_cache: Local<HashMap<u64, TouchInput>>,
    // Output
    mut commands: Commands,
    mut input_moves: EventWriter<InputMove>,
    mut input_presses: EventWriter<InputPress>,
    mut cancel_events: EventWriter<PointerCancel>,
) {
    for touch in touches.read() {
        let pointer = PointerId::Touch(touch.id);
        let location = Location {
            target: match RenderTarget::Window(WindowRef::Entity(touch.window))
                .normalize(primary_window.get_single().ok())
            {
                Some(target) => target,
                None => continue,
            },
            position: touch.position,
        };
        match touch.phase {
            TouchPhase::Started => {
                debug!("Spawning pointer {:?}", pointer);
                commands.spawn((
                    PointerBundle::new(pointer).with_location(location.clone()),
                    #[cfg(feature = "selection")]
                    bevy_picking_selection::PointerMultiselect::default(),
                ));

                input_moves.send(InputMove::new(pointer, location, Vec2::ZERO));
                input_presses.send(InputPress::new_down(pointer, PointerButton::Primary));
                location_cache.insert(touch.id, *touch);
            }
            TouchPhase::Moved => {
                // Send a move event only if it isn't the same as the last one
                if let Some(last_touch) = location_cache.get(&touch.id) {
                    if last_touch == touch {
                        continue;
                    }
                    input_moves.send(InputMove::new(
                        pointer,
                        location,
                        touch.position - last_touch.position,
                    ));
                }
                location_cache.insert(touch.id, *touch);
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                input_presses.send(InputPress::new_up(pointer, PointerButton::Primary));
                location_cache.remove(&touch.id);
                cancel_events.send(PointerCancel {
                    pointer_id: pointer,
                });
            }
        }
    }
}

/// Deactivates unused touch pointers.
///
/// Because each new touch gets assigned a new ID, we need to remove the pointers associated with
/// touches that are no longer active.
pub fn deactivate_touch_pointers(
    mut commands: Commands,
    mut despawn_list: Local<HashSet<(Entity, PointerId)>>,
    pointers: Query<(Entity, &PointerId)>,
    mut touches: EventReader<TouchInput>,
) {
    for touch in touches.read() {
        match touch.phase {
            TouchPhase::Ended | TouchPhase::Canceled => {
                for (entity, pointer) in &pointers {
                    if pointer.get_touch_id() == Some(touch.id) {
                        despawn_list.insert((entity, *pointer));
                    }
                }
            }
            _ => {}
        }
    }
    // A hash set is used to prevent despawning the same entity twice.
    for (entity, pointer) in despawn_list.drain() {
        debug!("Despawning pointer {:?}", pointer);
        commands.entity(entity).despawn_recursive();
    }
}
