use crate::{
    conversion::{to_quat, to_vec3},
    InteractionContext, OpenXrSession,
};
use bevy_math::Vec3;
use bevy_xr::{
    interaction::implementation::XrTrackingSourceBackend, XrHandType, XrJointPose, XrPose,
    XrReferenceSpaceType, XrRigidTransform,
};
use openxr as xr;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

pub fn openxr_pose_to_rigid_transform(pose: xr::Posef) -> XrRigidTransform {
    XrRigidTransform {
        position: to_vec3(pose.position),
        orientation: to_quat(pose.orientation),
    }
}

/// Usage: `prediction_time` must be the same time used to obtain `pose`.
pub fn openxr_pose_to_corrected_rigid_transform(
    pose: xr::Posef,
    reference: &OpenXrTrackingReference,
    prediction_time: xr::Time,
) -> XrRigidTransform {
    let transform = openxr_pose_to_rigid_transform(pose);

    if reference.change_time.as_nanos() > prediction_time.as_nanos() {
        reference.previous_pose_offset * transform
    } else {
        transform
    }
}

pub fn predict_pose(
    space: &xr::Space,
    reference: &OpenXrTrackingReference,
    prediction_time: xr::Time,
) -> Option<XrPose> {
    let (location, velocity) = space.relate(&reference.space, prediction_time).ok()?;
    if !location.location_flags.contains(
        xr::SpaceLocationFlags::ORIENTATION_VALID | xr::SpaceLocationFlags::POSITION_VALID,
    ) {
        return None;
    }

    let linear_velocity = velocity
        .velocity_flags
        .contains(xr::SpaceVelocityFlags::LINEAR_VALID)
        .then(|| to_vec3(velocity.linear_velocity));
    let angular_velocity = velocity
        .velocity_flags
        .contains(xr::SpaceVelocityFlags::ANGULAR_VALID)
        .then(|| to_vec3(velocity.angular_velocity));

    Some(XrPose {
        transform: openxr_pose_to_corrected_rigid_transform(
            location.pose,
            reference,
            prediction_time,
        ),
        linear_velocity,
        angular_velocity,
        emulated_position: location
            .location_flags
            .contains(xr::SpaceLocationFlags::POSITION_TRACKED),
    })
}

pub fn predict_hand_skeleton_pose(
    hand_tracker: &xr::HandTracker,
    reference: &OpenXrTrackingReference,
    prediction_time: xr::Time,
) -> Option<Vec<XrJointPose>> {
    let (poses, velocities) = reference
        .space
        .relate_hand_joints(hand_tracker, prediction_time)
        .ok()
        .flatten()?;

    Some(
        poses
            .iter()
            .zip(velocities.iter())
            .skip(1) // exclude palm joint
            .map(|(location, velocity)| {
                let linear_velocity = velocity
                    .velocity_flags
                    .contains(xr::SpaceVelocityFlags::LINEAR_VALID)
                    .then(|| to_vec3(velocity.linear_velocity));
                let angular_velocity = velocity
                    .velocity_flags
                    .contains(xr::SpaceVelocityFlags::ANGULAR_VALID)
                    .then(|| to_vec3(velocity.angular_velocity));

                XrJointPose {
                    pose: XrPose {
                        transform: openxr_pose_to_corrected_rigid_transform(
                            location.pose,
                            reference,
                            prediction_time,
                        ),
                        linear_velocity,
                        angular_velocity,
                        emulated_position: location
                            .location_flags
                            .contains(xr::SpaceLocationFlags::POSITION_TRACKED),
                    },
                    radius: location.radius,
                }
            })
            .collect(),
    )
}

pub struct OpenXrTrackingReference {
    pub space_type: xr::ReferenceSpaceType,
    pub space: xr::Space,
    pub change_time: xr::Time,
    pub previous_pose_offset: XrRigidTransform,
}

pub struct OpenXrTrackingContext {
    pub reference: RwLock<OpenXrTrackingReference>,
    pub grip_spaces: [xr::Space; 2],
    pub target_ray_spaces: [xr::Space; 2],
    pub hand_trackers: Option<[xr::HandTracker; 2]>,
}

impl OpenXrTrackingContext {
    pub(crate) fn new(
        instance: &xr::Instance,
        system: xr::SystemId,
        interaction_context: &InteractionContext,
        session: OpenXrSession,
    ) -> Self {
        // Select the most immersive type available
        let reference = [
            xr::ReferenceSpaceType::STAGE,
            xr::ReferenceSpaceType::LOCAL,
            xr::ReferenceSpaceType::VIEW,
        ]
        .iter()
        .cloned()
        .find_map(|space_type| {
            let space = session
                .create_reference_space(space_type, xr::Posef::IDENTITY)
                .ok()?;

            Some(OpenXrTrackingReference {
                space_type,
                space,
                change_time: xr::Time::from_nanos(0),
                previous_pose_offset: XrRigidTransform::default(),
            })
        })
        .unwrap();

        let grip_spaces = [
            interaction_context
                .grip_actions
                .get(&XrHandType::Left)
                .unwrap()
                .create_space((*session).clone(), xr::Path::NULL, xr::Posef::IDENTITY)
                .unwrap(),
            interaction_context
                .grip_actions
                .get(&XrHandType::Right)
                .unwrap()
                .create_space((*session).clone(), xr::Path::NULL, xr::Posef::IDENTITY)
                .unwrap(),
        ];
        let target_ray_spaces = [
            interaction_context
                .target_ray_actions
                .get(&XrHandType::Left)
                .unwrap()
                .create_space((*session).clone(), xr::Path::NULL, xr::Posef::IDENTITY)
                .unwrap(),
            interaction_context
                .target_ray_actions
                .get(&XrHandType::Right)
                .unwrap()
                .create_space((*session).clone(), xr::Path::NULL, xr::Posef::IDENTITY)
                .unwrap(),
        ];
        let hand_trackers = instance.supports_hand_tracking(system).unwrap().then(|| {
            [
                session.create_hand_tracker(xr::Hand::LEFT).unwrap(),
                session.create_hand_tracker(xr::Hand::RIGHT).unwrap(),
            ]
        });

        Self {
            reference: RwLock::new(reference),
            grip_spaces,
            target_ray_spaces,
            hand_trackers,
        }
    }
}

