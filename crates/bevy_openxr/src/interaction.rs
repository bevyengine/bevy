use crate::{
    conversion::{to_vec2, to_xr_time},
    Session,
};
use bevy_app::{EventReader, EventWriter};
use bevy_xr::{interaction::*, XrTime};
use glam::Vec2;
use openxr as xr;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use xr::ActionState;

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

// Pose actions
pub const LEFT_GRIP_POSE: &str = "bevy_left_grip_pose";
pub const RIGHT_GRIP_POSE: &str = "bevy_right_grip_pose";
pub const LEFT_AIM_POSE: &str = "bevy_left_aim_pose";
pub const RIGHT_AIM_POSE: &str = "bevy_right_aim_pose";

// Virtual controllers action names. The user can override the virtual controllers behavior by
// submitting `OpenXrBindingDesc`s with these names.
pub const MENU_CLICK: &str = "bevy_menu_click";
pub const LEFT_PRIMARY_CLICK: &str = "bevy_left_primary_click";
pub const RIGHT_PRIMARY_CLICK: &str = "bevy_right_primary_click";
// ... todo: add the rest
pub const LEFT_VIBRATION: &str = "bevy_left_vibration";
pub const RIGHT_VIBRATION: &str = "bevy_right_vibration";

pub(crate) struct ControllerPairInputActions {
    menu_click: xr::Action<bool>,
    left_hand: ControllerInputActions,
    right_hand: ControllerInputActions,
}

struct ControllerInputActions {
    primary_click: xr::Action<bool>,
    primary_touch: xr::Action<bool>,
    secondary_click: xr::Action<bool>,
    secondary_touch: xr::Action<bool>,
    trigger_value: xr::Action<f32>,
    trigger_touch: xr::Action<bool>,
    squeeze_value: xr::Action<f32>,
    directional_value: xr::Action<xr::Vector2f>,
    directional_click: xr::Action<bool>,
    directional_touch: xr::Action<bool>,
}

pub(crate) struct ControllerPairOutputActions {
    left_vibration: xr::Action<xr::Haptic>,
    right_vibration: xr::Action<xr::Haptic>,
}

#[derive(Clone)]
pub struct OpenXrActionPath {
    pub profile: &'static str,
    pub path: &'static str,
}

#[derive(Clone)]
pub enum OpenXrActionType {
    BinaryInput,
    FloatInput,
    Vector2Input,
    PoseInput,
    VibrationOutput,
}

#[derive(Clone)]
pub struct OpenXrBindingDesc {
    pub name: &'static str,
    pub paths: Vec<OpenXrActionPath>,
    pub action_type: OpenXrActionType,
}

#[derive(Clone)]
pub(crate) enum XrAction {
    BinaryInput(xr::Action<bool>),
    FloatInput(xr::Action<f32>),
    Vector2Input(xr::Action<xr::Vector2f>),
    PoseInput(xr::Action<xr::Posef>),
    VibrationOutput(xr::Action<xr::Haptic>),
}

impl XrAction {
    fn unwrap_binary(self) -> xr::Action<bool> {
        match self {
            Self::BinaryInput(action) => action,
            _ => panic!(),
        }
    }
    fn unwrap_float(self) -> xr::Action<f32> {
        match self {
            Self::FloatInput(action) => action,
            _ => panic!(),
        }
    }
    fn unwrap_vector2(self) -> xr::Action<xr::Vector2f> {
        match self {
            Self::Vector2Input(action) => action,
            _ => panic!(),
        }
    }
    fn unwrap_pose(self) -> xr::Action<xr::Posef> {
        match self {
            Self::PoseInput(action) => action,
            _ => panic!(),
        }
    }
    fn unwrap_vibration(self) -> xr::Action<xr::Haptic> {
        match self {
            Self::VibrationOutput(action) => action,
            _ => panic!(),
        }
    }
}

