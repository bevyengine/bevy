use bevy_utils::HashSet;
use std::hash::Hash;

/// A "press-able" input of type `T`
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
    pub fn press(&mut self, input: T) {
        if !self.pressed(input) {
            self.just_pressed.insert(input);
        }

        self.pressed.insert(input);
    }

    pub fn pressed(&self, input: T) -> bool {
        self.pressed.contains(&input)
    }

    pub fn release(&mut self, input: T) {
        self.pressed.remove(&input);
        self.just_released.insert(input);
    }

    pub fn just_pressed(&self, input: T) -> bool {
        self.just_pressed.contains(&input)
    }

    pub fn just_released(&self, input: T) -> bool {
        self.just_released.contains(&input)
    }

    pub fn reset(&mut self, input: T) {
        self.pressed.remove(&input);
        self.just_pressed.remove(&input);
        self.just_released.remove(&input);
    }

    pub fn update(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    pub fn get_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.pressed.iter()
    }

    pub fn get_just_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_pressed.iter()
    }

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

        // Update the `Input` and check press state
        input.update();

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

        // Update the `Input` and check for removal from `just_released`

        input.update();

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
