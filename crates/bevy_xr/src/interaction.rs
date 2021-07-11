use bevy_math::{Mat4, Quat, Vec3};
use bevy_utils::Duration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Note: indices follow WebXR convention. OpenXR's palm joint is missing, but it can be retrieved
// using `XrState::hand_pose()`.
pub const XR_HAND_JOINT_WRIST: usize = 0;
pub const XR_HAND_JOINT_THUMB_METACARPAL: usize = 1;
pub const XR_HAND_JOINT_THUMB_PROXIMAL: usize = 2;
pub const XR_HAND_JOINT_THUMB_DISTAL: usize = 3;
pub const XR_HAND_JOINT_THUMB_TIP: usize = 4;
pub const XR_HAND_JOINT_INDEX_METACARPAL: usize = 5;
pub const XR_HAND_JOINT_INDEX_PROXIMAL: usize = 6;
pub const XR_HAND_JOINT_INDEX_INTERMEDIATE: usize = 7;
pub const XR_HAND_JOINT_INDEX_DISTAL: usize = 8;
pub const XR_HAND_JOINT_INDEX_TIP: usize = 9;
pub const XR_HAND_JOINT_MIDDLE_METACARPAL: usize = 10;
pub const XR_HAND_JOINT_MIDDLE_PROXIMAL: usize = 11;
pub const XR_HAND_JOINT_MIDDLE_INTERMEDIATE: usize = 12;
pub const XR_HAND_JOINT_MIDDLE_DISTAL: usize = 13;
pub const XR_HAND_JOINT_MIDDLE_TIP: usize = 14;
pub const XR_HAND_JOINT_RING_METACARPAL: usize = 15;
pub const XR_HAND_JOINT_RING_PROXIMAL: usize = 16;
pub const XR_HAND_JOINT_RING_INTERMEDIATE: usize = 17;
pub const XR_HAND_JOINT_RING_DISTAL: usize = 18;
pub const XR_HAND_JOINT_RING_TIP: usize = 19;
pub const XR_HAND_JOINT_LITTLE_METACARPAL: usize = 20;
pub const XR_HAND_JOINT_LITTLE_PROXIMAL: usize = 21;
pub const XR_HAND_JOINT_LITTLE_INTERMEDIATE: usize = 22;
pub const XR_HAND_JOINT_LITTLE_DISTAL: usize = 23;
pub const XR_HAND_JOINT_LITTLE_TIP: usize = 24;
pub const XR_HAND_JOINT_COUNT: usize = 25;

#[derive(Clone, Debug, Default)]
pub struct XrRigidTransform {
    // todo: for OpenXR, provide a neck/arm model if needed and remove `Option`
    pub position: Option<Vec3>,
    pub orientation: Quat,
}

impl XrRigidTransform {
    pub fn to_mat4(&self) -> Mat4 {
        todo!()
    }
}

