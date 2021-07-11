use bevy_utils::HashMap;
use bevy_xr::{HandType, XrAxisType, XrButtonType};
use serde::{Deserialize, Serialize};

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
