//! Types and systems for pointer inputs, such as position and buttons.

use bevy_ecs::prelude::*;
use bevy_math::{Rect, Vec2};
use bevy_reflect::prelude::*;
use bevy_render::camera::{Camera, NormalizedRenderTarget};
use bevy_utils::HashMap;
use bevy_window::PrimaryWindow;

use uuid::Uuid;

use std::fmt::Debug;

use crate::backend::HitData;

/// Identifies a unique pointer entity. `Mouse` and `Touch` pointers are automatically spawned.
///
/// This component is needed because pointers can be spawned and despawned, but they need to have a
/// stable ID that persists regardless of the Entity they are associated with.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash, Component, Reflect)]
#[reflect(Component, Default)]
pub enum PointerId {
    /// The mouse pointer.
    #[default]
    Mouse,
    /// A touch input, usually numbered by window touch events from `winit`.
    Touch(u64),
    /// A custom, uniquely identified pointer. Useful for mocking inputs or implementing a software
    /// controlled cursor.
    #[reflect(ignore)]
    Custom(Uuid),
}

impl PointerId {
    /// Returns true if the pointer is a touch input.
    pub fn is_touch(&self) -> bool {
        matches!(self, PointerId::Touch(_))
    }
    /// Returns true if the pointer is the mouse.
    pub fn is_mouse(&self) -> bool {
        matches!(self, PointerId::Mouse)
    }
    /// Returns true if the pointer is a custom input.
    pub fn is_custom(&self) -> bool {
        matches!(self, PointerId::Custom(_))
    }
    /// Returns the touch id if the pointer is a touch input.
    pub fn get_touch_id(&self) -> Option<u64> {
        if let PointerId::Touch(id) = self {
            Some(*id)
        } else {
            None
        }
    }
}

/// Holds a list of entities this pointer is currently interacting with, sorted from nearest to
/// farthest.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct PointerInteraction {
    pub(crate) sorted_entities: Vec<(Entity, HitData)>,
}

/// A resource that maps each [`PointerId`] to their [`Entity`] for easy lookups.
#[derive(Debug, Clone, Default, Resource)]
pub struct PointerMap {
    inner: HashMap<PointerId, Entity>,
}

impl PointerMap {
    /// Get the [`Entity`] of the supplied [`PointerId`].
    pub fn get_entity(&self, pointer_id: PointerId) -> Option<Entity> {
        self.inner.get(&pointer_id).copied()
    }
}

/// Update the [`PointerMap`] resource with the current frame's data.
pub fn update_pointer_map(pointers: Query<(Entity, &PointerId)>, mut map: ResMut<PointerMap>) {
    map.inner.clear();
    for (entity, id) in &pointers {
        map.inner.insert(*id, entity);
    }
}

/// Tracks the state of the pointer's buttons in response to [`InputPress`]s.
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct PointerPress {
    primary: bool,
    secondary: bool,
    middle: bool,
}

impl PointerPress {
    /// Returns true if the primary pointer button is pressed.
    #[inline]
    pub fn is_primary_pressed(&self) -> bool {
        self.primary
    }

    /// Returns true if the secondary pointer button is pressed.
    #[inline]
    pub fn is_secondary_pressed(&self) -> bool {
        self.secondary
    }

    /// Returns true if the middle (tertiary) pointer button is pressed.
    #[inline]
    pub fn is_middle_pressed(&self) -> bool {
        self.middle
    }

    /// Returns true if any pointer button is pressed.
    #[inline]
    pub fn is_any_pressed(&self) -> bool {
        self.primary || self.middle || self.secondary
    }
}

/// Pointer input event for button presses. Fires when a pointer button changes state.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct InputPress {
    /// The [`PointerId`] of the pointer that pressed a button.
    pub pointer_id: PointerId,
    /// Direction of the button press.
    pub direction: PressDirection,
    /// Identifies the pointer button changing in this event.
    pub button: PointerButton,
}

impl InputPress {
    /// Create a new pointer button down event.
    pub fn new_down(id: PointerId, button: PointerButton) -> InputPress {
        Self {
            pointer_id: id,
            direction: PressDirection::Down,
            button,
        }
    }

    /// Create a new pointer button up event.
    pub fn new_up(id: PointerId, button: PointerButton) -> InputPress {
        Self {
            pointer_id: id,
            direction: PressDirection::Up,
            button,
        }
    }

    /// Returns true if the `button` of this pointer was just pressed.
    #[inline]
    pub fn is_just_down(&self, button: PointerButton) -> bool {
        self.button == button && self.direction == PressDirection::Down
    }

