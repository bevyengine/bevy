mod bindings;
mod tracking;

pub use bindings::*;
pub use tracking::*;

use crate::{conversion::from_duration, OpenXrSession};
use bevy_app::{Events, ManualEventReader};
use bevy_utils::HashMap;
use bevy_xr::{
    HandType, VibrationEvent, VibrationEventType, XrAxes, XrAxisType, XrButtonState, XrButtonType,
    XrButtons,
};
use openxr as xr;

fn hand_str(hand_type: HandType) -> &'static str {
    match hand_type {
        HandType::Left => "left",
        HandType::Right => "right",
    }
}

fn button_str(button_type: XrButtonType) -> &'static str {
    match button_type {
        XrButtonType::Menu => "menu",
        XrButtonType::Trigger => "trigger",
        XrButtonType::Squeeze => "squeeze",
        XrButtonType::Touchpad => "touchpad",
        XrButtonType::Thumbstick => "thumbstick",
        XrButtonType::FaceButton1 => "a",
        XrButtonType::FaceButton2 => "b",
        XrButtonType::Thumbrest => "thumbrest",
    }
}

fn axis_identifier(axis_type: XrAxisType) -> &'static str {
    match axis_type {
        XrAxisType::TouchpadX | XrAxisType::TouchpadY => "touchpad",
        XrAxisType::ThumbstickX | XrAxisType::ThumbstickY => "thumbstick",
    }
}

fn axis_component(axis_type: XrAxisType) -> &'static str {
    match axis_type {
        XrAxisType::TouchpadX | XrAxisType::ThumbstickX => "x",
        XrAxisType::TouchpadY | XrAxisType::ThumbstickY => "y",
    }
}

struct ButtonActions {
    touch: xr::Action<bool>,
    click: xr::Action<bool>,
    value: xr::Action<f32>,
}

pub(crate) struct InteractionContext {
    pub action_set: xr::ActionSet,
    button_actions: HashMap<(HandType, XrButtonType), ButtonActions>,
    axes_actions: HashMap<(HandType, XrAxisType), xr::Action<f32>>,
    grip_actions: HashMap<HandType, xr::Action<xr::Posef>>,
    target_ray_actions: HashMap<HandType, xr::Action<xr::Posef>>,
    vibration_actions: HashMap<HandType, xr::Action<xr::Haptic>>,
}

