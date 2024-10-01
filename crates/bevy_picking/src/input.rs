//! This module provides unsurprising default inputs to `bevy_picking` through [`PointerInput`].
//! The included systems are responsible for sending  mouse and touch inputs to their
//! respective `Pointer`s.
//!
//! Because this has it's own plugin, it's easy to omit it, and provide your own inputs as
//! needed. Because `Pointer`s aren't coupled to the underlying input hardware, you can easily mock
//! inputs, and allow users full accessibility to map whatever inputs they need to pointer input.
//!
//! If, for example, you wanted to add support for VR input, all you need to do is spawn a pointer
//! entity with a custom [`PointerId`], and write a system
//! that updates its position. If you want this to work properly with the existing interaction events,
//! you need to be sure that you also write a [`PointerInput`] event stream.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_hierarchy::DespawnRecursiveExt;
use bevy_input::{
    prelude::*,
    touch::{TouchInput, TouchPhase},
    ButtonState,
};
use bevy_math::Vec2;
use bevy_reflect::prelude::*;
use bevy_render::camera::RenderTarget;
use bevy_utils::{tracing::debug, HashMap, HashSet};
use bevy_window::{PrimaryWindow, WindowEvent, WindowRef};

use crate::{
    pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput, PressDirection},
    PointerBundle,
};

use crate::PickSet;

/// The picking input prelude.
///
/// This includes the most common types in this module, re-exported for your convenience.
pub mod prelude {
    pub use crate::input::PointerInputPlugin;
}

/// Adds mouse and touch inputs for picking pointers to your app. This is a default input plugin,
/// that you can replace with your own plugin as needed.
///
/// [`crate::PickingPlugin::is_input_enabled`] can be used to toggle whether
/// the core picking plugin processes the inputs sent by this, or other input plugins, in one place.
///
/// This plugin contains several settings, and is added to the world as a resource after initialization.
/// You can configure pointer input settings at runtime by accessing the resource.
#[derive(Copy, Clone, Resource, Debug, Reflect)]
#[reflect(Resource, Default)]
pub struct PointerInputPlugin {
    /// Should touch inputs be updated?
    pub is_touch_enabled: bool,
    /// Should mouse inputs be updated?
    pub is_mouse_enabled: bool,
}

impl PointerInputPlugin {
    fn is_mouse_enabled(state: Res<Self>) -> bool {
        state.is_mouse_enabled
    }

    fn is_touch_enabled(state: Res<Self>) -> bool {
        state.is_touch_enabled
    }
}

impl Default for PointerInputPlugin {
    fn default() -> Self {
        Self {
            is_touch_enabled: true,
            is_mouse_enabled: true,
        }
    }
}

impl Plugin for PointerInputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(*self)
            .add_systems(Startup, spawn_mouse_pointer)
            .add_systems(
                First,
                (
                    mouse_pick_events.run_if(PointerInputPlugin::is_mouse_enabled),
                    touch_pick_events.run_if(PointerInputPlugin::is_touch_enabled),
                )
                    .chain()
                    .in_set(PickSet::Input),
            )
            .add_systems(
                Last,
                deactivate_touch_pointers.run_if(PointerInputPlugin::is_touch_enabled),
            )
            .register_type::<Self>()
            .register_type::<PointerInputPlugin>();
    }
}

/// Spawns the default mouse pointer.
pub fn spawn_mouse_pointer(mut commands: Commands) {
    commands.spawn((PointerBundle::new(PointerId::Mouse),));
}

/// Sends mouse pointer events to be processed by the core plugin
pub fn mouse_pick_events(
    // Input
    mut window_events: EventReader<WindowEvent>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    // Locals
    mut cursor_last: Local<Vec2>,
    // Output
    mut pointer_events: EventWriter<PointerInput>,
) {
    for window_event in window_events.read() {
        match window_event {
            // Handle cursor movement events
            WindowEvent::CursorMoved(event) => {
                let location = Location {
                    target: match RenderTarget::Window(WindowRef::Entity(event.window))
                        .normalize(primary_window.get_single().ok())
                    {
                        Some(target) => target,
                        None => continue,
                    },
                    position: event.position,
                };
                pointer_events.send(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::Moved {
                        delta: event.position - *cursor_last,
                    },
                ));
                *cursor_last = event.position;
            }
            // Handle mouse button press events
            WindowEvent::MouseButtonInput(input) => {
                let location = Location {
                    target: match RenderTarget::Window(WindowRef::Entity(input.window))
                        .normalize(primary_window.get_single().ok())
                    {
                        Some(target) => target,
                        None => continue,
                    },
                    position: *cursor_last,
                };
                let button = match input.button {
                    MouseButton::Left => PointerButton::Primary,
                    MouseButton::Right => PointerButton::Secondary,
                    MouseButton::Middle => PointerButton::Middle,
                    MouseButton::Other(_) | MouseButton::Back | MouseButton::Forward => continue,
                };
                let direction = match input.state {
                    ButtonState::Pressed => PressDirection::Down,
                    ButtonState::Released => PressDirection::Up,
                };
                pointer_events.send(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::Pressed { direction, button },
                ));
            }
            _ => {}
        }
    }
}

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
