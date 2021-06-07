pub mod interaction;
pub mod presentation;

use bevy_app::{AppBuilder, Plugin, StartupStage};
use bevy_ecs::prelude::{IntoExclusiveSystem, World};
use interaction::{
    GenericControllerPairButtons, GenericControllerVibration, HandAction, HandType, Motion, Pose,
    Position, TrackingReferenceMode, XR_HAND_JOINT_COUNT,
};

pub struct XrDuration(i64);

impl XrDuration {
    pub fn from_nanos(nanos: i64) -> Self {
        Self(nanos)
    }

    pub fn as_nanos(&self) -> i64 {
        self.0
    }
}

pub struct XrTime(i64);

impl XrTime {
    pub fn from_nanos(x: i64) -> Self {
        Self(x)
    }

    pub fn as_nanos(&self) -> i64 {
        self.0
    }
}

impl std::ops::Add<XrDuration> for XrTime {
    type Output = XrTime;

    fn add(self, rhs: XrDuration) -> XrTime {
        XrTime(self.0 + rhs.0)
    }
}

impl std::ops::Sub<XrDuration> for XrTime {
    type Output = XrTime;

    fn sub(self, rhs: XrDuration) -> XrTime {
        XrTime(self.0 - rhs.0)
    }
}

impl std::ops::Sub for XrTime {
    type Output = XrDuration;

    fn sub(self, rhs: XrTime) -> XrDuration {
        XrDuration(self.0 - rhs.0)
    }
}

pub mod implementation {
    use super::{HandAction, HandType, TrackingReferenceMode, XrDuration, XrTime};
    use crate::interaction::{Motion, Pose, XR_HAND_JOINT_COUNT};

    pub trait XrStateBackend: Send + Sync {
        fn set_tracking_reference_mode(&self, mode: TrackingReferenceMode) -> bool;
        fn views_poses(&self, time: XrTime) -> Vec<Pose>;
        fn hand_motion(&self, hand_type: HandType, action: HandAction, time: XrTime) -> Motion;
        fn hand_skeleton_motion(
            &self,
            hand_type: HandType,
            time: XrTime,
        ) -> [Motion; XR_HAND_JOINT_COUNT];
        fn generic_tracker_motion(&self, index: usize, time: XrTime) -> Motion;
        fn predicted_display_time(&self) -> XrTime;
        fn predicted_display_period(&self) -> XrDuration;
        fn should_render(&self) -> bool;
    }
}
use implementation::XrStateBackend;

pub struct XrState {
    inner: Box<dyn XrStateBackend>,
}
impl XrState {
    // Returns true if the tracking mode has been set correctly. If false is returned the tracking
    // mode is not supported and another one must be chosen.
    pub fn set_tracking_reference_mode(&self, mode: TrackingReferenceMode) -> bool {
        self.inner.set_tracking_reference_mode(mode)
    }

    /// Average pose of all views. Correspond roughly to the middle point between the eyes or the
    /// camera lens position for AR.
    pub fn viewer_pose(&self) -> Pose {
        self.viewer_pose_at_time(self.inner.predicted_display_time())
    }

    pub fn viewer_pose_at_time(&self, time: XrTime) -> Pose {
        let poses = self.inner.views_poses(time);

        let orientation = poses.iter().find_map(|pose| pose.orientation.clone());

        let position = poses
            .iter()
            .filter_map(|pose| pose.position.clone())
            .reduce(|pos1, pos2| Position {
                value: pos1.value + pos2.value,
                tracked: pos1.tracked,
            })
            .map(|sum| Position {
                value: sum.value / poses.len() as f32,
                tracked: sum.tracked,
            });

        Pose {
            position,
            orientation,
        }
    }

    pub fn views_poses(&self) -> Vec<Pose> {
        self.inner.views_poses(self.inner.predicted_display_time())
    }

    pub fn views_poses_at_time(&self, time: XrTime) -> Vec<Pose> {
        self.inner.views_poses(time)
    }

    pub fn hand_motion(&self, hand_type: HandType, action: HandAction) -> Motion {
        self.inner
            .hand_motion(hand_type, action, self.inner.predicted_display_time())
    }

    pub fn hand_motion_at_time(
        &self,
        hand_type: HandType,
        action: HandAction,
        time: XrTime,
    ) -> Motion {
        self.inner.hand_motion(hand_type, action, time)
    }

    pub fn hand_skeleton_motion(&self, hand_type: HandType) -> [Motion; XR_HAND_JOINT_COUNT] {
        self.inner
            .hand_skeleton_motion(hand_type, self.inner.predicted_display_time())
    }

    pub fn hand_skeleton_motion_at_time(
        &self,
        hand_type: HandType,
        time: XrTime,
    ) -> [Motion; XR_HAND_JOINT_COUNT] {
        self.inner.hand_skeleton_motion(hand_type, time)
    }

    pub fn generic_tracker_motion(&self, index: usize) -> Motion {
        self.inner
            .generic_tracker_motion(index, self.inner.predicted_display_time())
    }

    pub fn generic_tracker_motion_at_time(&self, index: usize, time: XrTime) -> Motion {
        self.inner.generic_tracker_motion(index, time)
    }

    pub fn predicted_display_time(&self) -> XrTime {
        self.inner.predicted_display_time()
    }

    pub fn predicted_display_period(&self) -> XrDuration {
        self.inner.predicted_display_period()
    }

    pub fn should_render(&self) -> bool {
        self.inner.should_render()
    }

    // future extensions:
    // * eye tracking
    // * lower face tracking
    // * AR face tracking
}

#[derive(Clone)]
pub enum ViewerType {
    PreferHeadMounted,
    PreferHandheld,
}

impl Default for ViewerType {
    fn default() -> Self {
        Self::PreferHandheld
    }
}

#[derive(Clone)]
pub enum BlendMode {
    PreferVR,
    AR,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::PreferVR
    }
}

#[derive(Clone)]
pub enum XrMode {
    Display {
        viewer: ViewerType,
        blend: BlendMode,
    },
    OnlyTracking,
}

impl Default for XrMode {
    fn default() -> Self {
        Self::Display {
            viewer: Default::default(),
            blend: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct XrConfig {
    pub mode: XrMode,
    pub enable_generic_controllers: bool,
}

impl Default for XrConfig {
    fn default() -> Self {
        Self {
            mode: Default::default(),
            enable_generic_controllers: true,
        }
    }
}

#[derive(Default)]
pub struct XrPlugin;

impl Plugin for XrPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let config = if let Some(config) = app.world().get_resource::<XrConfig>() {
            config.clone()
        } else {
            return;
        };

        if config.enable_generic_controllers {
            app.add_event::<GenericControllerPairButtons>()
                .add_event::<GenericControllerVibration>();
        }

        app.add_system_to_stage(
            StartupStage::PreStartup,
            add_xr_state_resource.exclusive_system(),
        );
    }
}

fn add_xr_state_resource(world: &mut World) {
    let state_backend = world.remove_resource::<Box<dyn XrStateBackend>>().unwrap();

    world.insert_resource(XrState {
        inner: state_backend,
    });
}

pub struct XrFov {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}
