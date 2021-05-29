use glam::{Quat, Vec2, Vec3};
use std::time::Duration;

use crate::XrDuration;

// Note: indices follow WebXR convention. OpenXR's palm joint is missing, but it can be retrieved
// using `XrState::hand_motion(..., HandAction::Grip)`.
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

pub struct Pose {
    pub position: Option<Vec3>,
    pub orientation: Option<Quat>,
}

impl Pose {
    pub fn is_tracked(&self) -> bool {
        self.position.is_some() || self.orientation.is_some()
    }
}

pub struct Motion {
    pub pose: Pose,
    pub occluded: bool,
    pub linear_velocity: Option<Vec3>,
    pub angular_velocity: Option<Vec3>,
}

impl Motion {
    pub fn is_tracked(&self) -> bool {
        self.pose.is_tracked()
    }
}

#[derive(Clone)]
pub enum TrackingReferenceMode {
    /// The coordinate system (position and orientation) is set as the headset pose at startup or
    /// after a recenter. This should be used only for experiences where the user is laid down.
    Tilted,

    /// The coordinate system (position and gravity-aligned orientation) is calculated from the
    /// headset pose at startup or after a recenter. This is for seated experiences.
    GravityAligned,

    /// The coordinate system (position and orientation) corresponds to the center of a rectangle at
    /// floor level, with +Y up. This is for stading or room-scale experiences.
    Stage,
}

impl Default for TrackingReferenceMode {
    fn default() -> Self {
        Self::GravityAligned
    }
}

pub enum HandType {
    Left,
    Right,
}

pub enum HandAction {
    /// Position at the center of the palm, +X exiting the palm, -Z up the grip of a virtual gun.
    Grip,

    /// Runtime-dependent. For controllers, -Z exists the barrel of a virtual gun. For Oculus hand
    /// tracking, -Z corresponds to the ray from the shoulder to the hand.
    Aim,
}

pub enum BinaryEventType {
    Toggled,
    Unchanged,
}

impl Default for BinaryEventType {
    fn default() -> Self {
        Self::Unchanged
    }
}

#[derive(Default)]
pub struct BinaryEvent {
    pub value: bool,
    pub event: BinaryEventType,
}

/// Oculus-Touch-like virtual controller buttons. Different types of controller inputs get mapped to
/// these inputs for egonomics sake. Keep in mind that for some vendor-specific controllers some
/// buttons will remain unused, while for others the behavior could change (for example the trigger
/// value might be only 0 or 1). Many controllers do not support touch inputs so they should only be
/// used for visual feedback.
#[derive(Default)]
pub struct GenericControllerPairButtons {
    pub menu_click: BinaryEvent,
    pub left_hand: GenericControllerButtons,
    pub right_hand: GenericControllerButtons,
}

#[derive(Default)]
pub struct GenericControllerButtons {
    pub primary_click: BinaryEvent,
    pub primary_touch: BinaryEvent,
    pub secondary_click: BinaryEvent,
    pub secondary_touch: BinaryEvent,
    pub trigger_value: f32,
    pub trigger_touch: BinaryEvent,
    pub squeeze_value: f32,
    pub directional_value: Vec2,
    pub directional_click: BinaryEvent,
    pub directional_touch: BinaryEvent,
}

pub enum Vibration {
    Apply {
        duration: XrDuration,
        frequency: f32,
        amplitude: f32,
    },
    Stop,
}

pub struct GenericControllerVibration {
    pub hand: HandType,
    pub action: Vibration,
}