#[derive(Clone, Debug, Default)]
pub struct XrPose {
    pub transform: XrRigidTransform,
    pub linear_velocity: Option<Vec3>,
    pub angular_velocity: Option<Vec3>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum XrReferenceSpaceType {
    /// The coordinate system (position and orientation) is set as the headset pose at startup or
    /// after a recenter. This should be used only for experiences where the user is laid down.
    Viewer,

    /// The coordinate system (position and gravity-aligned orientation) is calculated from the
    /// headset pose at startup or after a recenter. This is for seated experiences.
    Local,

    /// The coordinate system (position and orientation) corresponds to the center of a rectangle at
    /// floor level, with +Y up. This is for stading or room-scale experiences.
    BoundedFloor,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum HandType {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum XrButtonState {
    Default,
    Touched,
    Pressed,
}

impl Default for XrButtonState {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum XrButtonType {
    Menu,
    Trigger,
    Squeeze,
    Touchpad,
    Thumbstick,
    FaceButton1,
    FaceButton2,
    Thumbrest,
}

pub struct XrButtons {
    current_state: HashMap<(HandType, XrButtonType), XrButtonState>,
    previous_state: HashMap<(HandType, XrButtonType), XrButtonState>,
    values: HashMap<(HandType, XrButtonType), f32>,
}

impl Default for XrButtons {
    fn default() -> Self {
        Self {
            current_state: HashMap::new(),
            previous_state: HashMap::new(),
            values: HashMap::new(),
        }
    }
}

impl XrButtons {
    fn previous_state(&self, hand: HandType, button: XrButtonType) -> XrButtonState {
        self.previous_state
            .get(&(hand, button))
            .cloned()
            .unwrap_or_default()
    }

    pub fn state(&self, hand: HandType, button: XrButtonType) -> XrButtonState {
        self.current_state
            .get(&(hand, button))
            .cloned()
            .unwrap_or_default()
    }

    pub fn touched(&self, hand: HandType, button: XrButtonType) -> bool {
        self.state(hand, button) != XrButtonState::Default
    }

    pub fn pressed(&self, hand: HandType, button: XrButtonType) -> bool {
        self.state(hand, button) == XrButtonState::Pressed
    }

    pub fn just_touched(&self, hand: HandType, button: XrButtonType) -> bool {
        self.touched(hand, button) && self.previous_state(hand, button) == XrButtonState::Default
    }

    pub fn just_untouched(&self, hand: HandType, button: XrButtonType) -> bool {
        !self.touched(hand, button) && self.previous_state(hand, button) != XrButtonState::Default
    }

    pub fn just_pressed(&self, hand: HandType, button: XrButtonType) -> bool {
        self.pressed(hand, button) && self.previous_state(hand, button) != XrButtonState::Pressed
    }

    pub fn just_unpressed(&self, hand: HandType, button: XrButtonType) -> bool {
        !self.pressed(hand, button) && self.previous_state(hand, button) == XrButtonState::Pressed
    }

    // Returns a value between 0 and 1, where 1 is completely pressed.
    pub fn value(&self, hand: HandType, button: XrButtonType) -> f32 {
        self.values
            .get(&(hand, button))
            .cloned()
            .unwrap_or_default()
    }

    pub fn set(&mut self, hand: HandType, button: XrButtonType, state: XrButtonState, value: f32) {
        self.previous_state
            .insert((hand, button), self.state(hand, button));
        self.current_state.insert((hand, button), state);
        self.values.insert((hand, button), value);
    }

    pub fn clear(&mut self) {
        self.current_state.clear();
        self.previous_state.clear();
        self.values.clear();
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum XrAxisType {
    TouchpadX,
    TouchpadY,
    ThumbstickX,
    ThumbstickY,
}

pub struct XrAxes(HashMap<(HandType, XrAxisType), f32>);

impl XrAxes {
    pub fn value(&self, hand: HandType, axis: XrAxisType) -> f32 {
        self.0.get(&(hand, axis)).cloned().unwrap_or_default()
    }

    pub fn set(&mut self, hand: HandType, axis: XrAxisType, value: f32) {
        self.0.insert((hand, axis), value);
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }
}

impl Default for XrAxes {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VibrationEventType {
    Apply {
        duration: Duration,
        frequency: f32,
        amplitude: f32,
    },
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VibrationEvent {
    pub hand: HandType,
    pub command: VibrationEventType,
}

/// Active interaction profiles. The format is backend-specific. They can be used to choose the
/// controller 3D models to display.
/// Note: in case skeletal hand tracking is active, the profiles still point to controller profiles.
/// The correct 3D model to display can be decided depending on if skeletal hand tracking data is
/// available or not.
#[derive(Default)]
pub struct XrProfiles {
    pub left_hand: Option<String>,
    pub right_hand: Option<String>,
}

pub mod implementation {
    use super::{HandType, XrReferenceSpaceType};
    use crate::interaction::{XrPose, XR_HAND_JOINT_COUNT};

    pub trait XrTrackingStateBackend: Send + Sync {
        fn get_reference_space_type(&self) -> XrReferenceSpaceType;
        fn set_reference_space_type(&self, reference_space_type: XrReferenceSpaceType) -> bool;
        fn tracking_area_bounds(&self) -> Option<(f32, f32)>;
        fn views_poses(&self) -> Vec<XrPose>;
        fn hand_pose(&self, hand_type: HandType) -> Option<XrPose>;
        fn hand_skeleton_pose(&self, hand_type: HandType) -> Option<[XrPose; XR_HAND_JOINT_COUNT]>;
        fn hand_target_ray(&self, hand_type: HandType) -> Option<XrPose>;
        fn viewer_target_ray(&self) -> XrPose;
    }
}

/// Component used to poll tracking data. Tracking data is obtained "on-demand" to get the best
/// precision possible. Poses are predicted for the next V-Sync. To obtain poses for an arbitrary
/// point in time, `bevy_openxr` backend provides this functionality with OpenXrTrackingState.
pub struct XrTrackingState {
    reference_space_type: XrReferenceSpaceType,
    inner: Box<dyn implementation::XrTrackingStateBackend>,
}

impl XrTrackingState {
    pub fn new(backend: Box<dyn implementation::XrTrackingStateBackend>) -> Self {
        Self {
            reference_space_type: XrReferenceSpaceType::Local,
            inner: backend,
        }
    }

    pub fn get_reference_space_type(&self) -> XrReferenceSpaceType {
        self.reference_space_type
    }

    // Returns true if the tracking mode has been set correctly. If false is returned the tracking
    // mode is not supported and another one must be chosen.
    pub fn set_reference_space_type(&mut self, reference_space_type: XrReferenceSpaceType) -> bool {
        if self.inner.set_reference_space_type(reference_space_type) {
            self.reference_space_type = reference_space_type;

            true
        } else {
            false
        }
    }

    pub fn tracking_area_bounds(&self) -> Option<(f32, f32)> {
        self.inner.tracking_area_bounds()
    }

    pub fn views_poses(&self) -> Vec<XrPose> {
        self.inner.views_poses()
    }

    pub fn hand_pose(&self, hand_type: HandType) -> Option<XrPose> {
        self.inner.hand_pose(hand_type)
    }

    pub fn hand_skeleton_pose(&self, hand_type: HandType) -> Option<[XrPose; XR_HAND_JOINT_COUNT]> {
        self.inner.hand_skeleton_pose(hand_type)
    }

    /// Returns a pose that can be used to render a target ray or cursor. The ray is along -Z. The
    /// behavior is vendor-specific.
    pub fn hand_target_ray(&self, hand_type: HandType) -> Option<XrPose> {
        self.inner.hand_target_ray(hand_type)
    }

    /// Returns a pose that can be used to render a target ray or cursor. The ray is along -Z. The
    /// origin is between the eyes for head-mounted displays and the center of the screen for
    /// handheld devices.
    pub fn viewer_target_ray(&self) -> XrPose {
        self.inner.viewer_target_ray()
    }

    // future extensions:
    // * eye tracking
    // * lower face tracking
    // * AR face tracking
    // * body/skeletal trackers
    // * scene understanding (anchors, planes, meshes)
}