impl InteractionContext {
    pub fn new(instance: &xr::Instance, bindings: OpenXrBindings) -> Self {
        let action_set = instance
            .create_action_set("bevy_controller_bindings", "bevy controller bindings", 0)
            .unwrap();

        let button_actions = [HandType::Left, HandType::Right]
            .iter()
            .flat_map(|hand| {
                let hand_str = hand_str(*hand);
                [
                    XrButtonType::Menu,
                    XrButtonType::Trigger,
                    XrButtonType::Squeeze,
                    XrButtonType::Touchpad,
                    XrButtonType::Thumbstick,
                    XrButtonType::FaceButton1,
                    XrButtonType::FaceButton2,
                    XrButtonType::Thumbrest,
                ]
                .iter()
                .map({
                    let action_set = action_set.clone();
                    move |button| {
                        let button_str = button_str(*button);
                        let touch_name = format!("{}_{}_touch", hand_str, button_str);
                        let click_name = format!("{}_{}_click", hand_str, button_str);
                        let value_name = format!("{}_{}_value", hand_str, button_str);
                        let actions = ButtonActions {
                            touch: action_set
                                .create_action(&touch_name, &touch_name, &[])
                                .unwrap(),
                            click: action_set
                                .create_action(&click_name, &click_name, &[])
                                .unwrap(),
                            value: action_set
                                .create_action(&value_name, &value_name, &[])
                                .unwrap(),
                        };
                        ((*hand, *button), actions)
                    }
                })
            })
            .collect::<HashMap<_, _>>();

        let axes_actions = [HandType::Left, HandType::Right]
            .iter()
            .flat_map(|hand| {
                let hand_str = hand_str(*hand);
                [
                    XrAxisType::TouchpadX,
                    XrAxisType::TouchpadY,
                    XrAxisType::ThumbstickX,
                    XrAxisType::ThumbstickY,
                ]
                .iter()
                .map({
                    let action_set = action_set.clone();
                    move |axis| {
                        let name = format!(
                            "{}_{}_{}",
                            hand_str,
                            axis_identifier(*axis),
                            axis_component(*axis)
                        );
                        let action = action_set.create_action(&name, &name, &[]).unwrap();
                        ((*hand, *axis), action)
                    }
                })
            })
            .collect::<HashMap<_, _>>();

        let grip_actions = [HandType::Left, HandType::Right]
            .iter()
            .map(|hand| {
                let name = format!("{}_grip", hand_str(*hand));
                (*hand, action_set.create_action(&name, &name, &[]).unwrap())
            })
            .collect::<HashMap<_, _>>();

        let target_ray_actions = [HandType::Left, HandType::Right]
            .iter()
            .map(|hand| {
                let name = format!("{}_target_ray", hand_str(*hand));
                (*hand, action_set.create_action(&name, &name, &[]).unwrap())
            })
            .collect::<HashMap<_, _>>();

        let vibration_actions = [HandType::Left, HandType::Right]
            .iter()
            .map(|hand| {
                let name = format!("{}_vibration", hand_str(*hand));
                (*hand, action_set.create_action(&name, &name, &[]).unwrap())
            })
            .collect::<HashMap<_, _>>();

        for profile in bindings.profiles {
            let mut bindings = vec![];

            for ((hand, button), paths) in profile.buttons {
                let actions = button_actions.get(&(hand, button)).unwrap();
                match paths {
                    ButtonPaths::Default { has_touch } => {
                        let path_prefix =
                            format!("/user/hand/{}/input/{}", hand_str(hand), button_str(button));

                        if has_touch {
                            bindings.push(xr::Binding::new(
                                &actions.touch,
                                instance
                                    .string_to_path(&format!("{}/touch", path_prefix))
                                    .unwrap(),
                            ));
                        }

                        // Note: `click` and `value` components are inferred and automatically
                        // polyfilled by the runtimes. The runtime may use a 0/1 value using the
                        // click path or infer the click using the value path and a hysteresis
                        // threshold.
                        bindings.push(xr::Binding::new(
                            &actions.click,
                            instance.string_to_path(&path_prefix).unwrap(),
                        ));
                        bindings.push(xr::Binding::new(
                            &actions.value,
                            instance.string_to_path(&path_prefix).unwrap(),
                        ));
                    }
                    ButtonPaths::Custom {
                        touch,
                        click,
                        value,
                    } => {
                        for path in touch {
                            bindings.push(xr::Binding::new(
                                &actions.touch,
                                instance.string_to_path(&path).unwrap(),
                            ));
                        }
                        for path in click {
                            bindings.push(xr::Binding::new(
                                &actions.click,
                                instance.string_to_path(&path).unwrap(),
                            ));
                        }
                        for path in value {
                            bindings.push(xr::Binding::new(
                                &actions.value,
                                instance.string_to_path(&path).unwrap(),
                            ));
                        }
                    }
                }
            }

            match profile.axes {
                AxesBindings::Default {
                    touchpad,
                    thumbstick,
                } => {
                    for hand in [HandType::Left, HandType::Right] {
                        let mut axes = vec![];
                        if touchpad {
                            axes.extend([XrAxisType::TouchpadX, XrAxisType::TouchpadY]);
                        }
                        if thumbstick {
                            axes.extend([XrAxisType::ThumbstickX, XrAxisType::ThumbstickY]);
                        }
                        for axis in axes {
                            let action = axes_actions.get(&(hand, axis)).unwrap();
                            let path = format!(
                                "/user/hand/{}/input/{}/{}",
                                hand_str(hand),
                                axis_identifier(axis),
                                axis_component(axis)
                            );
                            bindings.push(xr::Binding::new(
                                action,
                                instance.string_to_path(&path).unwrap(),
                            ));
                        }
                    }
                }
                AxesBindings::Custom(paths) => {
                    for ((hand, axis), paths) in paths {
                        let action = axes_actions.get(&(hand, axis)).unwrap();
                        for path in paths {
                            bindings.push(xr::Binding::new(
                                action,
                                instance.string_to_path(&path).unwrap(),
                            ));
                        }
                    }
                }
            }

            match profile.poses {
                PosesBindings::None => (),
                PosesBindings::Default => {
                    for hand in [HandType::Left, HandType::Right] {
                        let path_prefix = format!("/user/hand/{}/input/", hand_str(hand));

                        let action = grip_actions.get(&hand).unwrap();
                        let path = format!("{}grip/pose", path_prefix);
                        bindings.push(xr::Binding::new(
                            action,
                            instance.string_to_path(&path).unwrap(),
                        ));

                        let action = target_ray_actions.get(&hand).unwrap();
                        let path = format!("{}aim/pose", path_prefix);
                        bindings.push(xr::Binding::new(
                            action,
                            instance.string_to_path(&path).unwrap(),
                        ));
                    }
                }
                PosesBindings::Custom { grip, target_ray } => {
                    for (hand, path) in grip {
                        let action = grip_actions.get(&hand).unwrap();
                        bindings.push(xr::Binding::new(
                            action,
                            instance.string_to_path(&path).unwrap(),
                        ));
                    }
                    for (hand, path) in target_ray {
                        let action = target_ray_actions.get(&hand).unwrap();
                        bindings.push(xr::Binding::new(
                            action,
                            instance.string_to_path(&path).unwrap(),
                        ));
                    }
                }
            }

            match profile.vibration {
                VibrationBindings::None => (),
                VibrationBindings::Default => {
                    for hand in [HandType::Left, HandType::Right] {
                        let action = vibration_actions.get(&hand).unwrap();
                        let path = format!("/user/hand/{}/output/haptic", hand_str(hand));
                        bindings.push(xr::Binding::new(
                            action,
                            instance.string_to_path(&path).unwrap(),
                        ));
                    }
                }
                VibrationBindings::Custom(paths) => {
                    for (hand, paths) in paths {
                        let action = vibration_actions.get(&hand).unwrap();
                        for path in paths {
                            bindings.push(xr::Binding::new(
                                action,
                                instance.string_to_path(&path).unwrap(),
                            ));
                        }
                    }
                }
            }

            let profile_path = instance.string_to_path(&profile.profile_path).unwrap();

            // Ignore error for unsupported profiles.
            instance
                .suggest_interaction_profile_bindings(profile_path, &bindings)
                .ok();
        }

        InteractionContext {
            action_set,
            button_actions,
            axes_actions,
            grip_actions,
            target_ray_actions,
            vibration_actions,
        }
    }
}

