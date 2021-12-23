use bevy_utils::{HashMap, HashSet};
use std::{fmt::Debug, hash::Hash};
use strum::IntoEnumIterator;

// unused import, but needed for intra doc link to work
#[allow(unused_imports)]
use bevy_ecs::schedule::State;

/// A "press-able" input of type `T`.
///
/// Pressable inputs of this sort can either be continuous or discrete.
/// Their [`value`](Self::value) will always be between 0.0 and 1.0,
/// where 0.0 represents a fully released state and 1.0 is the fully pressed state.
/// If you need to represent an input value with a neutral position and a direction,
/// use an [Axis](crate::Axis) instead.
///
/// This type can be used as a resource to keep the current state of an input, by reacting to
/// events from the input. For a given input value:
///
/// * [`Input::pressed`] will return `true` between a press and a release event.
/// * [`Input::just_pressed`] will return `true` for one frame after a press event.
/// * [`Input::just_released`] will return `true` for one frame after a release event.
///
/// In case multiple systems are checking for [`Input::just_pressed`] or [`Input::just_released`]
/// but only one should react, for example in the case of triggering
/// [`State`] change, you should consider clearing the input state, either by:
///
/// * Using [`Input::clear_just_pressed`] or [`Input::clear_just_released`] instead.
/// * Calling [`Input::clear`] or [`Input::reset`] immediately after the state change.
///
/// ## Notes when adding this resource for a new input type
///
/// When adding this resource for a new input type, you should:
///
/// * Call the [`Input::press`] method for each press event.
/// * Call the [`Input::release`] method for each release event.
/// * Call the [`Input::clear`] method at each frame start, before processing events.
#[derive(Debug)]
pub struct Input<T: Inputlike> {
    pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    just_released: HashSet<T>,
    values: HashMap<T, f32>,
}

/// Allows a type to be used with the [Input] type
///
/// The [IntoEnumIterator] trait bound on this assocaited type allows users to iterate over all possible valid values of an input.
/// If you are looking to implement this trait for you own type, you will need to derive that trait using the `strum` crate.
pub trait Inputlike: Clone + Copy + PartialEq + Eq + Hash + IntoEnumIterator {}

impl<T: Inputlike> Default for Input<T> {
    fn default() -> Self {
        // PERF: this is pointlessly slow;
        // should use HashMap::from_iter() instead
        let mut values = HashMap::default();

        for input_variant in T::iter() {
            values.insert(input_variant, 0.0);
        }

        Self {
            pressed: Default::default(),
            just_pressed: Default::default(),
            just_released: Default::default(),
            values,
        }
    }
}

