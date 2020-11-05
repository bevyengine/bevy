use super::*;
use bevy_app::Events;
use bevy_ecs::{Local, Res, ResMut};
use std::ops::DerefMut;

/// Updates the Touches resource with the latest TouchInput events
pub fn touch_screen_input_system(
    mut state: Local<TouchSystemState>,
    mut touch_state: ResMut<Touches>,
    touch_input_events: Res<Events<TouchEvent>>,
) {
    let touch_state = touch_state.deref_mut();
    touch_state.just_pressed.clear();
    touch_state.just_released.clear();
    for event in state.touch_event_reader.iter(&touch_input_events) {
        match event.phase {
            TouchPhaseCode::Started => {
                touch_state.pressed.insert(event.id, event.into());
                touch_state.just_pressed.insert(event.id, event.into());
            }
            TouchPhaseCode::Moved => {
                let mut new_touch = touch_state.pressed.get(&event.id).cloned().unwrap();
                new_touch.previous_position = new_touch.position;
                new_touch.previous_force = new_touch.force;
                new_touch.position = event.position;
                new_touch.force = event.force;
                touch_state.pressed.insert(event.id, new_touch);
            }
            TouchPhaseCode::Ended => {
                touch_state.just_released.insert(event.id, event.into());
                touch_state.pressed.remove_entry(&event.id);
            }
            TouchPhaseCode::Cancelled => {
                touch_state.just_cancelled.insert(event.id, event.into());
                touch_state.pressed.remove_entry(&event.id);
            }
        };
    }
}
