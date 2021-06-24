use crate::{
    conversion::{from_duration, to_quat, to_vec3},
    OpenXrSession,
};
use bevy_app::EventReader;
use bevy_ecs::{prelude::ResMut, system::Res};
use bevy_utils::HashMap;
use bevy_xr::interaction::{
    implementation::XrTrackingStateBackend, HandType, VibrationEvent, VibrationEventType, XrAxes,
    XrAxisType, XrButtonState, XrButtonType, XrButtons, XrPose, XrReferenceSpaceType,
    XrRigidTransform, XR_HAND_JOINT_COUNT,
};
use openxr as xr;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// If a button has a `value` binding but not a `click` binding, this threshold is used to determine
/// if the button has been pressed.
pub const PRESSED_THRESHOLD: f32 = 0.1;

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

pub struct Spaces {
    pub reference: Mutex<(XrReferenceSpaceType, xr::Space)>,
    pub left_grip: xr::Space,
    pub right_grip: xr::Space,
    pub left_target_ray: xr::Space,
    pub right_target_ray: xr::Space,
}

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

#[derive(Clone, Serialize, Deserialize)]
pub enum ButtonPaths {
    // `Default` uses "/user/hand/{hand}/input/{identifier}" for both `click` and `value` components
    // (which are omitted). Identifers are extracted from `XrButtonType` by converting the variants
    // to lowercase, except for FaceButton1 => "a", FaceButton2 => "b".
    Default {
        has_touch: bool,
    },
    Custom {
        touch: Vec<String>,
        click: Vec<String>,
        value: Vec<String>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub enum AxesBindings {
    Default { touchpad: bool, thumbstick: bool },
    Custom(HashMap<(HandType, XrAxisType), Vec<String>>),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum VibrationBindings {
    None,
    // `Default` uses "/user/hand/{hand}/output/haptic" paths
    Default,
    Custom(HashMap<HandType, Vec<String>>),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum PosesBindings {
    None,
    Default,
    Custom {
        grip: HashMap<HandType, String>,
        target_ray: HashMap<HandType, String>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OpenXrProfileBindings {
    pub profile_path: String,
    /// The first action of each type is used. `force` and `value` are considered as the same type.
    pub buttons: HashMap<(HandType, XrButtonType), ButtonPaths>,
    pub axes: AxesBindings,
    pub poses: PosesBindings,
    pub vibration: VibrationBindings,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OpenXrBindings {
    /// Ordered by preference. As a fallback, KHR_PROFILE should be last.
    pub profiles: Vec<OpenXrProfileBindings>,
}

/// Default mappings for known controllers. The mappings mirror WebXR ones. In case the user wants
/// to support newer controllers or wants to remap the buttons, OpenXrBindings resource must be
/// provided exlicitly.
impl Default for OpenXrBindings {
    fn default() -> Self {
        Self {
            profiles: vec![
                OpenXrProfileBindings {
                    profile_path: OCULUS_TOUCH_PROFILE.into(),
                    buttons: vec![
                        (
                            (HandType::Left, XrButtonType::Menu),
                            ButtonPaths::Default { has_touch: false },
                        ),
                        (
                            (HandType::Right, XrButtonType::Menu),
                            ButtonPaths::Custom {
                                touch: vec![],
                                click: vec!["/user/hand/right/input/system".into()],
                                value: vec!["/user/hand/right/input/system".into()],
                            },
                        ),
                        (
                            (HandType::Left, XrButtonType::Trigger),
                            ButtonPaths::Default { has_touch: true },
                        ),
                        (
                            (HandType::Right, XrButtonType::Trigger),
                            ButtonPaths::Default { has_touch: true },
                        ),
                        (
                            (HandType::Left, XrButtonType::Squeeze),
                            ButtonPaths::Default { has_touch: false },
                        ),
                        (
                            (HandType::Right, XrButtonType::Squeeze),
                            ButtonPaths::Default { has_touch: false },
                        ),
                        (
                            (HandType::Left, XrButtonType::Thumbstick),
                            ButtonPaths::Default { has_touch: true },
                        ),
                        (
                            (HandType::Right, XrButtonType::Thumbstick),
                            ButtonPaths::Default { has_touch: true },
                        ),
                        (
                            (HandType::Left, XrButtonType::FaceButton1),
                            ButtonPaths::Custom {
                                touch: vec!["/user/hand/left/input/x/touch".into()],
                                click: vec!["/user/hand/left/input/x".into()],
                                value: vec!["/user/hand/left/input/x".into()],
                            },
                        ),
                        (
                            (HandType::Right, XrButtonType::FaceButton1),
                            ButtonPaths::Default { has_touch: true },
                        ),
                        (
                            (HandType::Left, XrButtonType::FaceButton2),
                            ButtonPaths::Custom {
                                touch: vec!["/user/hand/left/input/y/touch".into()],
                                click: vec!["/user/hand/left/input/y".into()],
                                value: vec!["/user/hand/left/input/y".into()],
                            },
                        ),
                        (
                            (HandType::Right, XrButtonType::FaceButton2),
                            ButtonPaths::Default { has_touch: true },
                        ),
                        (
                            (HandType::Left, XrButtonType::Thumbrest),
                            ButtonPaths::Custom {
                                touch: vec!["/user/hand/left/input/thumbrest/touch".into()],
                                click: vec![],
                                value: vec![],
                            },
                        ),
                        (
                            (HandType::Right, XrButtonType::Thumbrest),
                            ButtonPaths::Custom {
                                touch: vec!["/user/hand/right/input/thumbrest/touch".into()],
                                click: vec![],
                                value: vec![],
                            },
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    axes: AxesBindings::Default {
                        touchpad: false,
                        thumbstick: true,
                    },
                    poses: PosesBindings::Default,
                    vibration: VibrationBindings::Default,
                },
                // todo: the rest of the profiles
                OpenXrProfileBindings {
                    profile_path: KHR_PROFILE.into(),
                    buttons: vec![
                        (
                            (HandType::Left, XrButtonType::Menu),
                            ButtonPaths::Default { has_touch: false },
                        ),
                        (
                            (HandType::Right, XrButtonType::Menu),
                            ButtonPaths::Default { has_touch: false },
                        ),
                        (
                            (HandType::Left, XrButtonType::Trigger),
                            ButtonPaths::Default { has_touch: false },
                        ),
                        (
                            (HandType::Right, XrButtonType::Trigger),
                            ButtonPaths::Default { has_touch: false },
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    axes: AxesBindings::Default {
                        touchpad: false,
                        thumbstick: false,
                    },
                    poses: PosesBindings::Default,
                    vibration: VibrationBindings::Default,
                },
            ],
        }
    }
}

struct ButtonActions {
    touch: xr::Action<bool>,
    click: xr::Action<bool>,
    value: xr::Action<f32>,
}

pub struct OpenXrInteractionContext {
    action_set: xr::ActionSet,
    button_actions: HashMap<(HandType, XrButtonType), ButtonActions>,
    axes_actions: HashMap<(HandType, XrAxisType), xr::Action<f32>>,
    grip_actions: HashMap<HandType, xr::Action<xr::Posef>>,
    target_ray_actions: HashMap<HandType, xr::Action<xr::Posef>>,
    vibration_actions: HashMap<HandType, xr::Action<xr::Haptic>>,
}

impl OpenXrInteractionContext {
    pub(crate) fn new(instance: &xr::Instance, bindings: OpenXrBindings) -> Self {
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

        OpenXrInteractionContext {
            action_set,
            button_actions,
            axes_actions,
            grip_actions,
            target_ray_actions,
            vibration_actions,
        }
    }
}

pub(crate) fn input_system(
    context: Res<OpenXrInteractionContext>,
    frame_state: Res<xr::FrameState>,
    mut tracking_state: ResMut<OpenXrTrackingState>,
    mut buttons: ResMut<XrButtons>,
    mut axes: ResMut<XrAxes>,
) {
    tracking_state.next_vsync_time = frame_state.predicted_display_time;

    let session = &tracking_state.session;

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

pub(crate) fn output_system(
    context: Res<Arc<OpenXrInteractionContext>>,
    session: Res<OpenXrSession>,
    mut vibration_events: EventReader<VibrationEvent>,
) {
    let session = session.to_backend();

    for event in vibration_events.iter() {
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
}

pub struct OpenXrTrackingState {
    view_type: xr::ViewConfigurationType,
    action_set: xr::ActionSet,
    session: xr::Session<xr::AnyGraphics>,
    spaces: Spaces,
    next_vsync_time: xr::Time,
}

impl OpenXrTrackingState {
    pub fn spaces(&self) -> &Spaces {
        &self.spaces
    }
}

impl XrTrackingStateBackend for OpenXrTrackingState {
    fn get_reference_space_type(&self) -> XrReferenceSpaceType {
        self.spaces.reference.lock().unwrap().0
    }

    fn set_reference_space_type(&self, mode: XrReferenceSpaceType) -> bool {
        let reference_type = match mode {
            XrReferenceSpaceType::Viewer => xr::ReferenceSpaceType::VIEW,
            XrReferenceSpaceType::Local => xr::ReferenceSpaceType::LOCAL,
            XrReferenceSpaceType::BoundedFloor => xr::ReferenceSpaceType::STAGE,
        };
        if let Ok(space) = self
            .session
            .create_reference_space(reference_type, xr::Posef::IDENTITY)
        {
            *self.spaces.reference.lock().unwrap() = (mode, space);

            true
        } else {
            false
        }
    }

    fn tracking_area_bounds(&self) -> Option<(f32, f32)> {
        todo!()
    }

    fn views_poses(&self) -> Vec<XrPose> {
        self.session
            .sync_actions(&[(&self.action_set).into()])
            .unwrap();

        let (flags, views) = self
            .session
            .locate_views(
                self.view_type,
                self.next_vsync_time,
                &self.spaces.reference.lock().unwrap().1,
            )
            .unwrap();

        views
            .into_iter()
            .map(|view| XrPose {
                transform: XrRigidTransform {
                    position: if flags.contains(xr::ViewStateFlags::POSITION_VALID) {
                        Some(to_vec3(view.pose.position))
                    } else {
                        None
                    },
                    orientation: to_quat(view.pose.orientation),
                },
                linear_velocity: None,
                angular_velocity: None,
            })
            .collect()
    }

    fn hand_pose(&self, hand_type: HandType) -> Option<XrPose> {
        self.session
            .sync_actions(&[(&self.action_set).into()])
            .unwrap();

        let space = match hand_type {
            HandType::Left => &self.spaces.left_grip,
            HandType::Right => &self.spaces.right_grip,
        };

        let (location, velocity) = space
            .relate(
                &self.spaces.reference.lock().unwrap().1,
                self.next_vsync_time,
            )
            .unwrap();

        let position = if location
            .location_flags
            .contains(xr::SpaceLocationFlags::POSITION_VALID)
        {
            Some(to_vec3(location.pose.position))
        } else {
            None
        };
        let linear_velocity = if velocity
            .velocity_flags
            .contains(xr::SpaceVelocityFlags::LINEAR_VALID)
        {
            Some(to_vec3(velocity.linear_velocity))
        } else {
            None
        };
        let angular_velocity = if velocity
            .velocity_flags
            .contains(xr::SpaceVelocityFlags::ANGULAR_VALID)
        {
            Some(to_vec3(velocity.angular_velocity))
        } else {
            None
        };

        if location
            .location_flags
            .contains(xr::SpaceLocationFlags::ORIENTATION_VALID)
        {
            Some(XrPose {
                transform: XrRigidTransform {
                    position,
                    orientation: to_quat(location.pose.orientation),
                },
                linear_velocity,
                angular_velocity,
            })
        } else {
            None
        }
    }

    fn hand_skeleton_pose(&self, hand_type: HandType) -> Option<[XrPose; XR_HAND_JOINT_COUNT]> {
        self.session
            .sync_actions(&[(&self.action_set).into()])
            .unwrap();

        todo!()
    }

    fn hand_target_ray(&self, hand_type: HandType) -> Option<XrPose> {
        self.session
            .sync_actions(&[(&self.action_set).into()])
            .unwrap();

        let space = match hand_type {
            HandType::Left => &self.spaces.left_target_ray,
            HandType::Right => &self.spaces.right_target_ray,
        };

        let (location, velocity) = space
            .relate(
                &self.spaces.reference.lock().unwrap().1,
                self.next_vsync_time,
            )
            .unwrap();

        let position = if location
            .location_flags
            .contains(xr::SpaceLocationFlags::POSITION_VALID)
        {
            Some(to_vec3(location.pose.position))
        } else {
            None
        };
        let linear_velocity = if velocity
            .velocity_flags
            .contains(xr::SpaceVelocityFlags::LINEAR_VALID)
        {
            Some(to_vec3(velocity.linear_velocity))
        } else {
            None
        };
        let angular_velocity = if velocity
            .velocity_flags
            .contains(xr::SpaceVelocityFlags::ANGULAR_VALID)
        {
            Some(to_vec3(velocity.angular_velocity))
        } else {
            None
        };

        if location
            .location_flags
            .contains(xr::SpaceLocationFlags::ORIENTATION_VALID)
        {
            Some(XrPose {
                transform: XrRigidTransform {
                    position,
                    orientation: to_quat(location.pose.orientation),
                },
                linear_velocity,
                angular_velocity,
            })
        } else {
            None
        }
    }

    fn viewer_target_ray(&self) -> XrPose {
        let poses = self.views_poses();
        let poses_count = poses.len() as f32;

        let orientation = poses.first().unwrap().transform.orientation;

        let position = poses
            .into_iter()
            .map(|pose| pose.transform.position)
            .reduce(|pos1, pos2| Some(pos1? + pos2?))
            .unwrap()
            .map(|sum| sum / poses_count);

        XrPose {
            transform: XrRigidTransform {
                position,
                orientation,
            },
            linear_velocity: None,
            angular_velocity: None,
        }
    }
}
