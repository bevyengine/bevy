use crate::{
    mouse::{MouseButton, MouseScrollUnit},
    ElementState,
};
use bevy_math::Vec2;

/// A mouse button input event.
///
/// This event is the translated version of the `WindowEvent::MouseInput` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
///
/// ## Access
///
/// To access or send mouse input events use one of the following:
/// - To access mouse input events: `EventReader<MouseButtonInput>`
/// - To send mouse input events: `EventWriter<MouseButtonInput>`
///
/// ## Usage
///
/// The event is read inside of the [`mouse_button_input_system`](crate::mouse::mouse_button_input_system)
/// to update the [`Input<MouseButton>`](crate::Input<MouseButton>) resource.
#[derive(Debug, Clone)]
pub struct MouseButtonInput {
    /// The mouse button assigned to the event.
    pub button: MouseButton,
    /// The state of the [`MouseButton`].
    pub state: ElementState,
}

impl MouseButtonInput {
    /// Creates a new [`MouseButtonInput`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::{
    /// #     ElementState,
    /// #     mouse::{MouseButtonInput, MouseButton}
    /// # };
    /// #
    /// let mouse_button_input = MouseButtonInput::new(
    ///     MouseButton::Left,
    ///     ElementState::Pressed,
    /// );
    /// ```
    pub fn new(button: MouseButton, state: ElementState) -> Self {
        Self { button, state }
    }
}

/// A mouse motion event.
///
/// This event is the translated version of the `DeviceEvent::MouseMotion` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
///
/// ## Access
///
/// To access or send mouse motion events use one of the following:
/// - To access mouse motion events: `EventReader<MouseMotion>`
/// - To send mouse motion events: `EventWriter<MouseMotion>`
#[derive(Debug, Clone)]
pub struct MouseMotion {
    /// The delta of the previous and current mouse position.
    pub delta: Vec2,
}

impl MouseMotion {
    /// Creates a new [`MouseMotion`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::mouse::MouseMotion;
    /// # use bevy_math::Vec2;
    /// #
    /// let mouse_motion = MouseMotion::new(Vec2::new(1.0, 1.0));
    /// ```
    pub fn new(delta: Vec2) -> Self {
        Self { delta }
    }
}

/// A mouse wheel event.
///
/// This event is the translated version of the `WindowEvent::MouseWheel` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
///
/// ## Access
///
/// To access or send mouse wheel events use one of the following:
/// - To access mouse wheel events: `EventReader<MouseWheel>`
/// - To send mouse wheel events: `EventWriter<MouseWheel>`
#[derive(Debug, Clone)]
pub struct MouseWheel {
    /// The mouse scroll unit.
    pub unit: MouseScrollUnit,
    /// The horizontal scroll value.
    pub x: f32,
    /// The vertical scroll value.
    pub y: f32,
}

impl MouseWheel {
    /// Creates a new [`MouseWheel`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::mouse::{MouseWheel, MouseScrollUnit};
    /// #
    /// let mouse_wheel = MouseWheel::new(
    ///     MouseScrollUnit::Line,
    ///     1.0,
    ///     2.0,
    /// );
    /// ```
    pub fn new(unit: MouseScrollUnit, x: f32, y: f32) -> Self {
        Self { unit, x, y }
    }
}
