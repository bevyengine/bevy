use crate::{
    mouse::{MouseButton, MouseButtonInput},
    ElementState, Input,
};
use bevy_app::EventReader;
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
            ElementState::Pressed => mouse_button_input.press(event.button),
            ElementState::Released => mouse_button_input.release(event.button),
        }
    }
}
