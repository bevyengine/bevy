use crate::touch::{TouchInput, Touches};
use bevy_ecs::event::EventReader;
use bevy_ecs::system::ResMut;

/// Updates the [`Touches`] resource with the latest [`TouchInput`] events.
///
/// ## Differences
///
/// The main difference between the [`TouchInput`] event and the [`Touches`] resource is that
/// the latter has convenient functions like [`Touches::just_pressed`] and [`Touches::just_released`].
pub fn touch_screen_input_system(
    mut touch_state: ResMut<Touches>,
    mut touch_input_events: EventReader<TouchInput>,
) {
    touch_state.update();

    for event in touch_input_events.iter() {
        touch_state.process_touch_event(event);
    }
}
