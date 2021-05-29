mod tracking;

use bevy_math::Vec2;
pub use tracking::*;

use crate::{conversion::from_duration, OpenXrSession};
use bevy_app::{Events, ManualEventReader};
use bevy_xr::{
    XrActionSet, XrActionState, XrActionType, XrButtonState, XrHandType, XrProfileDescriptor,
    XrVibrationEvent, XrVibrationEventType,
};
use openxr as xr;
use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc};

// Profiles
pub const KHR_PROFILE: &str = "/interaction_profiles/khr/simple_controller";
pub const DAYDREAM_PROFILE: &str = "/interaction_profiles/google/daydream_controller";
pub const VIVE_PROFILE: &str = "/interaction_profiles/htc/vive_controller";
pub const VIVE_PRO_PROFILE: &str = "/interaction_profiles/htc/vive_pro";
pub const WMR_PROFILE: &str = "/interaction_profiles/microsoft/motion_controller";
pub const XBOX_PROFILE: &str = "/interaction_profiles/microsoft/xbox_controller";
pub const GO_PROFILE: &str = "/interaction_profiles/oculus/go_controller";
pub const OCULUS_TOUCH_PROFILE: &str = "/interaction_profiles/oculus/touch_controller";
pub const VALVE_INDEX_PROFILE: &str = "/interaction_profiles/valve/index_controller";

fn hand_str(hand_type: XrHandType) -> &'static str {
    match hand_type {
        XrHandType::Left => "left",
        XrHandType::Right => "right",
    }
}

struct ButtonActions {
    touch: xr::Action<bool>,
    click: xr::Action<bool>,
    value: xr::Action<f32>,
}

pub(crate) struct InteractionContext {
    // Every time `session.sync_action` is called, the result of `locate_space` can change. In case
    // of concurrent use, this becomes unpredictable. Use a Mutex on the `action_set` to allow
    // proper synchronization. (NB: synchronization is not ensured: the lock must be held until all
    // `locate_space` calls have been performed)
    pub action_set: Arc<Mutex<xr::ActionSet>>,
    button_actions: HashMap<String, ButtonActions>,
    binary_actions: HashMap<String, xr::Action<bool>>,
    scalar_actions: HashMap<String, xr::Action<f32>>,
    vec_2d_actions: HashMap<String, (xr::Action<f32>, xr::Action<f32>)>,
    grip_actions: HashMap<XrHandType, xr::Action<xr::Posef>>,
    target_ray_actions: HashMap<XrHandType, xr::Action<xr::Posef>>,
    vibration_actions: HashMap<XrHandType, xr::Action<xr::Haptic>>,
}

