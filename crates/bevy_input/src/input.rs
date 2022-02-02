use bevy_utils::HashSet;
use std::hash::Hash;

// unused import, but needed for intra doc link to work
#[allow(unused_imports)]
use bevy_ecs::schedule::State;

/// A "press-able" input of type `T`.
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
pub struct Input<T> {
    pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    just_released: HashSet<T>,
}

impl<T> Default for Input<T> {
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
    T: Copy + Eq + Hash,
{
    /// Register a press for input `input`.
    pub fn press(&mut self, input: T) {
        if !self.pressed(input) {
            self.just_pressed.insert(input);
        }

        self.pressed.insert(input);
    }

    /// Check if `input` has been pressed.
    pub fn pressed(&self, input: T) -> bool {
        self.pressed.contains(&input)
    }

    /// Check if any item in `inputs` has been pressed.
    pub fn any_pressed(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.pressed(it))
    }

    /// Register a release for input `input`.
    pub fn release(&mut self, input: T) {
        self.pressed.remove(&input);
        self.just_released.insert(input);
    }

    /// Check if `input` has been just pressed.
    pub fn just_pressed(&self, input: T) -> bool {
        self.just_pressed.contains(&input)
    }

    /// Check if any item in `inputs` has just been pressed.
    pub fn any_just_pressed(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.just_pressed(it))
    }

    /// Clear the "just pressed" state of `input`. Future calls to [`Input::just_pressed`] for the
    /// given input will return false until a new press event occurs.
    /// Returns true if `input` is currently "just pressed"
    pub fn clear_just_pressed(&mut self, input: T) -> bool {
        self.just_pressed.remove(&input)
    }

    /// Check if `input` has been just released.
    pub fn just_released(&self, input: T) -> bool {
        self.just_released.contains(&input)
    }

    /// Check if any item in `inputs` has just been released.
    pub fn any_just_released(&self, inputs: impl IntoIterator<Item = T>) -> bool {
        inputs.into_iter().any(|it| self.just_released(it))
    }

    /// Clear the "just released" state of `input`. Future calls to [`Input::just_released`] for the
    /// given input will return false until a new release event occurs.
    /// Returns true if `input` is currently "just released"
    pub fn clear_just_released(&mut self, input: T) -> bool {
        self.just_released.remove(&input)
    }

    /// Reset all status for input `input`.
    pub fn reset(&mut self, input: T) {
        self.pressed.remove(&input);
        self.just_pressed.remove(&input);
        self.just_released.remove(&input);
    }

    /// Clear just pressed and just released information.
    pub fn clear(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// List all inputs that are pressed.
    pub fn get_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.pressed.iter()
    }

    /// List all inputs that are just pressed.
    pub fn get_just_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_pressed.iter()
    }

    /// List all inputs that are just released.
    pub fn get_just_released(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_released.iter()
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn input_test() {
        use crate::Input;

        /// Used for testing `Input` functionality
        #[derive(Copy, Clone, Eq, PartialEq, Hash)]
        enum DummyInput {
            Input1,
            Input2,
        }

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
    }
}
