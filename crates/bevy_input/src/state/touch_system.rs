use crate::events::TouchEvent;
use bevy_app::EventReader;

#[derive(Default)]
pub struct TouchSystemState {
    touch_event_reader: EventReader<TouchEvent>,
}
