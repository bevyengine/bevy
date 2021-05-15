use bevy_app::{EventWriter, Events};
use bevy_ecs::system::ResMut;

use crate::{
    event::{XREvent, XRState, XRViewCreated},
    hand_tracking::HandPoseState,
    XRDevice,
};

pub(crate) fn openxr_event_system(
    mut openxr: ResMut<XRDevice>,
    mut hand_pose: ResMut<HandPoseState>,
    mut state_events: ResMut<Events<XRState>>,
    mut view_created_sender: EventWriter<XRViewCreated>,
) {
    // TODO add this drain -system as pre-render and post-render system?
    for event in openxr.drain_events() {
        println!("Drained openxr event {:?}", event);
        match event {
            XREvent::ViewCreated(view_created) => view_created_sender.send(view_created),
        }
    }

    // This should be before all other events
    match openxr.inner.handle_openxr_events() {
        None => (),
        Some(changed_state) => {
            // FIXME handle XRState::Exiting
            state_events.send(changed_state);
        }
    }

    // FIXME: this should happen just before bevy render graph and / or wgpu render?
    openxr.touch_update();

    // FIXME this should be in before-other-systems system? so that all systems can use hand pose data...
    if let Some(hp) = openxr.get_hand_positions() {
        *hand_pose = hp;
    }
}