pub(crate) fn pose_bindings() -> Vec<OpenXrBindingDesc> {
    fn binding(name: &'static str, path: &'static str) -> OpenXrBindingDesc {
        OpenXrBindingDesc {
            name,
            paths: vec![OpenXrActionPath {
                profile: KHR_PROFILE,
                path,
            }],
            action_type: OpenXrActionType::PoseInput,
        }
    }

    vec![
        binding(LEFT_GRIP_POSE, "/user/hand/left/input/grip/pose"),
        binding(RIGHT_GRIP_POSE, "/user/hand/right/input/grip/pose"),
        binding(LEFT_AIM_POSE, "/user/hand/left/input/aim/pose"),
        binding(RIGHT_AIM_POSE, "/user/hand/right/input/aim/pose"),
    ]
}

pub(crate) fn create_actions(
    instance: &xr::Instance,
    action_set: &xr::ActionSet,
    bindings: &[OpenXrBindingDesc],
) -> HashMap<&'static str, XrAction> {
    // filter out duplicates, retain the last occurrence
    let mut filtered_bindings = HashMap::new();
    for binding in bindings {
        filtered_bindings.insert(binding.name, binding);
    }

    let mut actions = HashMap::new();
    let mut bindings_by_profile = HashMap::<&'static str, Vec<_>>::new();
    for binding in filtered_bindings.values() {
        let action = match binding.action_type {
            OpenXrActionType::BinaryInput => XrAction::BinaryInput(
                action_set
                    .create_action(binding.name, binding.name, &[])
                    .unwrap(),
            ),
            OpenXrActionType::FloatInput => XrAction::FloatInput(
                action_set
                    .create_action(binding.name, binding.name, &[])
                    .unwrap(),
            ),
            OpenXrActionType::Vector2Input => XrAction::Vector2Input(
                action_set
                    .create_action(binding.name, binding.name, &[])
                    .unwrap(),
            ),
            OpenXrActionType::PoseInput => XrAction::PoseInput(
                action_set
                    .create_action(binding.name, binding.name, &[])
                    .unwrap(),
            ),
            OpenXrActionType::VibrationOutput => XrAction::VibrationOutput(
                action_set
                    .create_action(binding.name, binding.name, &[])
                    .unwrap(),
            ),
        };
        actions.insert(binding.name, action);

        for id in &binding.paths {
            bindings_by_profile
                .entry(id.profile)
                .or_default()
                .push((binding.name, id.path));
        }
    }

    for (profile, bindings) in bindings_by_profile {
        let profile = instance.string_to_path(profile).unwrap();

        let mut xr_bindings = vec![];
        for (name, path) in bindings {
            let path = instance.string_to_path(path).unwrap();

            let action = actions.get(name).unwrap();

            let binding = match action {
                XrAction::BinaryInput(action) => xr::Binding::new(action, path),
                XrAction::FloatInput(action) => xr::Binding::new(action, path),
                XrAction::Vector2Input(action) => xr::Binding::new(action, path),
                XrAction::PoseInput(action) => xr::Binding::new(action, path),
                XrAction::VibrationOutput(action) => xr::Binding::new(action, path),
            };

            xr_bindings.push(binding);
        }

        // This must be called only once per interaction profile.
        // Ignore error for unsupported profiles.
        instance
            .suggest_interaction_profile_bindings(profile, &xr_bindings)
            .ok();
    }

    actions
}

pub(crate) struct PoseActions {
    left_grip: xr::Action<xr::Posef>,
    right_grip: xr::Action<xr::Posef>,
    left_aim: xr::Action<xr::Posef>,
    right_aim: xr::Action<xr::Posef>,
}

pub(crate) struct Spaces {
    pub reference: xr::Space,
    pub left_grip: xr::Space,
    pub right_grip: xr::Space,
    pub left_aim: xr::Space,
    pub right_aim: xr::Space,
}

pub(crate) fn extract_pose_actions(actions: &mut HashMap<&'static str, XrAction>) -> PoseActions {
    PoseActions {
        left_grip: actions.remove(LEFT_GRIP_POSE).unwrap().unwrap_pose(),
        right_grip: actions.remove(RIGHT_GRIP_POSE).unwrap().unwrap_pose(),
        left_aim: actions.remove(LEFT_GRIP_POSE).unwrap().unwrap_pose(),
        right_aim: actions.remove(LEFT_GRIP_POSE).unwrap().unwrap_pose(),
    }
}

