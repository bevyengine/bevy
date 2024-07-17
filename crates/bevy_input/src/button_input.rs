//! The generic input type.

use bevy_ecs::system::Resource;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_utils::HashSet;
use std::hash::Hash;

/// A "press-able" input of type `T`.
///
/// ## Usage
///
/// This type can be used as a resource to keep the current state of an input, by reacting to
/// events from the input. For a given input value:
///
/// * [`ButtonInput::pressed`] will return `true` between a press and a release event.
/// * [`ButtonInput::just_pressed`] will return `true` for one frame after a press event.
/// * [`ButtonInput::just_released`] will return `true` for one frame after a release event.
///
/// ## Multiple systems
///
/// In case multiple systems are checking for [`ButtonInput::just_pressed`] or [`ButtonInput::just_released`]
/// but only one should react, for example when modifying a
/// [`Resource`], you should consider clearing the input state, either by:
///
/// * Using [`ButtonInput::clear_just_pressed`] or [`ButtonInput::clear_just_released`] instead.
/// * Calling [`ButtonInput::clear`] or [`ButtonInput::reset`] immediately after the state change.
///
/// ## Performance
///
/// For all operations, the following conventions are used:
/// - **n** is the number of stored inputs.
/// - **m** is the number of input arguments passed to the method.
/// - **\***-suffix denotes an amortized cost.
/// - **~**-suffix denotes an expected cost.
///
/// See Rust's [std::collections doc on performance](https://doc.rust-lang.org/std/collections/index.html#performance) for more details on the conventions used here.
///
/// | **[`ButtonInput`] operations**          | **Computational complexity** |
/// |-----------------------------------|------------------------------------|
/// | [`ButtonInput::any_just_pressed`]       | *O*(m)~                      |
/// | [`ButtonInput::any_just_released`]      | *O*(m)~                      |
/// | [`ButtonInput::any_pressed`]            | *O*(m)~                      |
/// | [`ButtonInput::get_just_pressed`]       | *O*(n)                       |
/// | [`ButtonInput::get_just_released`]      | *O*(n)                       |
/// | [`ButtonInput::get_pressed`]            | *O*(n)                       |
/// | [`ButtonInput::just_pressed`]           | *O*(1)~                      |
/// | [`ButtonInput::just_released`]          | *O*(1)~                      |
/// | [`ButtonInput::pressed`]                | *O*(1)~                      |
/// | [`ButtonInput::press`]                  | *O*(1)~*                     |
/// | [`ButtonInput::release`]                | *O*(1)~*                     |
/// | [`ButtonInput::release_all`]            | *O*(n)~*                     |
/// | [`ButtonInput::clear_just_pressed`]     | *O*(1)~                      |
/// | [`ButtonInput::clear_just_released`]    | *O*(1)~                      |
/// | [`ButtonInput::reset_all`]              | *O*(n)                       |
/// | [`ButtonInput::clear`]                  | *O*(n)                       |
///
/// ## Window focus
///
/// `ButtonInput<KeyCode>` is tied to window focus. For example, if the user holds a button
/// while the window loses focus, [`ButtonInput::just_released`] will be triggered. Similarly if the window
/// regains focus, [`ButtonInput::just_pressed`] will be triggered. Currently this happens even if the
/// focus switches from one Bevy window to another (for example because a new window was just spawned).
///
/// `ButtonInput<GamepadButton>` is independent of window focus.
///
/// ## Examples
///
/// Reading and checking against the current set of pressed buttons:
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, Update};
/// # use bevy_ecs::{prelude::{IntoSystemConfigs, Res, Resource, resource_changed}, schedule::Condition};
/// # use bevy_input::{ButtonInput, prelude::{GamepadButton, KeyCode, MouseButton}};
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(
///             Update,
///             print_gamepad.run_if(resource_changed::<ButtonInput<GamepadButton>>),
///         )
///         .add_systems(
///             Update,
///             print_mouse.run_if(resource_changed::<ButtonInput<MouseButton>>),
///         )
///         .add_systems(
///             Update,
///             print_keyboard.run_if(resource_changed::<ButtonInput<KeyCode>>),
///         )
///         .run();
/// }
///
/// fn print_gamepad(gamepad: Res<ButtonInput<GamepadButton>>) {
///     println!("Gamepad: {:?}", gamepad.get_pressed().collect::<Vec<_>>());
/// }
///
/// fn print_mouse(mouse: Res<ButtonInput<MouseButton>>) {
///     println!("Mouse: {:?}", mouse.get_pressed().collect::<Vec<_>>());
/// }
///
/// fn print_keyboard(keyboard: Res<ButtonInput<KeyCode>>) {
///     if keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
///         && keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight])
///         && keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
///         && keyboard.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight])
///         && keyboard.pressed(KeyCode::KeyL)
///     {
///         println!("On Windows this opens LinkedIn.");
///     } else {
///         println!("keyboard: {:?}", keyboard.get_pressed().collect::<Vec<_>>());
///     }
/// }
/// ```
///
/// Accepting input from multiple devices:
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, Update};
/// # use bevy_ecs::{prelude::IntoSystemConfigs, schedule::Condition};
/// # use bevy_input::{ButtonInput, common_conditions::{input_just_pressed}, prelude::{GamepadButton, Gamepad, GamepadButtonType, KeyCode}};
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(
///             Update,
///             something_used.run_if(
///                 input_just_pressed(KeyCode::KeyE)
///                     .or_else(input_just_pressed(GamepadButton::new(
///                         Gamepad::new(0),
///                         GamepadButtonType::West,
///                     ))),
///             ),
///         )
///         .run();
/// }
///
/// fn something_used() {
///     println!("Generic use-ish button pressed.");
/// }
/// ```
///
/// ## Note
///
/// When adding this resource for a new input type, you should:
///
/// * Call the [`ButtonInput::press`] method for each press event.
/// * Call the [`ButtonInput::release`] method for each release event.
/// * Call the [`ButtonInput::clear`] method at each frame start, before processing events.
///
/// Note: Calling `clear` from a [`ResMut`] will trigger change detection.
/// It may be preferable to use [`DetectChangesMut::bypass_change_detection`]
/// to avoid causing the resource to always be marked as changed.
///
///[`ResMut`]: bevy_ecs::system::ResMut
///[`DetectChangesMut::bypass_change_detection`]: bevy_ecs::change_detection::DetectChangesMut::bypass_change_detection
#[derive(Debug, Clone, Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Default))]
pub struct ButtonInput<T: Copy + Eq + Hash + Send + Sync + 'static> {
    /// A collection of every button that is currently being pressed.
    pressed: HashSet<T>,
    /// A collection of every button that has just been pressed.
    just_pressed: HashSet<T>,
    /// A collection of every button that has just been released.
    just_released: HashSet<T>,
}

