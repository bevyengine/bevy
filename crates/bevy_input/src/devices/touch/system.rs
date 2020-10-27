use bevy_input::devices::touch::{TouchSystemState, Touches, TouchInput, TouchPhase, Touch};

/// Updates the Touches resource with the latest TouchInput events
pub fn touch_screen_input_system(
    mut state: Local<TouchSystemState>,
    mut touch_state: ResMut<Touches>,
    touch_input_events: Res<Events<TouchInput>>,
) {
    let touch_state = &mut *touch_state;
    for released_id in touch_state.just_released.iter() {
        touch_state.active_touches.remove(&released_id);
    }

    for cancelled_id in touch_state.just_cancelled.iter() {
        touch_state.active_touches.remove(&cancelled_id);
    }

    touch_state.just_pressed.clear();
    touch_state.just_cancelled.clear();

    for event in state.touch_event_reader.iter(&touch_input_events) {
        let active_touch = touch_state.active_touches.get(&event.id);
        match event.phase {
            TouchPhase::Started => {
                touch_state.active_touches.insert(
                    event.id,
                    Touch {
                        id: event.id,
                        start_position: event.position,
                        previous_position: event.position,
                        position: event.position,
                    },
                );
                touch_state.just_pressed.insert(event.id);
            }
            TouchPhase::Moved => {
                let old_touch = active_touch.unwrap();
                let mut new_touch = old_touch.clone();
                new_touch.previous_position = new_touch.position;
                new_touch.position = event.position;
                touch_state.active_touches.insert(event.id, new_touch);
            }
            TouchPhase::Ended => {
                touch_state.just_released.insert(event.id);
            }
            TouchPhase::Cancelled => {
                touch_state.just_cancelled.insert(event.id);
            }
        };
    }
}

#[derive(Default)]
pub struct TouchSystemState {
    touch_event_reader: EventReader<TouchInput>,
}

