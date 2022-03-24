use crate::touch::{TouchInput, Touches};
use bevy_ecs::{event::EventReader, system::ResMut};

/// Updates the `Touches` resource with the latest `TouchInput` events
pub fn touch_screen_input_system(
    mut touch_state: ResMut<Touches>,
    mut touch_input_events: EventReader<TouchInput>,
) {
    touch_state.update();

    for event in touch_input_events.iter() {
        touch_state.process_touch_event(event);
    }
}