impl<T: Copy + Eq + Hash + Send + Sync + 'static> Default for ButtonInput<T> {
    fn default() -> Self {
        Self {
            pressed: Default::default(),
            just_pressed: Default::default(),
            just_released: Default::default(),
        }
    }
}

impl<T> ButtonInput<T>
where
    T: Copy + Eq + Hash + Send + Sync + 'static,
{
    /// Registers a press for the given `input`.
    pub fn press(&mut self, input: T) {
        // Returns `true` if the `input` wasn't pressed.
        if self.pressed.insert(input) {
            self.just_pressed.insert(input);
        }
    }

    /// Returns `true` if the `input` has been pressed.
    pub fn pressed(&self, input: T) -> bool {
        self.pressed.contains(&input)
    }

    /// Returns `true` if any item in `inputs` has been pressed.
    pub fn any_pressed(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.pressed(it))
    }

    /// Returns `true` if all items in `inputs` have been pressed.
    pub fn all_pressed(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().all(|it| self.pressed(it))
    }

    /// Registers a release for the given `input`.
    pub fn release(&mut self, input: T) {
        // Returns `true` if the `input` was pressed.
        if self.pressed.remove(&input) {
            self.just_released.insert(input);
        }
    }

    /// Registers a release for all currently pressed inputs.
    pub fn release_all(&mut self) {
        // Move all items from pressed into just_released
        self.just_released.extend(self.pressed.drain());
    }

    /// Returns `true` if the `input` has been pressed during the current frame.
    ///
    /// Note: This function does not imply information regarding the current state of [`ButtonInput::pressed`] or [`ButtonInput::just_released`].
    pub fn just_pressed(&self, input: T) -> bool {
        self.just_pressed.contains(&input)
    }

    /// Returns `true` if any item in `inputs` has been pressed during the current frame.
    pub fn any_just_pressed(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.just_pressed(it))
    }

    /// Clears the `just_pressed` state of the `input` and returns `true` if the `input` has just been pressed.
    ///
    /// Future calls to [`ButtonInput::just_pressed`] for the given input will return false until a new press event occurs.
    pub fn clear_just_pressed(&mut self, input: T) -> bool {
        self.just_pressed.remove(&input)
    }

    /// Returns `true` if the `input` has been released during the current frame.
    ///
    /// Note: This function does not imply information regarding the current state of [`ButtonInput::pressed`] or [`ButtonInput::just_pressed`].
    pub fn just_released(&self, input: T) -> bool {
        self.just_released.contains(&input)
    }

    /// Returns `true` if any item in `inputs` has just been released.
    pub fn any_just_released(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.just_released(it))
    }

    /// Clears the `just_released` state of the `input` and returns `true` if the `input` has just been released.
    ///
    /// Future calls to [`ButtonInput::just_released`] for the given input will return false until a new release event occurs.
    pub fn clear_just_released(&mut self, input: T) -> bool {
        self.just_released.remove(&input)
    }

    /// Clears the `pressed`, `just_pressed` and `just_released` data of the `input`.
    pub fn reset(&mut self, input: T) {
        self.pressed.remove(&input);
        self.just_pressed.remove(&input);
        self.just_released.remove(&input);
    }

    /// Clears the `pressed`, `just_pressed`, and `just_released` data for every input.
    ///
    /// See also [`ButtonInput::clear`] for simulating elapsed time steps.
    pub fn reset_all(&mut self) {
        self.pressed.clear();
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// Clears the `just pressed` and `just released` data for every input.
    ///
    /// See also [`ButtonInput::reset_all`] for a full reset.
    pub fn clear(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// An iterator visiting every pressed input in arbitrary order.
    pub fn get_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.pressed.iter()
    }

    /// An iterator visiting every just pressed input in arbitrary order.
    ///
    /// Note: Returned elements do not imply information regarding the current state of [`ButtonInput::pressed`] or [`ButtonInput::just_released`].
    pub fn get_just_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_pressed.iter()
    }

    /// An iterator visiting every just released input in arbitrary order.
    ///
    /// Note: Returned elements do not imply information regarding the current state of [`ButtonInput::pressed`] or [`ButtonInput::just_pressed`].
    pub fn get_just_released(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_released.iter()
    }
}