pub(crate) struct TrackingSource {
    pub view_type: xr::ViewConfigurationType,
    pub action_set: Arc<Mutex<xr::ActionSet>>,
    pub session: OpenXrSession,
    pub context: Arc<OpenXrTrackingContext>,
    pub next_vsync_time: Arc<RwLock<xr::Time>>,
}

impl XrTrackingSourceBackend for TrackingSource {
    fn reference_space_type(&self) -> XrReferenceSpaceType {
        match self.context.reference.read().space_type {
            xr::ReferenceSpaceType::VIEW => XrReferenceSpaceType::Viewer,
            xr::ReferenceSpaceType::LOCAL => XrReferenceSpaceType::Local,
            xr::ReferenceSpaceType::STAGE => XrReferenceSpaceType::Stage,
            _ => unreachable!(),
        }
    }

    fn set_reference_space_type(&self, mode: XrReferenceSpaceType) -> bool {
        let reference_type = match mode {
            XrReferenceSpaceType::Viewer => xr::ReferenceSpaceType::VIEW,
            XrReferenceSpaceType::Local => xr::ReferenceSpaceType::LOCAL,
            XrReferenceSpaceType::Stage => xr::ReferenceSpaceType::STAGE,
        };
        if let Ok(space) = self
            .session
            .create_reference_space(reference_type, xr::Posef::IDENTITY)
        {
            let reference = &mut self.context.reference.write();
            reference.space_type = reference_type;
            reference.space = space;

            true
        } else {
            false
        }
    }

    fn bounds_geometry(&self) -> Option<Vec<Vec3>> {
        let rect = self
            .session
            .reference_space_bounds_rect(self.context.reference.read().space_type)
            .ok()
            .flatten()?;
        let half_width = rect.width / 2_f32;
        let half_height = rect.height / 2_f32;

        Some(vec![
            Vec3::new(-half_width, 0_f32, -half_height),
            Vec3::new(half_width, 0_f32, -half_height),
            Vec3::new(half_width, 0_f32, half_height),
            Vec3::new(-half_width, 0_f32, half_height),
        ])
    }

    fn views_poses(&self) -> Vec<XrPose> {
        // NB: hold the lock
        let action_set = &*self.action_set.lock();

        self.session.sync_actions(&[action_set.into()]).unwrap();
        let reference = &self.context.reference.read();
        let display_time = *self.next_vsync_time.read();

        let (flags, views) = self
            .session
            .locate_views(self.view_type, display_time, &reference.space)
            .unwrap();

        views
            .into_iter()
            .map(|view| XrPose {
                transform: openxr_pose_to_corrected_rigid_transform(
                    view.pose,
                    reference,
                    display_time,
                ),
                linear_velocity: None,
                angular_velocity: None,
                emulated_position: flags.contains(xr::ViewStateFlags::POSITION_TRACKED),
            })
            .collect()
    }

    fn hands_pose(&self) -> [Option<XrPose>; 2] {
        // NB: hold the lock
        let action_set = &*self.action_set.lock();

        self.session.sync_actions(&[action_set.into()]).unwrap();
        let reference = &self.context.reference.read();
        let display_time = *self.next_vsync_time.read();

        [
            predict_pose(&self.context.grip_spaces[0], reference, display_time),
            predict_pose(&self.context.grip_spaces[1], reference, display_time),
        ]
    }

    fn hands_skeleton_pose(&self) -> [Option<Vec<XrJointPose>>; 2] {
        if let Some(hand_trackers) = &self.context.hand_trackers {
            // NB: hold the lock
            let action_set = &*self.action_set.lock();

            self.session.sync_actions(&[action_set.into()]).unwrap();
            let display_time = *self.next_vsync_time.read();
            let reference = &self.context.reference.read();

            [
                predict_hand_skeleton_pose(&hand_trackers[0], reference, display_time),
                predict_hand_skeleton_pose(&hand_trackers[1], reference, display_time),
            ]
        } else {
            [None, None]
        }
    }

    fn hands_target_ray(&self) -> [Option<XrPose>; 2] {
        // NB: hold the lock
        let action_set = &*self.action_set.lock();

        self.session.sync_actions(&[action_set.into()]).unwrap();
        let display_time = *self.next_vsync_time.read();
        let reference = &self.context.reference.read();

        [
            predict_pose(&self.context.target_ray_spaces[0], reference, display_time),
            predict_pose(&self.context.target_ray_spaces[1], reference, display_time),
        ]
    }

    fn viewer_target_ray(&self) -> XrPose {
        let poses = self.views_poses();
        let poses_count = poses.len() as f32;

        // fixme: this is wrong when views point outwards (Pimax)
        // todo: quaternion averaging
        let orientation = poses[0].transform.orientation;

        let position = poses
            .iter()
            .map(|pose| pose.transform.position)
            .reduce(std::ops::Add::add)
            .unwrap()
            / poses_count;

        XrPose {
            transform: XrRigidTransform {
                position,
                orientation,
            },
            linear_velocity: None,
            angular_velocity: None,
            emulated_position: poses[0].emulated_position,
        }
    }
}