impl InteractionContext {
    pub fn new(instance: &xr::Instance, bindings: &[XrProfileDescriptor]) -> Self {
        let action_set = instance
            .create_action_set("bevy_bindings", "bevy bindings", 0)
            .unwrap();

        let mut button_actions = HashMap::new();
        for desc in bindings {
            for (action_desc, _) in &desc.bindings {
                if matches!(action_desc.action_type, XrActionType::Button { .. }) {
                    button_actions
                        .entry(action_desc.name.clone())
                        .or_insert_with(|| {
                            let touch_name = format!("{}_touch", action_desc.name);
                            let click_name = format!("{}_click", action_desc.name);
                            let value_name = format!("{}_value", action_desc.name);
                            ButtonActions {
                                touch: action_set
                                    .create_action(&touch_name, &touch_name, &[])
                                    .unwrap(),
                                click: action_set
                                    .create_action(&click_name, &click_name, &[])
                                    .unwrap(),
                                value: action_set
                                    .create_action(&value_name, &value_name, &[])
                                    .unwrap(),
                            }
                        });
                }
            }
        }

        let mut binary_actions = HashMap::new();
        for desc in bindings {
            for (action_desc, _) in &desc.bindings {
                if action_desc.action_type == XrActionType::Binary {
                    binary_actions
                        .entry(action_desc.name.clone())
                        .or_insert_with(|| {
                            action_set
                                .create_action(&action_desc.name, &action_desc.name, &[])
                                .unwrap()
                        });
                }
            }
        }

        let mut scalar_actions = HashMap::new();
        for desc in bindings {
            for (action_desc, _) in &desc.bindings {
                if action_desc.action_type == XrActionType::Scalar {
                    scalar_actions
                        .entry(action_desc.name.clone())
                        .or_insert_with(|| {
                            action_set
                                .create_action(&action_desc.name, &action_desc.name, &[])
                                .unwrap()
                        });
                }
            }
        }

        let mut vec_2d_actions = HashMap::new();
        for desc in bindings {
            for (action_desc, _) in &desc.bindings {
                if action_desc.action_type == XrActionType::Vec2D {
                    vec_2d_actions
                        .entry(action_desc.name.clone())
                        .or_insert_with(|| {
                            let name_x = format!("{}_x", action_desc.name);
                            let name_y = format!("{}_y", action_desc.name);
                            (
                                action_set.create_action(&name_x, &name_x, &[]).unwrap(),
                                action_set.create_action(&name_y, &name_y, &[]).unwrap(),
                            )
                        });
                }
            }
        }

        let grip_actions = [XrHandType::Left, XrHandType::Right]
            .iter()
            .map(|hand| {
                let name = format!("{}_grip", hand_str(*hand));
                (*hand, action_set.create_action(&name, &name, &[]).unwrap())
            })
            .collect::<HashMap<_, _>>();

        let target_ray_actions = [XrHandType::Left, XrHandType::Right]
            .iter()
            .map(|hand| {
                let name = format!("{}_target_ray", hand_str(*hand));
                (*hand, action_set.create_action(&name, &name, &[]).unwrap())
            })
            .collect::<HashMap<_, _>>();

        let vibration_actions = [XrHandType::Left, XrHandType::Right]
            .iter()
            .map(|hand| {
                let name = format!("{}_vibration", hand_str(*hand));
                (*hand, action_set.create_action(&name, &name, &[]).unwrap())
            })
            .collect::<HashMap<_, _>>();

        for desc in bindings {
            let mut bindings = vec![];

            for (action_desc, path_string) in &desc.bindings {
                let path = instance.string_to_path(path_string).unwrap();

                match action_desc.action_type {
                    XrActionType::Button { touch } => {
                        let actions = button_actions.get(&action_desc.name).unwrap();

                        if touch {
                            bindings.push(xr::Binding::new(
                                &actions.touch,
                                instance
                                    .string_to_path(&format!("{}/touch", path_string))
                                    .unwrap(),
                            ));
                        }

                        // Note: `click` and `value` components are inferred and automatically
                        // polyfilled by the runtimes. The runtime may use a 0/1 value using the
                        // click path or infer the click using the value path and a hysteresis
                        // threshold.
                        bindings.push(xr::Binding::new(&actions.click, path));
                        bindings.push(xr::Binding::new(&actions.value, path));
                    }
                    XrActionType::Binary => {
                        let action = binary_actions.get(&action_desc.name).unwrap();
                        bindings.push(xr::Binding::new(action, path))
                    }
                    XrActionType::Scalar => {
                        let action = scalar_actions.get(&action_desc.name).unwrap();
                        bindings.push(xr::Binding::new(action, path))
                    }
                    XrActionType::Vec2D => {
                        let (action_x, action_y) = vec_2d_actions.get(&action_desc.name).unwrap();

                        bindings.push(xr::Binding::new(
                            action_x,
                            instance
                                .string_to_path(&format!("{}/x", path_string))
                                .unwrap(),
                        ));
                        bindings.push(xr::Binding::new(
                            action_y,
                            instance
                                .string_to_path(&format!("{}/y", path_string))
                                .unwrap(),
                        ));
                    }
                }
            }

            if desc.tracked {
                for hand in [XrHandType::Left, XrHandType::Right] {
                    let path_prefix = format!("/user/hand/{}/input", hand_str(hand));

                    let action = grip_actions.get(&hand).unwrap();
                    let path = format!("{}/grip/pose", path_prefix);
                    bindings.push(xr::Binding::new(
                        action,
                        instance.string_to_path(&path).unwrap(),
                    ));

                    let action = target_ray_actions.get(&hand).unwrap();
                    let path = format!("{}/aim/pose", path_prefix);
                    bindings.push(xr::Binding::new(
                        action,
                        instance.string_to_path(&path).unwrap(),
                    ));
                }
            }

            if desc.has_haptics {
                for hand in [XrHandType::Left, XrHandType::Right] {
                    let action = vibration_actions.get(&hand).unwrap();
                    let path = format!("/user/hand/{}/output/haptic", hand_str(hand));
                    bindings.push(xr::Binding::new(
                        action,
                        instance.string_to_path(&path).unwrap(),
                    ));
                }
            }

            let profile_path = instance.string_to_path(&desc.profile).unwrap();

            // Ignore error for unsupported profiles.
            instance
                .suggest_interaction_profile_bindings(profile_path, &bindings)
                .ok();
        }

        InteractionContext {
            action_set: Arc::new(Mutex::new(action_set)),
            button_actions,
            binary_actions,
            scalar_actions,
            vec_2d_actions,
            grip_actions,
            target_ray_actions,
            vibration_actions,
        }
    }
}

