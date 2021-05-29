use bevy::{
    app::AppExit,
    openxr::{OpenXrPlugin, OCULUS_TOUCH_PROFILE},
    prelude::*,
    utils::Duration,
    xr::{
        XrActionDescriptor, XrActionSet, XrActionType, XrHandType, XrPlugin, XrProfileDescriptor,
        XrReferenceSpaceType, XrSessionMode, XrSystem, XrTrackingSource, XrVibrationEvent,
        XrVibrationEventType,
    },
    DefaultPlugins, PipelinedDefaultPlugins,
};

#[bevy_main]
fn main() {
    App::new()
        .add_plugin(XrPlugin)
        .add_plugin(OpenXrPlugin)
        .add_plugins(PipelinedDefaultPlugins)
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

    let left_button = XrActionDescriptor {
        name: "left_button".into(),
        action_type: XrActionType::Button { touch: false },
    };
    let right_button = XrActionDescriptor {
        name: "right_button".into(),
        action_type: XrActionType::Button { touch: false },
    };
    let left_squeeze = XrActionDescriptor {
        name: "left_squeeze".into(),
        action_type: XrActionType::Scalar,
    };
    let right_button = XrActionDescriptor {
        name: "right_squeeze".into(),
        action_type: XrActionType::Scalar,
    };

    let oculus_profile = XrProfileDescriptor {
        profile: OCULUS_TOUCH_PROFILE.into(),
        bindings: vec![
            (left_button.clone(), "/user/hand/left/input/trigger".into()),
            (left_button, "/user/hand/left/input/x".into()),
            (
                right_button.clone(),
                "/user/hand/right/input/trigger".into(),
            ),
            (right_button, "/user/hand/right/input/a".into()),
            (left_squeeze, "/user/hand/left/input/squeeze".into()),
            (right_squeeze, "/user/hand/right/input/squeeze".into()),
        ],
        tracked: true,
        has_haptics: true,
    };

    xr_system.set_action_set(vec![oculus_profile]);
}

fn interaction(
    action_set: Res<XrActionSet>,
    mut tracking_source: ResMut<XrTrackingSource>,
    mut vibration_events: EventWriter<XrVibrationEvent>,
) {
    if tracking_source.reference_space_type() != XrReferenceSpaceType::Local {
        tracking_source.set_reference_space_type(XrReferenceSpaceType::Local);
    }

    for (hand, button, squeeze) in [
        (
            XrHandType::Left,
            "left_button".to_owned(),
            "left_squeeze".to_owned(),
        ),
        (
            XrHandType::Right,
            "right_button".to_owned(),
            "right_squeeze".to_owned(),
        ),
    ] {
        if action_set.button_just_pressed(button) {
            // Short haptic click
            vibration_events.send(XrVibrationEvent {
                hand,
                command: XrVibrationEventType::Apply {
                    duration: Duration::from_millis(2),
                    frequency: 3000_f32, // Hz
                    amplitude: 1_f32,
                },
            });
        } else {
            let squeeze_value = action_set.scalar_value(squeeze);
            if squeeze_value > 0.0 {
                // Low frequency rumble
                vibration_events.send(XrVibrationEvent {
                    hand,
                    command: XrVibrationEventType::Apply {
                        duration: Duration::from_millis(100),
                        frequency: 100_f32, // Hz
                        // haptics intensity depends on the squeeze force
                        amplitude: squeeze_value,
                    },
                });
            }
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
