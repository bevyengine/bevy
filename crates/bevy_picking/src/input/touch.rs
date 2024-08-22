//! Provides sensible defaults for touch picking inputs.

use bevy_ecs::prelude::*;
use bevy_hierarchy::DespawnRecursiveExt;
use bevy_input::touch::{TouchInput, TouchPhase};
use bevy_render::camera::RenderTarget;
use bevy_utils::{tracing::debug, HashMap, HashSet};
use bevy_window::{PrimaryWindow, WindowEvent, WindowRef};

use crate::{
    pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput, PressDirection},
    PointerBundle,
};

/// Sends touch pointer events to be consumed by the core plugin
pub fn touch_pick_events(
    // Input
    mut window_events: EventReader<WindowEvent>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    // Locals
    mut touch_cache: Local<HashMap<u64, TouchInput>>,
    // Output
    mut commands: Commands,
    mut pointer_events: EventWriter<PointerInput>,
) {
    for window_event in window_events.read() {
        if let WindowEvent::TouchInput(touch) = window_event {
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
                    commands.spawn(PointerBundle::new(pointer).with_location(location.clone()));

                    pointer_events.send(PointerInput::new(
                        pointer,
                        location,
                        PointerAction::Pressed {
                            direction: PressDirection::Down,
                            button: PointerButton::Primary,
                        },
                    ));

                    touch_cache.insert(touch.id, *touch);
                }
                TouchPhase::Moved => {
                    // Send a move event only if it isn't the same as the last one
                    if let Some(last_touch) = touch_cache.get(&touch.id) {
                        if last_touch == touch {
                            continue;
                        }
                        pointer_events.send(PointerInput::new(
                            pointer,
                            location,
                            PointerAction::Moved {
                                delta: touch.position - last_touch.position,
                            },
                        ));
                    }
                    touch_cache.insert(touch.id, *touch);
                }
                TouchPhase::Ended => {
                    pointer_events.send(PointerInput::new(
                        pointer,
                        location,
                        PointerAction::Pressed {
                            direction: PressDirection::Up,
                            button: PointerButton::Primary,
                        },
                    ));
                    touch_cache.remove(&touch.id);
                }
                TouchPhase::Canceled => {
                    pointer_events.send(PointerInput::new(
                        pointer,
                        location,
                        PointerAction::Canceled,
                    ));
                    touch_cache.remove(&touch.id);
                }
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
        if let TouchPhase::Ended | TouchPhase::Canceled = touch.phase {
            for (entity, pointer) in &pointers {
                if pointer.get_touch_id() == Some(touch.id) {
                    despawn_list.insert((entity, *pointer));
                }
            }
        }
    }
    // A hash set is used to prevent despawning the same entity twice.
    for (entity, pointer) in despawn_list.drain() {
        debug!("Despawning pointer {:?}", pointer);
        commands.entity(entity).despawn_recursive();
    }
}