pub(crate) fn handle_input(
    context: &InteractionContext,
    session: &OpenXrSession,
    action_set: &mut XrActionSet,
) {
    // NB: hold the lock
    let action_set_backend = &*context.action_set.lock();

    session.sync_actions(&[action_set_backend.into()]).unwrap();

    let mut states = HashMap::new();

    for (name, actions) in &context.button_actions {
        let touched = actions
            .touch
            .state(session, xr::Path::NULL)
            .unwrap()
            .current_state;
        let pressed = actions
            .click
            .state(session, xr::Path::NULL)
            .unwrap()
            .current_state;
        let value = actions
            .value
            .state(session, xr::Path::NULL)
            .unwrap()
            .current_state;

        let state = if pressed {
            XrButtonState::Pressed
        } else if touched {
            XrButtonState::Touched
        } else {
            XrButtonState::Default
        };

        states.insert(name.clone(), XrActionState::Button { state, value });
    }

    for (name, action) in &context.binary_actions {
        let value = action.state(session, xr::Path::NULL).unwrap().current_state;
        states.insert(name.clone(), XrActionState::Binary(value));
    }

    for (name, action) in &context.scalar_actions {
        let value = action.state(session, xr::Path::NULL).unwrap().current_state;
        states.insert(name.clone(), XrActionState::Scalar(value));
    }

    for (name, (action1, action2)) in &context.vec_2d_actions {
        let value1 = action1
            .state(session, xr::Path::NULL)
            .unwrap()
            .current_state;
        let value2 = action2
            .state(session, xr::Path::NULL)
            .unwrap()
            .current_state;
        states.insert(
            name.clone(),
            XrActionState::Vec2D(Vec2::new(value1, value2)),
        );
    }

    action_set.set(states);
}

pub(crate) fn handle_output(
    context: &InteractionContext,
    session: &OpenXrSession,
    vibration_event_reader: &mut ManualEventReader<XrVibrationEvent>,
    vibration_events: &mut Events<XrVibrationEvent>,
) {
    for event in vibration_event_reader.iter(vibration_events) {
        let action = context.vibration_actions.get(&event.hand);
        if let Some(action) = action {
            match &event.command {
                XrVibrationEventType::Apply {
                    duration,
                    frequency,
                    amplitude,
                } => {
                    let haptic_vibration = xr::HapticVibration::new()
                        .duration(from_duration(*duration))
                        .frequency(*frequency)
                        .amplitude(*amplitude);

                    action
                        .apply_feedback(session, xr::Path::NULL, &haptic_vibration)
                        .unwrap();
                }
                XrVibrationEventType::Stop => {
                    action.stop_feedback(session, xr::Path::NULL).unwrap()
                }
            }
        }
    }

    vibration_events.update();
}