#[cfg(test)]
mod test {
    use crate::ButtonInput;

    /// Used for testing the functionality of [`ButtonInput`].
    #[derive(Copy, Clone, Eq, PartialEq, Hash)]
    enum DummyInput {
        Input1,
        Input2,
    }

    #[test]
    fn test_press() {
        let mut input = ButtonInput::default();
        assert!(!input.pressed.contains(&DummyInput::Input1));
        assert!(!input.just_pressed.contains(&DummyInput::Input1));
        input.press(DummyInput::Input1);
        assert!(input.just_pressed.contains(&DummyInput::Input1));
        assert!(input.pressed.contains(&DummyInput::Input1));
    }

    #[test]
    fn test_pressed() {
        let mut input = ButtonInput::default();
        assert!(!input.pressed(DummyInput::Input1));
        input.press(DummyInput::Input1);
        assert!(input.pressed(DummyInput::Input1));
    }

    #[test]
    fn test_any_pressed() {
        let mut input = ButtonInput::default();
        assert!(!input.any_pressed([DummyInput::Input1]));
        assert!(!input.any_pressed([DummyInput::Input2]));
        assert!(!input.any_pressed([DummyInput::Input1, DummyInput::Input2]));
        input.press(DummyInput::Input1);
        assert!(input.any_pressed([DummyInput::Input1]));
        assert!(!input.any_pressed([DummyInput::Input2]));
        assert!(input.any_pressed([DummyInput::Input1, DummyInput::Input2]));
    }

    #[test]
    fn test_all_pressed() {
        let mut input = ButtonInput::default();
        assert!(!input.all_pressed([DummyInput::Input1]));
        assert!(!input.all_pressed([DummyInput::Input2]));
        assert!(!input.all_pressed([DummyInput::Input1, DummyInput::Input2]));
        input.press(DummyInput::Input1);
        assert!(input.all_pressed([DummyInput::Input1]));
        assert!(!input.all_pressed([DummyInput::Input1, DummyInput::Input2]));
        input.press(DummyInput::Input2);
        assert!(input.all_pressed([DummyInput::Input1, DummyInput::Input2]));
    }

