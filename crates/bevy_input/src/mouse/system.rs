use crate::{
    mouse::{MouseButton, MouseButtonInput},
    ButtonState, Input,
};
use bevy_ecs::event::EventReader;
use bevy_ecs::system::ResMut;

/// Updates the [`Input<MouseButton>`] resource with the latest [`MouseButtonInput`] events.
///
/// ## Differences
///
/// The main difference between the [`MouseButtonInput`] event and the [`Input<MouseButton>`] resource is that
/// the latter has convenient functions like [`Input::pressed`], [`Input::just_pressed`] and [`Input::just_released`].
pub fn mouse_button_input_system(
    mut mouse_button_input: ResMut<Input<MouseButton>>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
) {
    mouse_button_input.clear();
    for event in mouse_button_input_events.iter() {
        match event.state {
            ButtonState::Pressed => mouse_button_input.press(event.button),
            ButtonState::Released => mouse_button_input.release(event.button),
        }
    }
}
