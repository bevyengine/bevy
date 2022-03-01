use crate::{
    keyboard::{KeyCode, KeyboardInput},
    ButtonState, Input,
};
use bevy_ecs::event::EventReader;
use bevy_ecs::system::ResMut;

/// Updates the [`Input<KeyCode>`] resource with the latest [`KeyboardInput`] events.
///
/// ## Differences
///
/// The main difference between the [`KeyboardInput`] event and the [`Input<KeyCode>`] resource is that
/// the latter has convenient functions like [`Input::pressed`], [`Input::just_pressed`] and [`Input::just_released`].
pub fn keyboard_input_system(
    mut keyboard_input: ResMut<Input<KeyCode>>,
    mut keyboard_input_events: EventReader<KeyboardInput>,
) {
    keyboard_input.clear();
    for event in keyboard_input_events.iter() {
        if let Some(key_code) = event.key_code {
            match event.state {
                ButtonState::Pressed => keyboard_input.press(key_code),
                ButtonState::Released => keyboard_input.release(key_code),
            }
        }
    }
}