    #[test]
    fn test_release() {
        let mut input = ButtonInput::default();
        input.press(DummyInput::Input1);
        assert!(input.pressed.contains(&DummyInput::Input1));
        assert!(!input.just_released.contains(&DummyInput::Input1));
        input.release(DummyInput::Input1);
        assert!(!input.pressed.contains(&DummyInput::Input1));
        assert!(input.just_released.contains(&DummyInput::Input1));
    }

    #[test]
    fn test_release_all() {
        let mut input = ButtonInput::default();
        input.press(DummyInput::Input1);
        input.press(DummyInput::Input2);
        input.release_all();
        assert!(input.pressed.is_empty());
        assert!(input.just_released.contains(&DummyInput::Input1));
        assert!(input.just_released.contains(&DummyInput::Input2));
    }

    #[test]
    fn test_just_pressed() {
        let mut input = ButtonInput::default();
        assert!(!input.just_pressed(DummyInput::Input1));
        input.press(DummyInput::Input1);
        assert!(input.just_pressed(DummyInput::Input1));
    }

    #[test]
    fn test_any_just_pressed() {
        let mut input = ButtonInput::default();
        assert!(!input.any_just_pressed([DummyInput::Input1]));
        assert!(!input.any_just_pressed([DummyInput::Input2]));
        assert!(!input.any_just_pressed([DummyInput::Input1, DummyInput::Input2]));
        input.press(DummyInput::Input1);
        assert!(input.any_just_pressed([DummyInput::Input1]));
        assert!(!input.any_just_pressed([DummyInput::Input2]));
        assert!(input.any_just_pressed([DummyInput::Input1, DummyInput::Input2]));
    }

    #[test]
    fn test_clear_just_pressed() {
        let mut input = ButtonInput::default();
        input.press(DummyInput::Input1);
        assert!(input.just_pressed(DummyInput::Input1));
        input.clear_just_pressed(DummyInput::Input1);
        assert!(!input.just_pressed(DummyInput::Input1));
    }

    #[test]
    fn test_just_released() {
        let mut input = ButtonInput::default();
        input.press(DummyInput::Input1);
        assert!(!input.just_released(DummyInput::Input1));
        input.release(DummyInput::Input1);
        assert!(input.just_released(DummyInput::Input1));
    }

    #[test]
    fn test_any_just_released() {
        let mut input = ButtonInput::default();
        input.press(DummyInput::Input1);
        assert!(!input.any_just_released([DummyInput::Input1]));
        assert!(!input.any_just_released([DummyInput::Input2]));
        assert!(!input.any_just_released([DummyInput::Input1, DummyInput::Input2]));
        input.release(DummyInput::Input1);
        assert!(input.any_just_released([DummyInput::Input1]));
        assert!(!input.any_just_released([DummyInput::Input2]));
        assert!(input.any_just_released([DummyInput::Input1, DummyInput::Input2]));
    }

    #[test]
    fn test_clear_just_released() {
        let mut input = ButtonInput::default();
        input.press(DummyInput::Input1);
        input.release(DummyInput::Input1);
        assert!(input.just_released(DummyInput::Input1));
        input.clear_just_released(DummyInput::Input1);
        assert!(!input.just_released(DummyInput::Input1));
    }

    #[test]
    fn test_reset() {
        let mut input = ButtonInput::default();

        // Pressed
        input.press(DummyInput::Input1);
        assert!(input.pressed(DummyInput::Input1));
        assert!(input.just_pressed(DummyInput::Input1));
        assert!(!input.just_released(DummyInput::Input1));
        input.reset(DummyInput::Input1);
        assert!(!input.pressed(DummyInput::Input1));
        assert!(!input.just_pressed(DummyInput::Input1));
        assert!(!input.just_released(DummyInput::Input1));

        // Released
        input.press(DummyInput::Input1);
        input.release(DummyInput::Input1);
        assert!(!input.pressed(DummyInput::Input1));
        assert!(input.just_pressed(DummyInput::Input1));
        assert!(input.just_released(DummyInput::Input1));
        input.reset(DummyInput::Input1);
        assert!(!input.pressed(DummyInput::Input1));
        assert!(!input.just_pressed(DummyInput::Input1));
        assert!(!input.just_released(DummyInput::Input1));
    }

