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
use bevy_camera::RenderTarget;
use bevy_ecs::prelude::*;
use bevy_input::{
    mouse::MouseWheel,
    prelude::*,
    touch::{TouchInput, TouchPhase},
    ButtonState,
};
use bevy_math::Vec2;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::prelude::*;
use bevy_window::{PrimaryWindow, WindowEvent, WindowRef};
use tracing::debug;

use crate::pointer::{
    Location, PointerAction, PointerButton, PointerId, PointerInput, PointerLocation,
};

use crate::PickingSystems;

/// The picking input prelude.
///
/// This includes the most common types in this module, re-exported for your convenience.
pub mod prelude {
    pub use crate::input::PointerInputPlugin;
}

#[derive(Copy, Clone, Resource, Debug, Reflect)]
#[reflect(Resource, Default, Clone)]
/// Settings for enabling and disabling updating mouse and touch inputs for picking
///
/// ## Custom initialization
/// ```
/// # use bevy_app::App;
/// # use bevy_picking::input::{PointerInputSettings,PointerInputPlugin};
/// App::new()
///     .insert_resource(PointerInputSettings {
///         is_touch_enabled: false,
///         is_mouse_enabled: true,
///     })
///     // or DefaultPlugins
///     .add_plugins(PointerInputPlugin);
/// ```
pub struct PointerInputSettings {
    /// Should touch inputs be updated?
    pub is_touch_enabled: bool,
    /// Should mouse inputs be updated?
    pub is_mouse_enabled: bool,
}

impl PointerInputSettings {
    fn is_mouse_enabled(state: Res<Self>) -> bool {
        state.is_mouse_enabled
    }

    fn is_touch_enabled(state: Res<Self>) -> bool {
        state.is_touch_enabled
    }
}

impl Default for PointerInputSettings {
    fn default() -> Self {
        Self {
            is_touch_enabled: true,
            is_mouse_enabled: true,
        }
    }
}

/// Adds mouse and touch inputs for picking pointers to your app. This is a default input plugin,
/// that you can replace with your own plugin as needed.
///
/// Toggling mouse input or touch input can be done at runtime by modifying
/// [`PointerInputSettings`] resource.
///
/// [`PointerInputSettings`] can be initialized with custom values, but will be
/// initialized with default values if it is not present at the moment this is
/// added to the app.
pub struct PointerInputPlugin;

impl Plugin for PointerInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PointerInputSettings>()
            .add_systems(Startup, spawn_mouse_pointer)
            .add_systems(
                First,
                (
                    mouse_pick_events.run_if(PointerInputSettings::is_mouse_enabled),
                    touch_pick_events.run_if(PointerInputSettings::is_touch_enabled),
                )
                    .chain()
                    .in_set(PickingSystems::Input),
            )
            .add_systems(
                Last,
                deactivate_touch_pointers.run_if(PointerInputSettings::is_touch_enabled),
            );
    }
}

/// Spawns the default mouse pointer.
pub fn spawn_mouse_pointer(mut commands: Commands) {
    commands.spawn(PointerId::Mouse);
}

/// Sends mouse pointer events to be processed by the core plugin
pub fn mouse_pick_events(
    // Input
    mut window_events: MessageReader<WindowEvent>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    // Locals
    mut cursor_last: Local<Vec2>,
    // Output
    mut pointer_inputs: MessageWriter<PointerInput>,
) {
    for window_event in window_events.read() {
        match window_event {
            // Handle cursor movement events
            WindowEvent::CursorMoved(event) => {
                let location = Location {
                    target: match RenderTarget::Window(WindowRef::Entity(event.window))
                        .normalize(primary_window.single().ok())
                    {
                        Some(target) => target,
                        None => continue,
                    },
                    position: event.position,
                };
                pointer_inputs.write(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::Move {
                        delta: event.position - *cursor_last,
                    },
                ));
                *cursor_last = event.position;
            }
            // Handle mouse button press events
            WindowEvent::MouseButtonInput(input) => {
                let location = Location {
                    target: match RenderTarget::Window(WindowRef::Entity(input.window))
                        .normalize(primary_window.single().ok())
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
                let action = match input.state {
                    ButtonState::Pressed => PointerAction::Press(button),
                    ButtonState::Released => PointerAction::Release(button),
                };
                pointer_inputs.write(PointerInput::new(PointerId::Mouse, location, action));
            }
            WindowEvent::MouseWheel(event) => {
                let MouseWheel { unit, x, y, window } = *event;

                let location = Location {
                    target: match RenderTarget::Window(WindowRef::Entity(window))
                        .normalize(primary_window.single().ok())
                    {
                        Some(target) => target,
                        None => continue,
                    },
                    position: *cursor_last,
                };

                let action = PointerAction::Scroll { x, y, unit };

                pointer_inputs.write(PointerInput::new(PointerId::Mouse, location, action));
            }
            _ => {}
        }
    }
}

/// Sends touch pointer events to be consumed by the core plugin
pub fn touch_pick_events(
    // Input
    mut window_events: MessageReader<WindowEvent>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    // Locals
    mut touch_cache: Local<HashMap<u64, TouchInput>>,
    // Output
    mut commands: Commands,
    mut pointer_inputs: MessageWriter<PointerInput>,
) {
    for window_event in window_events.read() {
        if let WindowEvent::TouchInput(touch) = window_event {
            let pointer = PointerId::Touch(touch.id);
            let location = Location {
                target: match RenderTarget::Window(WindowRef::Entity(touch.window))
                    .normalize(primary_window.single().ok())
                {
                    Some(target) => target,
                    None => continue,
                },
                position: touch.position,
            };
            match touch.phase {
                TouchPhase::Started => {
                    debug!("Spawning pointer {:?}", pointer);
                    commands.spawn((pointer, PointerLocation::new(location.clone())));

                    pointer_inputs.write(PointerInput::new(
                        pointer,
                        location,
                        PointerAction::Press(PointerButton::Primary),
                    ));

                    touch_cache.insert(touch.id, *touch);
                }
                TouchPhase::Moved => {
                    // Send a move event only if it isn't the same as the last one
                    if let Some(last_touch) = touch_cache.get(&touch.id) {
                        if last_touch == touch {
                            continue;
                        }
                        pointer_inputs.write(PointerInput::new(
                            pointer,
                            location,
                            PointerAction::Move {
                                delta: touch.position - last_touch.position,
                            },
                        ));
                    }
                    touch_cache.insert(touch.id, *touch);
                }
                TouchPhase::Ended => {
                    pointer_inputs.write(PointerInput::new(
                        pointer,
                        location,
                        PointerAction::Release(PointerButton::Primary),
                    ));
                    touch_cache.remove(&touch.id);
                }
                TouchPhase::Canceled => {
                    pointer_inputs.write(PointerInput::new(
                        pointer,
                        location,
                        PointerAction::Cancel,
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
    mut touches: MessageReader<TouchInput>,
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
        commands.entity(entity).despawn();
    }
}
