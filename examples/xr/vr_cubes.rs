use bevy::{
    app::AppExit,
    prelude::*,
    utils::Duration,
    xr::{
        XrButtonType, XrButtons, XrHandType, XrReferenceSpaceType, XrSessionMode, XrSystem,
        XrTrackingSource, XrVibrationEvent, XrVibrationEventType,
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
    if xr_system.is_session_mode_supported(XrSessionMode::ImmersiveVR) {
        xr_system.request_session_mode(XrSessionMode::ImmersiveVR);
    } else {
        bevy::log::error!("The XR device does not support immersive VR mode");
        app_exit_events.send(AppExit)
    }
}

fn interaction(
    buttons: Res<XrButtons>,
    mut tracking_source: ResMut<XrTrackingSource>,
    mut vibration_events: EventWriter<XrVibrationEvent>,
) {
    if tracking_source.reference_space_type() != XrReferenceSpaceType::Local {
        tracking_source.set_reference_space_type(XrReferenceSpaceType::Local);
    }

    for hand in [XrHandType::Left, XrHandType::Right] {
        if buttons.just_pressed(hand, XrButtonType::Trigger) {
            // Short haptic click
            vibration_events.send(XrVibrationEvent {
                hand,
                command: XrVibrationEventType::Apply {
                    duration: Duration::from_millis(2),
                    frequency: 3000_f32, // Hz
                    amplitude: 1_f32,
                },
            });
        } else if buttons.pressed(hand, XrButtonType::Squeeze) {
            // Low frequency rumble
            vibration_events.send(XrVibrationEvent {
                hand,
                command: XrVibrationEventType::Apply {
                    duration: Duration::from_millis(100),
                    frequency: 100_f32, // Hz
                    // haptics intensity depends on the squeeze force
                    amplitude: buttons.value(hand, XrButtonType::Squeeze),
                },
            });
        }
    }

    let [left_pose, right_pose] = tracking_source.hands_pose();
    if let Some(pose) = left_pose {
        let left_pose = pose.to_mat4();
    }
    if let Some(pose) = right_pose {
        let right_pose = pose.to_mat4();
    }

    todo!() // Draw hands
}
