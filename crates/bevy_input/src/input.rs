//! The generic input type.

use bevy_ecs::system::Resource;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_utils::HashSet;
use std::hash::Hash;

// unused import, but needed for intra doc link to work
#[allow(unused_imports)]
use bevy_ecs::schedule::State;

/// A "press-able" input of type `T`.
///
/// ## Usage
///
/// This type can be used as a resource to keep the current state of an input, by reacting to
/// events from the input. For a given input value:
///
/// * [`Input::pressed`] will return `true` between a press and a release event.
/// * [`Input::just_pressed`] will return `true` for one frame after a press event.
/// * [`Input::just_released`] will return `true` for one frame after a release event.
///
/// ## Multiple systems
///
/// In case multiple systems are checking for [`Input::just_pressed`] or [`Input::just_released`]
/// but only one should react, for example in the case of triggering
/// [`State`](bevy_ecs::schedule::State) change, you should consider clearing the input state, either by:
///
/// * Using [`Input::clear_just_pressed`] or [`Input::clear_just_released`] instead.
/// * Calling [`Input::clear`] or [`Input::reset`] immediately after the state change.
///
/// ## Note
///
/// When adding this resource for a new input type, you should:
///
/// * Call the [`Input::press`] method for each press event.
/// * Call the [`Input::release`] method for each release event.
/// * Call the [`Input::clear`] method at each frame start, before processing events.
///
/// Note: Calling `clear` from a [`ResMut`] will trigger change detection.
/// It may be preferable to use [`DetectChangesMut::bypass_change_detection`]
/// to avoid causing the resource to always be marked as changed.
///
///[`ResMut`]: bevy_ecs::system::ResMut
///[`DetectChangesMut::bypass_change_detection`]: bevy_ecs::change_detection::DetectChangesMut::bypass_change_detection
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Default)]
pub struct Input<T: Copy + Eq + Hash + Send + Sync + 'static> {
    /// A collection of every button that is currently being pressed.
    pressed: HashSet<T>,
    /// A collection of every button that has just been pressed.
    just_pressed: HashSet<T>,
    /// A collection of every button that has just been released.
    just_released: HashSet<T>,
}

impl<T: Copy + Eq + Hash + Send + Sync + 'static> Default for Input<T> {
    fn default() -> Self {
        Self {
            pressed: Default::default(),
            just_pressed: Default::default(),
            just_released: Default::default(),
        }
    }
}

impl<T> Input<T>
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

    /// Returns `true` if the `input` has just been pressed.
    pub fn just_pressed(&self, input: T) -> bool {
        self.just_pressed.contains(&input)
    }

    /// Returns `true` if any item in `inputs` has just been pressed.
    pub fn any_just_pressed(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.just_pressed(it))
    }

    /// Clears the `just_pressed` state of the `input` and returns `true` if the `input` has just been pressed.
    ///
    /// Future calls to [`Input::just_pressed`] for the given input will return false until a new press event occurs.
    pub fn clear_just_pressed(&mut self, input: T) -> bool {
        self.just_pressed.remove(&input)
    }

    /// Returns `true` if the `input` has just been released.
    pub fn just_released(&self, input: T) -> bool {
        self.just_released.contains(&input)
    }

    /// Returns `true` if any item in `inputs` has just been released.
    pub fn any_just_released(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.just_released(it))
    }

    /// Clears the `just_released` state of the `input` and returns `true` if the `input` has just been released.
    ///
    /// Future calls to [`Input::just_released`] for the given input will return false until a new release event occurs.
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
    /// See also [`Input::clear`] for simulating elapsed time steps.
    pub fn reset_all(&mut self) {
        self.pressed.clear();
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// Clears the `just pressed` and `just released` data for every input.
    ///
    /// See also [`Input::reset_all`] for a full reset.
    pub fn clear(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// An iterator visiting every pressed input in arbitrary order.
    pub fn get_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.pressed.iter()
    }

    /// An iterator visiting every just pressed input in arbitrary order.
    pub fn get_just_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_pressed.iter()
    }

    /// An iterator visiting every just released input in arbitrary order.
    pub fn get_just_released(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_released.iter()
    }
}

