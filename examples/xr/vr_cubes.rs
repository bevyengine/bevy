use bevy::{
    app::AppExit,
    prelude::*,
    utils::Duration,
    xr::{
        HandType, VibrationEvent, VibrationEventType, XrButtonType, XrButtons,
        XrReferenceSpaceType, XrSessionMode, XrSystem, XrTrackingState,
    },
    DefaultPlugins,
};

#[bevy_main]
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(startup)
        .add_system(interaction)
        .run();
}

fn startup(mut xr_system: ResMut<XrSystem>, mut app_exit_events: EventWriter<AppExit>) {
    if xr_system
        .available_session_modes()
        .contains(XrSessionMode::ImmersiveVR)
    {
        xr_system.request_session_mode(XrSessionMode::ImmersiveVR)
    } else {
        bevy::log::error!("The XR device does not support immersive VR mode");
        app_exit_events.send(AppExit)
    }
}

fn interaction(
    mut tracking_state: ResMut<XrTrackingState>,
    buttons: Res<XrButtons>,
    mut vibration_events: EventWriter<VibrationEvent>,
    xr_state: Res<XrTrackingState>,
) {
    if !tracking_state.get_reference_space_type(XrReferenceSpaceType::Local) {
        tracking_state.set_reference_space_type(XrReferenceSpaceType::Local)
    }

    for hand in [HandType::Left, HandType::Right] {
        if buttons.just_pressed(hand, XrButtonType::Trigger) {
            // Short haptic click
            vibration_events.send(VibrationEvent {
                hand,
                command: VibrationEventType::Apply {
                    duration: Duration::from_millis(2),
                    frequency: 3000_f32, // Hz
                    amplitude: 1_f32,
                },
            });
        } else if buttons.pressed(hand, XrButtonType::Squeeze) {
            // Low frequency rumble
            vibration_events.send(VibrationEvent {
                hand,
                command: VibrationEventType::Apply {
                    duration: Duration::from_millis(100),
                    frequency: 100_f32, // Hz
                    // haptics intensity depends on the squeeze force
                    amplitude: buttons.value(hand, XrButtonType::Squeeze),
                },
            });
        }
    }

    if let Some(pose) = xr_state.hand_pose(HandType::Left) {
        let left_pose = pose.transform.to_mat4();
    }
    if let Some(pose) = xr_state.hand_pose(HandType::Right) {
        let right_pose = pose.transform.to_mat4();
    }

    todo!() // Draw hands
}