pub(crate) fn extract_controller_actions(
    actions: &mut HashMap<&'static str, XrAction>,
) -> (ControllerPairInputActions, ControllerPairOutputActions) {
    (
        ControllerPairInputActions {
            menu_click: actions.remove(MENU_CLICK).unwrap().unwrap_binary(),
            left_hand: todo!(),
            right_hand: todo!(),
        },
        ControllerPairOutputActions {
            left_vibration: actions.remove(LEFT_VIBRATION).unwrap().unwrap_vibration(),
            right_vibration: actions.remove(RIGHT_VIBRATION).unwrap().unwrap_vibration(),
        },
    )
}

#[inline]
fn action_state_to_binary_event(state: ActionState<bool>) -> BinaryEvent {
    BinaryEvent {
        value: state.current_state,
        toggled: state.changed_since_last_sync,
    }
}

pub(crate) fn controller_input_system_fn(
    session: Arc<Mutex<Option<Session>>>,
    controller_input_actions: ControllerPairInputActions,
) -> impl FnMut(EventWriter<GenericControllerPairButtons>) {
    move |mut event_writer| {
        let session_lock = session.lock().unwrap();
        let session = if let Some(session) = &*session_lock {
            session
        } else {
            return;
        };

        let mut event = GenericControllerPairButtons::default();
        let mut input_changed = false;

        let menu_click = session
            .backend
            .state(&controller_input_actions.menu_click)
            .unwrap();
        input_changed |= menu_click.changed_since_last_sync;

        for (actions, event) in &mut [
            (&controller_input_actions.left_hand, &mut event.left_hand),
            (&controller_input_actions.right_hand, &mut event.right_hand),
        ] {
            let primary_click = session.backend.state(&actions.primary_click).unwrap();
            event.primary_click = action_state_to_binary_event(primary_click);

            let primary_touch = session.backend.state(&actions.primary_touch).unwrap();
            event.primary_touch = action_state_to_binary_event(primary_touch);

            let secondary_click = session.backend.state(&actions.secondary_click).unwrap();
            event.secondary_click = action_state_to_binary_event(secondary_click);

            let secondary_touch = session.backend.state(&actions.secondary_touch).unwrap();
            event.secondary_touch = action_state_to_binary_event(secondary_touch);

            let trigger_value = session.backend.state(&actions.trigger_value).unwrap();
            event.trigger_value = trigger_value.current_state;

            let trigger_touch = session.backend.state(&actions.trigger_touch).unwrap();
            event.trigger_touch = action_state_to_binary_event(trigger_touch);

            let squeeze_value = session.backend.state(&actions.squeeze_value).unwrap();
            event.squeeze_value = squeeze_value.current_state;

            let directional_value = session.backend.state(&actions.directional_value).unwrap();
            event.directional_value = to_vec2(directional_value.current_state);

            let directional_click = session.backend.state(&actions.directional_click).unwrap();
            event.directional_click = action_state_to_binary_event(directional_click);

            let directional_touch = session.backend.state(&actions.directional_touch).unwrap();
            event.directional_touch = action_state_to_binary_event(directional_touch);

            input_changed |= primary_click.changed_since_last_sync
                | primary_touch.changed_since_last_sync
                | secondary_click.changed_since_last_sync
                | secondary_touch.changed_since_last_sync
                | trigger_value.changed_since_last_sync
                | trigger_touch.changed_since_last_sync
                | squeeze_value.changed_since_last_sync
                | directional_value.changed_since_last_sync
                | directional_click.changed_since_last_sync
                | directional_touch.changed_since_last_sync;
        }

        if input_changed {
            event_writer.send(event);
        }
    }
}

