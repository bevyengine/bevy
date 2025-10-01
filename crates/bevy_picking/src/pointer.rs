//! Types and systems for pointer inputs, such as position and buttons.
//!
//! The picking system is built around the concept of a 'Pointer', which is an
//! abstract representation of a user input with a specific screen location. The cursor
//! and touch input is provided under [`input`](`crate::input`), but you can also implement
//! your own custom pointers by supplying a unique ID.
//!
//! The purpose of this module is primarily to provide a common interface that can be
//! driven by lower-level input devices and consumed by higher-level interaction systems.

use bevy_camera::Camera;
use bevy_camera::NormalizedRenderTarget;
use bevy_ecs::prelude::*;
use bevy_input::mouse::MouseScrollUnit;
use bevy_math::Vec2;
use bevy_platform::collections::HashMap;
use bevy_reflect::prelude::*;
use bevy_window::PrimaryWindow;

use uuid::Uuid;

use core::{fmt::Debug, ops::Deref};

use crate::backend::HitData;

/// Identifies a unique pointer entity. `Mouse` and `Touch` pointers are automatically spawned.
///
/// This component is needed because pointers can be spawned and despawned, but they need to have a
/// stable ID that persists regardless of the Entity they are associated with.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash, Component, Reflect)]
#[require(PointerLocation, PointerPress, PointerInteraction)]
#[reflect(Component, Default, Debug, Hash, PartialEq, Clone)]
pub enum PointerId {
    /// The mouse pointer.
    #[default]
    Mouse,
    /// A touch input, usually numbered by window touch events from `winit`.
    Touch(u64),
    /// A custom, uniquely identified pointer. Useful for mocking inputs or implementing a software
    /// controlled cursor.
    #[reflect(ignore, clone)]
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
#[reflect(Component, Default, Debug, Clone)]
pub struct PointerInteraction {
    pub(crate) sorted_entities: Vec<(Entity, HitData)>,
}

impl PointerInteraction {
    /// Returns the nearest hit entity and data about that intersection.
    pub fn get_nearest_hit(&self) -> Option<&(Entity, HitData)> {
        self.sorted_entities.first()
    }
}

impl Deref for PointerInteraction {
    type Target = Vec<(Entity, HitData)>;

    fn deref(&self) -> &Self::Target {
        &self.sorted_entities
    }
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

/// Tracks the state of the pointer's buttons in response to [`PointerInput`] events.
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
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

/// The stage of the pointer button press event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Clone, PartialEq)]
pub enum PressDirection {
    /// The pointer button was just pressed
    Pressed,
    /// The pointer button was just released
    Released,
}

/// The button that was just pressed or released
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Clone, PartialEq)]
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
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct PointerLocation {
    /// The [`Location`] of the pointer. Note that a location is both the target, and the position
    /// on the target.
    #[reflect(ignore, clone)]
    pub location: Option<Location>,
}

impl PointerLocation {
    ///Returns a [`PointerLocation`] associated with the given location
    pub fn new(location: Location) -> Self {
        Self {
            location: Some(location),
        }
    }

    /// Returns `Some(&`[`Location`]`)` if the pointer is active, or `None` if the pointer is
    /// inactive.
    pub fn location(&self) -> Option<&Location> {
        self.location.as_ref()
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
#[derive(Debug, Clone, Reflect, PartialEq)]
#[reflect(Debug, PartialEq, Clone)]
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
            .normalize(Some(match primary_window.single() {
                Ok(w) => w,
                Err(_) => return false,
            }))
            .as_ref()
            != Some(&self.target)
        {
            return false;
        }

        camera
            .logical_viewport_rect()
            .is_some_and(|rect| rect.contains(self.position))
    }
}

/// Event sent to drive a pointer.
#[derive(Debug, Clone, Copy, Reflect)]
#[reflect(Clone)]
pub enum PointerAction {
    /// Causes the pointer to press a button.
    Press(PointerButton),
    /// Causes the pointer to release a button.
    Release(PointerButton),
    /// Move the pointer.
    Move {
        /// How much the pointer moved from the previous position.
        delta: Vec2,
    },
    /// Scroll the pointer
    Scroll {
        /// The mouse scroll unit.
        unit: MouseScrollUnit,
        /// The horizontal scroll value.
        x: f32,
        /// The vertical scroll value.
        y: f32,
    },
    /// Cancel the pointer. Often used for touch events.
    Cancel,
}

/// An input event effecting a pointer.
#[derive(Message, Debug, Clone, Reflect)]
#[reflect(Clone)]
pub struct PointerInput {
    /// The id of the pointer.
    pub pointer_id: PointerId,
    /// The location of the pointer. For [`PointerAction::Move`], this is the location after the movement.
    pub location: Location,
    /// The action that the event describes.
    pub action: PointerAction,
}

impl PointerInput {
    /// Creates a new pointer input event.
    ///
    /// Note that `location` refers to the position of the pointer *after* the event occurred.
    pub fn new(pointer_id: PointerId, location: Location, action: PointerAction) -> PointerInput {
        PointerInput {
            pointer_id,
            location,
            action,
        }
    }

    /// Returns true if the `target_button` of this pointer was just pressed.
    #[inline]
    pub fn button_just_pressed(&self, target_button: PointerButton) -> bool {
        if let PointerAction::Press(button) = self.action {
            button == target_button
        } else {
            false
        }
    }

    /// Returns true if the `target_button` of this pointer was just released.
    #[inline]
    pub fn button_just_released(&self, target_button: PointerButton) -> bool {
        if let PointerAction::Release(button) = self.action {
            button == target_button
        } else {
            false
        }
    }

    /// Updates pointer entities according to the input events.
    pub fn receive(
        mut events: MessageReader<PointerInput>,
        mut pointers: Query<(&PointerId, &mut PointerLocation, &mut PointerPress)>,
    ) {
        for event in events.read() {
            match event.action {
                PointerAction::Press(button) => {
                    pointers
                        .iter_mut()
                        .for_each(|(pointer_id, _, mut pointer)| {
                            if *pointer_id == event.pointer_id {
                                match button {
                                    PointerButton::Primary => pointer.primary = true,
                                    PointerButton::Secondary => pointer.secondary = true,
                                    PointerButton::Middle => pointer.middle = true,
                                }
                            }
                        });
                }
                PointerAction::Release(button) => {
                    pointers
                        .iter_mut()
                        .for_each(|(pointer_id, _, mut pointer)| {
                            if *pointer_id == event.pointer_id {
                                match button {
                                    PointerButton::Primary => pointer.primary = false,
                                    PointerButton::Secondary => pointer.secondary = false,
                                    PointerButton::Middle => pointer.middle = false,
                                }
                            }
                        });
                }
                PointerAction::Move { .. } => {
                    pointers.iter_mut().for_each(|(id, mut pointer, _)| {
                        if *id == event.pointer_id {
                            pointer.location = Some(event.location.to_owned());
                        }
                    });
                }
                _ => {}
            }
        }
    }
}