pub(crate) fn handle_input(
    context: &InteractionContext,
    session: &OpenXrSession,
    buttons: &mut XrButtons,
    axes: &mut XrAxes,
) {
    session
        .sync_actions(&[(&context.action_set).into()])
        .unwrap();

    for (&(hand, button), actions) in &context.button_actions {
        let touched = actions
            .touch
            .state(&session, xr::Path::NULL)
            .unwrap()
            .current_state;
        let pressed = actions
            .click
            .state(&session, xr::Path::NULL)
            .unwrap()
            .current_state;
        let value = actions
            .value
            .state(&session, xr::Path::NULL)
            .unwrap()
            .current_state;

        let state = if pressed {
            XrButtonState::Pressed
        } else if touched {
            XrButtonState::Touched
        } else {
            XrButtonState::Default
        };

        buttons.set(hand, button, state, value);
    }

    for (&(hand, axis), action) in &context.axes_actions {
        let value = action
            .state(&session, xr::Path::NULL)
            .unwrap()
            .current_state;
        axes.set(hand, axis, value);
    }
}

pub(crate) fn handle_output(
    context: &InteractionContext,
    session: &OpenXrSession,
    vibration_event_reader: &mut ManualEventReader<VibrationEvent>,
    vibration_events: &mut Events<VibrationEvent>,
) {
    for event in vibration_event_reader.iter(&vibration_events) {
        let action = context.vibration_actions.get(&event.hand);
        if let Some(action) = action {
            match &event.command {
                VibrationEventType::Apply {
                    duration,
                    frequency,
                    amplitude,
                } => {
                    let haptic_vibration = xr::HapticVibration::new()
                        .duration(from_duration(*duration))
                        .frequency(*frequency)
                        .amplitude(*amplitude);

                    action
                        .apply_feedback(&session, xr::Path::NULL, &haptic_vibration)
                        .unwrap();
                }
                VibrationEventType::Stop => action.stop_feedback(&session, xr::Path::NULL).unwrap(),
            }
        }
    }

    vibration_events.update();
}
