use super::*;
use bevy_app::EventReader;

#[derive(Default)]
pub struct TouchSystemState {
    pub(crate) touch_event_reader: EventReader<TouchEvent>,
}