    #[test]
    fn test_reset_all() {
        let mut input = ButtonInput::default();

        input.press(DummyInput::Input1);
        input.press(DummyInput::Input2);
        input.release(DummyInput::Input2);
        assert!(input.pressed.contains(&DummyInput::Input1));
        assert!(input.just_pressed.contains(&DummyInput::Input1));
        assert!(input.just_released.contains(&DummyInput::Input2));
        input.reset_all();
        assert!(input.pressed.is_empty());
        assert!(input.just_pressed.is_empty());
        assert!(input.just_released.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut input = ButtonInput::default();

        // Pressed
        input.press(DummyInput::Input1);
        assert!(input.pressed(DummyInput::Input1));
        assert!(input.just_pressed(DummyInput::Input1));
        assert!(!input.just_released(DummyInput::Input1));
        input.clear();
        assert!(input.pressed(DummyInput::Input1));
        assert!(!input.just_pressed(DummyInput::Input1));
        assert!(!input.just_released(DummyInput::Input1));

        // Released
        input.press(DummyInput::Input1);
        input.release(DummyInput::Input1);
        assert!(!input.pressed(DummyInput::Input1));
        assert!(!input.just_pressed(DummyInput::Input1));
        assert!(input.just_released(DummyInput::Input1));
        input.clear();
        assert!(!input.pressed(DummyInput::Input1));
        assert!(!input.just_pressed(DummyInput::Input1));
        assert!(!input.just_released(DummyInput::Input1));
    }

    #[test]
    fn test_get_pressed() {
        let mut input = ButtonInput::default();
        input.press(DummyInput::Input1);
        input.press(DummyInput::Input2);
        let pressed = input.get_pressed();
        assert_eq!(pressed.len(), 2);
        for pressed_input in pressed {
            assert!(input.pressed.contains(pressed_input));
        }
    }

    #[test]
    fn test_get_just_pressed() {
        let mut input = ButtonInput::default();
        input.press(DummyInput::Input1);
        input.press(DummyInput::Input2);
        let just_pressed = input.get_just_pressed();
        assert_eq!(just_pressed.len(), 2);
        for just_pressed_input in just_pressed {
            assert!(input.just_pressed.contains(just_pressed_input));
        }
    }

    #[test]
    fn test_get_just_released() {
        let mut input = ButtonInput::default();
        input.press(DummyInput::Input1);
        input.press(DummyInput::Input2);
        input.release(DummyInput::Input1);
        input.release(DummyInput::Input2);
        let just_released = input.get_just_released();
        assert_eq!(just_released.len(), 2);
        for just_released_input in just_released {
            assert!(input.just_released.contains(just_released_input));
        }
    }

    #[test]
    fn test_general_input_handling() {
        let mut input = ButtonInput::default();

        // Test pressing
        input.press(DummyInput::Input1);
        input.press(DummyInput::Input2);

        // Check if they were `just_pressed` (pressed on this update)
        assert!(input.just_pressed(DummyInput::Input1));
        assert!(input.just_pressed(DummyInput::Input2));

        // Check if they are also marked as pressed
        assert!(input.pressed(DummyInput::Input1));
        assert!(input.pressed(DummyInput::Input2));

        // Clear the `input`, removing `just_pressed` and `just_released`
        input.clear();

        // Check if they're marked `just_pressed`
        assert!(!input.just_pressed(DummyInput::Input1));
        assert!(!input.just_pressed(DummyInput::Input2));

        // Check if they're marked as pressed
        assert!(input.pressed(DummyInput::Input1));
        assert!(input.pressed(DummyInput::Input2));

        // Release the inputs and check state
        input.release(DummyInput::Input1);
        input.release(DummyInput::Input2);

        // Check if they're marked as `just_released` (released on this update)
        assert!(input.just_released(DummyInput::Input1));
        assert!(input.just_released(DummyInput::Input2));

        // Check that they're not incorrectly marked as pressed
        assert!(!input.pressed(DummyInput::Input1));
        assert!(!input.pressed(DummyInput::Input2));

        // Clear the `Input` and check for removal from `just_released`
        input.clear();

        // Check that they're not incorrectly marked as just released
        assert!(!input.just_released(DummyInput::Input1));
        assert!(!input.just_released(DummyInput::Input2));

        // Set up an `Input` to test resetting
        let mut input = ButtonInput::default();

        input.press(DummyInput::Input1);
        input.release(DummyInput::Input2);

        // Reset the `Input` and test if it was reset correctly
        input.reset(DummyInput::Input1);
        input.reset(DummyInput::Input2);

        assert!(!input.just_pressed(DummyInput::Input1));
        assert!(!input.pressed(DummyInput::Input1));
        assert!(!input.just_released(DummyInput::Input2));
    }
}