    /// Returns true if the `button` of this pointer was just released.
    #[inline]
    pub fn is_just_up(&self, button: PointerButton) -> bool {
        self.button == button && self.direction == PressDirection::Up
    }

    /// Receives [`InputPress`] events and updates corresponding [`PointerPress`] components.
    pub fn receive(
        mut events: EventReader<InputPress>,
        mut pointers: Query<(&PointerId, &mut PointerPress)>,
    ) {
        for input_press_event in events.read() {
            pointers.iter_mut().for_each(|(pointer_id, mut pointer)| {
                if *pointer_id == input_press_event.pointer_id {
                    let is_down = input_press_event.direction == PressDirection::Down;
                    match input_press_event.button {
                        PointerButton::Primary => pointer.primary = is_down,
                        PointerButton::Secondary => pointer.secondary = is_down,
                        PointerButton::Middle => pointer.middle = is_down,
                    }
                }
            });
        }
    }
}

/// The stage of the pointer button press event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum PressDirection {
    /// The pointer button was just pressed
    Down,
    /// The pointer button was just released
    Up,
}

/// The button that was just pressed or released
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum PointerButton {
    /// The primary pointer button
    Primary,
    /// The secondary pointer button
    Secondary,
    /// The tertiary pointer button
    Middle,
}

impl PointerButton {
    /// Iterator over all buttons that a pointer can have.
    pub fn iter() -> impl Iterator<Item = PointerButton> {
        [Self::Primary, Self::Secondary, Self::Middle].into_iter()
    }
}

/// Component that tracks a pointer's current [`Location`].
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq)]
#[reflect(Component, Default)]
pub struct PointerLocation {
    /// The [`Location`] of the pointer. Note that a location is both the target, and the position
    /// on the target.
    #[reflect(ignore)]
    pub location: Option<Location>,
}

impl PointerLocation {
    /// Returns `Some(&`[`Location`]`)` if the pointer is active, or `None` if the pointer is
    /// inactive.
    pub fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }
}

/// Pointer input event for pointer moves. Fires when a pointer changes location.
#[derive(Event, Debug, Clone, Reflect)]
pub struct InputMove {
    /// The [`PointerId`] of the pointer that is moving.
    pub pointer_id: PointerId,
    /// The [`Location`] of the pointer.
    pub location: Location,
    /// The distance moved (change in `position`) since the last event.
    pub delta: Vec2,
}

impl InputMove {
    /// Create a new [`InputMove`] event.
    pub fn new(id: PointerId, location: Location, delta: Vec2) -> InputMove {
        Self {
            pointer_id: id,
            location,
            delta,
        }
    }

    /// Receives [`InputMove`] events and updates corresponding [`PointerLocation`] components.
    pub fn receive(
        mut events: EventReader<InputMove>,
        mut pointers: Query<(&PointerId, &mut PointerLocation)>,
    ) {
        for event_pointer in events.read() {
            pointers.iter_mut().for_each(|(id, mut pointer)| {
                if *id == event_pointer.pointer_id {
                    pointer.location = Some(event_pointer.location.to_owned());
                }
            });
        }
    }
}

/// The location of a pointer, including the current [`NormalizedRenderTarget`], and the x/y
/// position of the pointer on this render target.
///
/// Note that:
/// - a pointer can move freely between render targets
/// - a pointer is not associated with a [`Camera`] because multiple cameras can target the same
///   render target. It is up to picking backends to associate a Pointer's `Location` with a
///   specific `Camera`, if any.
#[derive(Debug, Clone, Component, Reflect, PartialEq)]
pub struct Location {
    /// The [`NormalizedRenderTarget`] associated with the pointer, usually a window.
    pub target: NormalizedRenderTarget,
    /// The position of the pointer in the `target`.
    pub position: Vec2,
}

impl Location {
    /// Returns `true` if this pointer's [`Location`] is within the [`Camera`]'s viewport.
    ///
    /// Note this returns `false` if the location and camera have different render targets.
    #[inline]
    pub fn is_in_viewport(
        &self,
        camera: &Camera,
        primary_window: &Query<Entity, With<PrimaryWindow>>,
    ) -> bool {
        if camera
            .target
            .normalize(Some(match primary_window.get_single() {
                Ok(w) => w,
                Err(_) => return false,
            }))
            .as_ref()
            != Some(&self.target)
        {
            return false;
        }

        let position = Vec2::new(self.position.x, self.position.y);

        camera
            .logical_viewport_rect()
            .map(|Rect { min, max }| {
                (position - min).min_element() >= 0.0 && (position - max).max_element() <= 0.0
            })
            .unwrap_or(false)
    }
}