#[cfg(test)]
mod test {
    use bevy_reflect::TypePath;

    use crate::Input;

    /// Used for testing the functionality of [`Input`].
    #[derive(TypePath, Copy, Clone, Eq, PartialEq, Hash)]
    enum DummyInput {
        Input1,
        Input2,
    }

    #[test]
    fn test_press() {
        let mut input = Input::default();
        assert!(!input.pressed.contains(&DummyInput::Input1));
        assert!(!input.just_pressed.contains(&DummyInput::Input1));
        input.press(DummyInput::Input1);
        assert!(input.just_pressed.contains(&DummyInput::Input1));
        assert!(input.pressed.contains(&DummyInput::Input1));
    }

    #[test]
    fn test_pressed() {
        let mut input = Input::default();
        assert!(!input.pressed(DummyInput::Input1));
        input.press(DummyInput::Input1);
        assert!(input.pressed(DummyInput::Input1));
    }

    #[test]
    fn test_any_pressed() {
        let mut input = Input::default();
        assert!(!input.any_pressed([DummyInput::Input1]));
        assert!(!input.any_pressed([DummyInput::Input2]));
        assert!(!input.any_pressed([DummyInput::Input1, DummyInput::Input2]));
        input.press(DummyInput::Input1);
        assert!(input.any_pressed([DummyInput::Input1]));
        assert!(!input.any_pressed([DummyInput::Input2]));
        assert!(input.any_pressed([DummyInput::Input1, DummyInput::Input2]));
    }

    #[test]
    fn test_release() {
        let mut input = Input::default();
        input.press(DummyInput::Input1);
        assert!(input.pressed.contains(&DummyInput::Input1));
        assert!(!input.just_released.contains(&DummyInput::Input1));
        input.release(DummyInput::Input1);
        assert!(!input.pressed.contains(&DummyInput::Input1));
        assert!(input.just_released.contains(&DummyInput::Input1));
    }

    #[test]
    fn test_release_all() {
        let mut input = Input::default();
        input.press(DummyInput::Input1);
        input.press(DummyInput::Input2);
        input.release_all();
        assert!(input.pressed.is_empty());
        assert!(input.just_released.contains(&DummyInput::Input1));
        assert!(input.just_released.contains(&DummyInput::Input2));
    }

    #[test]
    fn test_just_pressed() {
        let mut input = Input::default();
        assert!(!input.just_pressed(DummyInput::Input1));
        input.press(DummyInput::Input1);
        assert!(input.just_pressed(DummyInput::Input1));
    }

    #[test]
    fn test_any_just_pressed() {
        let mut input = Input::default();
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
        let mut input = Input::default();
        input.press(DummyInput::Input1);
        assert!(input.just_pressed(DummyInput::Input1));
        input.clear_just_pressed(DummyInput::Input1);
        assert!(!input.just_pressed(DummyInput::Input1));
    }

    #[test]
    fn test_just_released() {
        let mut input = Input::default();
        input.press(DummyInput::Input1);
        assert!(!input.just_released(DummyInput::Input1));
        input.release(DummyInput::Input1);
        assert!(input.just_released(DummyInput::Input1));
    }

    #[test]
    fn test_any_just_released() {
        let mut input = Input::default();
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
        let mut input = Input::default();
        input.press(DummyInput::Input1);
        input.release(DummyInput::Input1);
        assert!(input.just_released(DummyInput::Input1));
        input.clear_just_released(DummyInput::Input1);
        assert!(!input.just_released(DummyInput::Input1));
    }

    #[test]
    fn test_reset() {
        let mut input = Input::default();

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
        let mut input = Input::default();

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
        let mut input = Input::default();

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
        let mut input = Input::default();
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
        let mut input = Input::default();
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
        let mut input = Input::default();
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
        let mut input = Input::default();

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
        let mut input = Input::default();

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
