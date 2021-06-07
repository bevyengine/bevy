use bevy::{
    openxr::{
        interaction::{
            OpenXrActionPath, OpenXrActionType, OpenXrBindingDesc, OpenXrVendorInput,
            VALVE_INDEX_PROFILE,
        },
        OpenXrConfig,
    },
    prelude::*,
    xr::{
        interaction::{
            GenericControllerPairButtons, GenericControllerVibration, HandAction, HandType,
            TrackingReferenceMode, Vibration,
        },
        BlendMode, ViewerType, XrConfig, XrDuration, XrMode, XrState,
    },
    DefaultPlugins,
};

const LEFT_SQUEEZE_FORCE: &str = "left_squeeze_force";
const LEFT_SQUEEZE_FORCE_PATH: &str = "/user/hand/left/input/squeeze/force";
const RIGHT_SQUEEZE_FORCE: &str = "right_squeeze_force";
const RIGHT_SQUEEZE_FORCE_PATH: &str = "/user/hand/right/input/squeeze/force";

fn main() {
    App::build()
        // XrConfig is mandatory
        .insert_resource(XrConfig {
            mode: XrMode::Display {
                viewer: ViewerType::PreferHeadMounted,
                blend: BlendMode::PreferVR,
            },
            enable_generic_controllers: true,
        })
        // OpenXrConfig is optional
        .insert_resource(OpenXrConfig {
            vendor_bindings: vec![
                OpenXrBindingDesc {
                    name: LEFT_SQUEEZE_FORCE,
                    paths: vec![OpenXrActionPath {
                        profile: VALVE_INDEX_PROFILE,
                        path: LEFT_SQUEEZE_FORCE_PATH,
                    }],
                    action_type: OpenXrActionType::FloatInput,
                },
                OpenXrBindingDesc {
                    name: RIGHT_SQUEEZE_FORCE,
                    paths: vec![OpenXrActionPath {
                        profile: VALVE_INDEX_PROFILE,
                        path: RIGHT_SQUEEZE_FORCE_PATH,
                    }],
                    action_type: OpenXrActionType::FloatInput,
                },
            ],
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(startup.system())
        .add_system(interaction.system())
        .run();
}

fn startup(xr_state: Res<XrState>) {
    // This is the default
    xr_state.set_tracking_reference_mode(TrackingReferenceMode::GravityAligned);
}

fn interaction(
    mut controller_button_events: EventReader<GenericControllerPairButtons>,
    mut float_events: EventReader<OpenXrVendorInput<f32>>,
    mut vibration_events: EventWriter<GenericControllerVibration>,
    xr_state: Res<XrState>,
) {
    for e in controller_button_events.iter() {
        let hand = if e.left_hand.primary_click.value && e.left_hand.primary_click.toggled {
            HandType::Left
        } else if e.right_hand.primary_click.value && e.right_hand.primary_click.toggled {
            HandType::Right
        } else {
            continue;
        };

        // Short haptic click
        vibration_events.send(GenericControllerVibration {
            hand,
            action: Vibration::Apply {
                duration: XrDuration::from_nanos(2_000_000), // 2ms
                frequency: 3000_f32,                         // Hz
                amplitude: 1_f32,
            },
        });
    }

    for e in float_events.iter() {
        let hand = if e.name == LEFT_SQUEEZE_FORCE {
            HandType::Left
        } else if e.name == RIGHT_SQUEEZE_FORCE {
            HandType::Right
        } else {
            continue;
        };

        // Low frequency rumble
        vibration_events.send(GenericControllerVibration {
            hand,
            action: Vibration::Apply {
                duration: XrDuration::from_nanos(100_000_000), // 100ms
                frequency: 100_f32,                            // Hz
                amplitude: e.value, // haptics intensity depends on the squeeze force
            },
        });
    }

    let left_pose = xr_state
        .hand_motion(HandType::Left, HandAction::Grip)
        .pose
        .to_mat4();
    let right_pose = xr_state
        .hand_motion(HandType::Right, HandAction::Grip)
        .pose
        .to_mat4();

    todo!() // Draw hands
}
