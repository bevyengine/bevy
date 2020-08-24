use super::keyboard::ElementState;
use crate::Input;
use bevy_app::prelude::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_math::Vec2;


/// A finger on a touch screen device
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Finger(pub u64);

/// A finger pressed event
#[derive(Debug, Clone)]
pub struct TouchFingerInput {
    pub finger: Finger,
    pub state: ElementState,
    pub position: Vec2
}


/// A finer motion event
#[derive(Debug, Clone)]
pub struct TouchMotion {
    pub finger: Finger,
    pub position: Vec2
}


/// State used by the mouse button input system
#[derive(Default)]
pub struct TouchFingerInputState {
    touch_finger_input_event_reader: EventReader<TouchFingerInput>,
}

/// Updates the Input<Finger> resource with the latest TouchFingerInput events
pub fn touch_finger_input_system(
    mut state: Local<TouchFingerInputState>,
    mut touch_finger_input: ResMut<Input<Finger>>,
    touch_finger_input_events: Res<Events<TouchFingerInput>>,
) {
    touch_finger_input.update();
    for event in state
        .touch_finger_input_event_reader
        .iter(&touch_finger_input_events)
    {
        match event.state {
            ElementState::Pressed => touch_finger_input.press(event.finger),
            ElementState::Released => touch_finger_input.release(event.finger),
        }
    }
}