pub(crate) fn controller_output_system_fn(
    session: Arc<Mutex<Option<Session>>>,
    controller_output_actions: ControllerPairOutputActions,
) -> impl Fn(EventReader<GenericControllerVibration>) + 'static {
    move |mut event_reader| {
        let session_lock = session.lock().unwrap();
        let session = if let Some(session) = &*session_lock {
            session
        } else {
            return;
        };

        for event in event_reader.iter() {
            let action = match event.hand {
                HandType::Left => &controller_output_actions.left_vibration,
                HandType::Right => &controller_output_actions.right_vibration,
            };

            match &event.action {
                Vibration::Apply {
                    duration,
                    frequency,
                    amplitude,
                } => {
                    let haptic_vibration = xr::HapticVibration::new()
                        .duration(xr::Duration::from_nanos(duration.as_nanos()))
                        .frequency(*frequency)
                        .amplitude(*amplitude);
                    session
                        .backend
                        .apply_feedback(action, &haptic_vibration)
                        .unwrap();
                }
                Vibration::Stop => session.backend.stop_feedback(action).unwrap(),
            }
        }
    }
}

pub struct OpenXrVendorInput<T> {
    pub name: &'static str,
    pub value: T,
    pub last_change_time: XrTime,
}

pub(crate) fn vendor_input_system_fn(
    session: Arc<Mutex<Option<Session>>>,
    actions: HashMap<&'static str, XrAction>,
) -> impl FnMut(
    EventWriter<OpenXrVendorInput<bool>>,
    EventWriter<OpenXrVendorInput<f32>>,
    EventWriter<OpenXrVendorInput<Vec2>>,
) {
    move |mut binary_events, mut float_events, mut vec2_events| {
        let session_lock = session.lock().unwrap();
        let session = if let Some(session) = &*session_lock {
            session
        } else {
            return;
        };

        for (name, action) in actions.iter() {
            match action {
                XrAction::BinaryInput(action) => {
                    let state = session.backend.state(action).unwrap();
                    if state.changed_since_last_sync {
                        binary_events.send(OpenXrVendorInput {
                            name,
                            value: state.current_state,
                            last_change_time: to_xr_time(state.last_change_time),
                        })
                    }
                }
                XrAction::FloatInput(action) => {
                    let state = session.backend.state(action).unwrap();
                    if state.changed_since_last_sync {
                        float_events.send(OpenXrVendorInput {
                            name,
                            value: state.current_state,
                            last_change_time: to_xr_time(state.last_change_time),
                        })
                    }
                }
                XrAction::Vector2Input(action) => {
                    let state = session.backend.state(action).unwrap();
                    if state.changed_since_last_sync {
                        vec2_events.send(OpenXrVendorInput {
                            name,
                            value: to_vec2(state.current_state),
                            last_change_time: to_xr_time(state.last_change_time),
                        })
                    }
                }
                XrAction::PoseInput(_) => panic!("XR: Custom Pose events are not supported."),
                XrAction::VibrationOutput(_) => (),
            }
        }
    }
}

pub struct OpenXrVendorOutput<T> {
    pub name: &'static str,
    pub value: T,
}

pub(crate) fn vendor_output_system_fn(
    session: Arc<Mutex<Option<Session>>>,
    actions: HashMap<&'static str, XrAction>,
) -> impl FnMut(EventReader<OpenXrVendorOutput<Vibration>>) {
    move |mut vibration_events| {
        let session_lock = session.lock().unwrap();
        let session = if let Some(session) = &*session_lock {
            session
        } else {
            return;
        };

        for event in vibration_events.iter() {
            if let Some(action) = actions.get(event.name) {
                if let XrAction::VibrationOutput(action) = action {
                    match &event.value {
                        Vibration::Apply {
                            duration,
                            frequency,
                            amplitude,
                        } => {
                            let haptic_vibration = xr::HapticVibration::new()
                                .duration(xr::Duration::from_nanos(duration.as_nanos()))
                                .frequency(*frequency)
                                .amplitude(*amplitude);
                            session
                                .backend
                                .apply_feedback(action, &haptic_vibration)
                                .unwrap();
                        }
                        Vibration::Stop => session.backend.stop_feedback(action).unwrap(),
                    }
                } else {
                    panic!("XR: \"{}\" is not a vibration action.", event.name)
                }
            } else {
                panic!("XR: \"{}\" action is not registered.", event.name)
            }
        }
    }
}