impl<T: Inputlike> Input<T>
where
    T: Copy + Eq + Hash,
{
    /// Register a press for input `input`.
    #[inline]
    pub fn press(&mut self, input: T) {
        if !self.pressed(input) {
            self.just_pressed.insert(input);
        }

        self.pressed.insert(input);
        self.values.insert(input, 1.0);
    }

    /// Check if `input` has been pressed.
    #[inline]
    pub fn pressed(&self, input: T) -> bool {
        self.pressed.contains(&input)
    }

    /// Check if any item in `inputs` has been pressed.
    #[inline]
    pub fn any_pressed(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.pressed(it))
    }

    /// Register a release for input `input`.
    #[inline]
    pub fn release(&mut self, input: T) {
        self.pressed.remove(&input);
        self.just_released.insert(input);
        self.values.insert(input, 0.0);
    }

    /// Check if `input` has been just pressed.
    #[inline]
    pub fn just_pressed(&self, input: T) -> bool {
        self.just_pressed.contains(&input)
    }

    /// Check if any item in `inputs` has just been pressed.
    #[inline]
    pub fn any_just_pressed(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.just_pressed(it))
    }

    /// Clear the "just pressed" state of `input`. Future calls to [`Input::just_pressed`] for the
    /// given input will return false until a new press event occurs.
    /// Returns true if `input` is currently "just pressed"
    #[inline]
    pub fn clear_just_pressed(&mut self, input: T) -> bool {
        self.just_pressed.remove(&input)
    }

    /// Check if `input` has been just released.
    #[inline]
    pub fn just_released(&self, input: T) -> bool {
        self.just_released.contains(&input)
    }

    /// Check if any item in `inputs` has just been released.
    #[inline]
    pub fn any_just_released(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.just_released(it))
    }

    /// Clear the "just released" state of `input`. Future calls to [`Input::just_released`] for the
    /// given input will return false until a new release event occurs.
    /// Returns true if `input` is currently "just released"
    #[inline]
    pub fn clear_just_released(&mut self, input: T) -> bool {
        self.just_released.remove(&input)
    }

    /// Returns the degree to which an input is pressed: 0.0 if it is fully released, and 1.0 if it is fully pressed
    ///
    /// Most buttons and other inputs are fully binary,
    /// and this method will only ever return 0.0 or 1.0.
    /// Values returned will always be in [0.0, 1.0].
    #[inline]
    pub fn value(&self, input: T) -> f32 {
        *self
            .values
            .get(&input)
            .expect("Input value not found in in Input<T> resource.")
    }

    /// Manually set the value of an input
    ///
    /// This is particularly useful for mocking analogue inputs during tests.
    #[inline]
    pub fn set_value(&mut self, input: T, value: f32) {
        self.values.insert(input, value.clamp(0.0, 1.0));
    }

    /// Reset all status for input `input`.
    #[inline]
    pub fn reset(&mut self, input: T) {
        self.pressed.remove(&input);
        self.just_pressed.remove(&input);
        self.just_released.remove(&input);
    }

    /// Clear just pressed and just released information.
    #[inline]
    pub fn clear(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// List all inputs that are pressed.
    #[inline]
    pub fn get_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.pressed.iter()
    }

    /// List all inputs that are just pressed.
    #[inline]
    pub fn get_just_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_pressed.iter()
    }

    /// List all inputs that are just released.
    #[inline]
    pub fn get_just_released(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_released.iter()
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn input_test() {
        use crate::{Input, Inputlike};
        use strum_macros::EnumIter;

        /// Used for testing `Input` functionality
        #[derive(Copy, Clone, Eq, PartialEq, Hash, EnumIter, Debug)]
        enum DummyInput {
            Input1,
            Input2,
        }

        impl Inputlike for DummyInput {}

        let mut input = Input::default();

        // Test pressing
        input.press(DummyInput::Input1);
        input.press(DummyInput::Input2);

        // Check if they were "just pressed" (pressed on this update)
        assert!(input.just_pressed(DummyInput::Input1));
        assert!(input.just_pressed(DummyInput::Input2));

        // Check if they are also marked as pressed
        assert!(input.pressed(DummyInput::Input1));
        assert!(input.pressed(DummyInput::Input2));

        // Check that their value was set to 1.0 when pressed
        assert!(input.value(DummyInput::Input1) == 1.0);
        assert!(input.value(DummyInput::Input2) == 1.0);

        // Clear the `input`, removing just pressed and just released
        input.clear();

        // Check if they're marked "just pressed"
        assert!(!input.just_pressed(DummyInput::Input1));
        assert!(!input.just_pressed(DummyInput::Input2));

        // Check if they're marked as pressed
        assert!(input.pressed(DummyInput::Input1));
        assert!(input.pressed(DummyInput::Input2));

        // Release the inputs and check state

        input.release(DummyInput::Input1);
        input.release(DummyInput::Input2);

        // Check if they're marked as "just released" (released on this update)
        assert!(input.just_released(DummyInput::Input1));
        assert!(input.just_released(DummyInput::Input2));

        // Check that they're not incorrectly marked as pressed
        assert!(!input.pressed(DummyInput::Input1));
        assert!(!input.pressed(DummyInput::Input2));

        // Check that their value was set to 0.0 when released
        assert!(input.value(DummyInput::Input1) == 0.0);
        assert!(input.value(DummyInput::Input2) == 0.0);

        // Clear the `Input` and check for removal from `just_released`
        input.clear();

        // Check that they're not incorrectly marked as just released
        assert!(!input.just_released(DummyInput::Input1));
        assert!(!input.just_released(DummyInput::Input2));

        // Set up an `Input` to test resetting.
        let mut input = Input::default();

        input.press(DummyInput::Input1);
        input.release(DummyInput::Input2);

        // Reset the `Input` and test it was reset correctly.
        input.reset(DummyInput::Input1);
        input.reset(DummyInput::Input2);

        assert!(!input.just_pressed(DummyInput::Input1));
        assert!(!input.pressed(DummyInput::Input1));

        assert!(!input.just_released(DummyInput::Input2));

        // Manually set a value to an intermediate valid value
        input.set_value(DummyInput::Input1, 0.42);
        assert!(input.value(DummyInput::Input1) == 0.42);

        // Manually set to values that are outside of the valid range
        input.set_value(DummyInput::Input1, -1.0);
        assert!(input.value(DummyInput::Input1) == 0.0);

        input.set_value(DummyInput::Input2, 2.7);
        assert!(input.value(DummyInput::Input2) == 1.0);
    }
}
