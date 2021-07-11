use bevy_xr::{
    interaction::implementation::XrTrackingStateBackend, HandType, XrPose, XrReferenceSpaceType,
    XrRigidTransform, XR_HAND_JOINT_COUNT,
};
use openxr as xr;
use std::sync::Mutex;

use crate::conversion::{to_quat, to_vec3};

pub struct Spaces {
    pub reference: Mutex<(XrReferenceSpaceType, xr::Space)>,
    pub left_grip: xr::Space,
    pub right_grip: xr::Space,
    pub left_target_ray: xr::Space,
    pub right_target_ray: xr::Space,
}

pub(crate) struct TrackingState {
    view_type: xr::ViewConfigurationType,
    action_set: xr::ActionSet,
    pub(crate) session: xr::Session<xr::AnyGraphics>,
    spaces: Spaces,
    pub(crate) next_vsync_time: xr::Time,
}

impl XrTrackingStateBackend for TrackingState {
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
