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
            BinaryEventType, GenericControllerPairButtons, GenericControllerVibration, HandType,
            TrackingReferenceMode, Vibration,
        },
        BlendMode, ViewerType, XrConfig, XrDuration, XrMode, XrState,
    },
    DefaultPlugins,
};

const INDEX_LEFT_FORCE: &str = "index_left_force";
const INDEX_LEFT_FORCE_PATH: &str = "/user/hand/left/input/squeeze/force";
const INDEX_RIGHT_FORCE: &str = "index_right_force";
const INDEX_RIGHT_FORCE_PATH: &str = "/user/hand/right/input/squeeze/force";

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
                    name: INDEX_LEFT_FORCE,
                    ids: vec![OpenXrActionPath {
                        profile: VALVE_INDEX_PROFILE,
                        path: INDEX_LEFT_FORCE_PATH,
                    }],
                    action_type: OpenXrActionType::FloatInput,
                },
                OpenXrBindingDesc {
                    name: INDEX_RIGHT_FORCE,
                    ids: vec![OpenXrActionPath {
                        profile: VALVE_INDEX_PROFILE,
                        path: INDEX_RIGHT_FORCE_PATH,
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
) {
    for e in controller_button_events.iter() {
        let hand = if e.left_hand.primary_click.value
            && matches!(e.left_hand.primary_click.event, BinaryEventType::Toggled)
        {
            HandType::Left
        } else if e.left_hand.primary_click.value
            && matches!(e.left_hand.primary_click.event, BinaryEventType::Toggled)
        {
            HandType::Right
        } else {
            continue;
        };

        // Short haptic click
        vibration_events.send(GenericControllerVibration {
            hand,
            action: Vibration::Apply {
                duration: XrDuration::from_nanos(2_000_000), // 2ms
                frequency: 3000_f32,                         //Hz
                amplitude: 1_f32,
            },
        });
    }

    for e in float_events.iter() {
        let hand = if e.name == INDEX_LEFT_FORCE {
            HandType::Left
        } else if e.name == INDEX_RIGHT_FORCE {
            HandType::Right
        } else {
            continue;
        };

        // Low Frequency rumble
        vibration_events.send(GenericControllerVibration {
            hand,
            action: Vibration::Apply {
                duration: XrDuration::from_nanos(100_000_000), // 100ms
                frequency: 100_f32,                            //Hz
                amplitude: e.value, // haptics intensity depends on the squeeze force
            },
        });
    }
}
