use bevy_math::{Mat4, Quat, Vec2, Vec3};
use bevy_utils::Duration;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::{Deref, Mul},
};

// Note: indices follow WebXR convention. OpenXR's palm joint is missing, but it can be retrieved
// using `XrTrackingSource::hands_pose()`.
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

// To be verified: in all useful instances, when the orientation is valid, the position is also
// valid. In case of 3DOF headsets, position should always be emulated using a neck and arm model.
// In case of hand tracking, when a joint is estimated, both pose and orientation are available.
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct XrRigidTransform {
    pub position: Vec3,
    pub orientation: Quat,
}

impl Mul for XrRigidTransform {
    type Output = XrRigidTransform;

    fn mul(self, rhs: Self) -> Self::Output {
        XrRigidTransform {
            position: self.position + self.orientation * rhs.position,
            orientation: self.orientation * rhs.orientation,
        }
    }
}

impl XrRigidTransform {
    pub fn to_mat4(&self) -> Mat4 {
        todo!()
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct XrPose {
    pub transform: XrRigidTransform,
    pub linear_velocity: Option<Vec3>,
    pub angular_velocity: Option<Vec3>,
    pub emulated_position: bool,
}

impl Deref for XrPose {
    type Target = XrRigidTransform;

    fn deref(&self) -> &Self::Target {
        &self.transform
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct XrJointPose {
    pub pose: XrPose,

    /// Radius of a sphere placed at the center of the joint that roughly touches the skin on both
    /// sides of the hand.
    pub radius: f32,
}

impl Deref for XrJointPose {
    type Target = XrPose;

    fn deref(&self) -> &Self::Target {
        &self.pose
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub enum XrReferenceSpaceType {
    /// The coordinate system (position and orientation) is set as the headset pose at startup or
    /// after a recenter. This should be used only for experiences where the user is laid down.
    Viewer,

    /// The coordinate system (position and gravity-aligned orientation) is calculated from the
    /// headset pose at startup or after a recenter. This is for seated experiences.
    Local,

    /// The coordinate system (position and orientation) corresponds to the center of a rectangle at
    /// floor level, with +Y up. This is for stading or room-scale experiences.
    Stage,
}

pub mod implementation {
    use super::XrReferenceSpaceType;
    use crate::{interaction::XrPose, XrJointPose};
    use bevy_math::Vec3;

    pub trait XrTrackingSourceBackend: Send + Sync {
        fn reference_space_type(&self) -> XrReferenceSpaceType;
        fn set_reference_space_type(&self, reference_space_type: XrReferenceSpaceType) -> bool;
        fn bounds_geometry(&self) -> Option<Vec<Vec3>>;
        fn views_poses(&self) -> Vec<XrPose>;
        fn hands_pose(&self) -> [Option<XrPose>; 2];
        fn hands_skeleton_pose(&self) -> [Option<Vec<XrJointPose>>; 2];
        fn hands_target_ray(&self) -> [Option<XrPose>; 2];
        fn viewer_target_ray(&self) -> XrPose;
    }
}

/// Component used to poll tracking data. Tracking data is obtained "on-demand" to get the best
/// precision possible. Poses are predicted for the next V-Sync. To obtain poses for an arbitrary
/// point in time, `bevy_openxr` backend provides this functionality with OpenXrTrackingState.
pub struct XrTrackingSource {
    inner: Box<dyn implementation::XrTrackingSourceBackend>,
}

impl XrTrackingSource {
    pub fn new(backend: Box<dyn implementation::XrTrackingSourceBackend>) -> Self {
        Self { inner: backend }
    }

    pub fn reference_space_type(&self) -> XrReferenceSpaceType {
        self.inner.reference_space_type()
    }

    /// Returns true if the tracking mode has been set correctly. If false is returned the tracking
    /// mode is not supported and another one must be chosen.
    pub fn set_reference_space_type(&mut self, reference_space_type: XrReferenceSpaceType) -> bool {
        self.inner.set_reference_space_type(reference_space_type)
    }

    pub fn just_reset_reference_space(&mut self) -> bool {
        todo!()
    }

    /// Returns a list of points, ordered clockwise, that define the playspace boundary. Only
    /// available when the reference space is set to `BoundedFloor`. Y component is always 0.
    pub fn bounds_geometry(&self) -> Option<Vec<Vec3>> {
        self.inner.bounds_geometry()
    }

    pub fn views_poses(&self) -> Vec<XrPose> {
        self.inner.views_poses()
    }

    /// Index 0 corresponds to the left hand, index 1 corresponds to the right hand.
    pub fn hands_pose(&self) -> [Option<XrPose>; 2] {
        self.inner.hands_pose()
    }

    /// Index 0 corresponds to the left hand, index 1 corresponds to the right hand.
    pub fn hands_skeleton_pose(&self) -> [Option<Vec<XrJointPose>>; 2] {
        self.inner.hands_skeleton_pose()
    }

    /// Returns poses that can be used to render a target ray or cursor. The ray is along -Z. The
    /// behavior is vendor-specific. Index 0 corresponds to the left hand, index 1 corresponds to
    /// the right hand.
    pub fn hand_target_ray(&self) -> [Option<XrPose>; 2] {
        self.inner.hands_target_ray()
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

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub enum XrHandType {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum XrActionType {
    /// Convenience type that groups click, touch and value actions for a single button.  
    /// The last segment of the path (`/click`, `/touch` or `/value`) must be omitted.
    Button {
        touch: bool,
    },

    Binary,

    Scalar,

    /// Convenience type that groups x and y axes for a touchpad or thumbstick action.  
    /// The last segment of the path (`/x` or `/y`) must be omitted.
    Vec2D,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum XrActionState {
    Button { state: XrButtonState, value: f32 },
    Binary(bool),
    Scalar(f32),
    Vec2D(Vec2),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct XrActionDescriptor {
    pub name: String,
    pub action_type: XrActionType,
}

/// List bindings related to a single interaction profile. `tracked` and `has_haptics` can always be
/// set to false but if they are set to true and the interaction profile does not support them, the
/// the profile will be disabled completely.
pub struct XrProfileDescriptor {
    pub profile: String,
    pub bindings: Vec<(XrActionDescriptor, String)>,
    pub tracked: bool,
    pub has_haptics: bool,
}

pub struct XrActionSet {
    current_states: HashMap<String, XrActionState>,
    previous_states: HashMap<String, XrActionState>,
}

impl XrActionSet {
    pub fn state(&self, action: &str) -> Option<XrActionState> {
        self.current_states.get(action).cloned()
    }

    pub fn button_state(&self, action: &str) -> XrButtonState {
        if let Some(XrActionState::Button { state, .. }) = self.current_states.get(action) {
            *state
        } else {
            XrButtonState::Default
        }
    }

    pub fn button_touched(&self, action: &str) -> bool {
        if let Some(XrActionState::Button { state, .. }) = self.current_states.get(action) {
            *state != XrButtonState::Default
        } else {
            false
        }
    }

    pub fn button_pressed(&self, action: &str) -> bool {
        if let Some(XrActionState::Button { state, .. }) = self.current_states.get(action) {
            *state == XrButtonState::Pressed
        } else {
            false
        }
    }

    fn button_states(&self, action: &str) -> Option<(XrButtonState, XrButtonState)> {
        if let (
            Some(XrActionState::Button {
                state: current_state,
                ..
            }),
            Some(XrActionState::Button {
                state: previous_state,
                ..
            }),
        ) = (
            self.current_states.get(action),
            self.previous_states.get(action),
        ) {
            Some((*current_state, *previous_state))
        } else {
            None
        }
    }

    pub fn button_just_touched(&self, action: &str) -> bool {
        self.button_states(action)
            .map(|(cur, prev)| cur != XrButtonState::Default && prev == XrButtonState::Default)
            .unwrap_or(false)
    }

    pub fn button_just_untouched(&self, action: &str) -> bool {
        self.button_states(action)
            .map(|(cur, prev)| cur == XrButtonState::Default && prev != XrButtonState::Default)
            .unwrap_or(false)
    }

    pub fn button_just_pressed(&self, action: &str) -> bool {
        self.button_states(action)
            .map(|(cur, prev)| cur == XrButtonState::Pressed && prev != XrButtonState::Pressed)
            .unwrap_or(false)
    }

    pub fn button_just_unpressed(&self, action: &str) -> bool {
        self.button_states(action)
            .map(|(cur, prev)| cur != XrButtonState::Pressed && prev == XrButtonState::Pressed)
            .unwrap_or(false)
    }

    pub fn binary_value(&self, action: &str) -> bool {
        if let Some(XrActionState::Binary(value)) = self.current_states.get(action) {
            *value
        } else {
            self.button_pressed(action)
        }
    }

    pub fn scalar_value(&self, action: &str) -> f32 {
        if let Some(XrActionState::Scalar(value) | XrActionState::Button { value, .. }) =
            self.current_states.get(action)
        {
            *value
        } else {
            0.0
        }
    }

    pub fn vec_2d_value(&self, action: &str) -> Vec2 {
        if let Some(XrActionState::Vec2D(value)) = self.current_states.get(action) {
            *value
        } else {
            Vec2::ZERO
        }
    }

    pub fn set(&mut self, states: HashMap<String, XrActionState>) {
        self.previous_states = self.current_states.clone();
        self.current_states = states;
    }

    pub fn clear(&mut self) {
        self.current_states.clear();
        self.previous_states.clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum XrVibrationEventType {
    Apply {
        duration: Duration,
        frequency: f32,
        amplitude: f32,
    },
    Stop,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct XrVibrationEvent {
    pub hand: XrHandType,
    pub command: XrVibrationEventType,
}

/// Active interaction profiles. The format is backend-specific. They can be used to choose the
/// controller 3D models to display.
/// Note: in case skeletal hand tracking is active, the profiles still point to controller profiles.
/// The correct 3D model to display can be decided depending on if skeletal hand tracking data is
/// available or not.
#[derive(Clone, PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct XrProfiles {
    pub left_hand: Option<String>,
    pub right_hand: Option<String>,
}
