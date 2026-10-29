//! Utilities to manipulate on Transform.

use bevy_math::{DQuat, DVec3};
use bevy_transform::prelude::*;

/// Read the translation and rotation of an entity.
pub fn read_transform(cam_transform: &Transform) -> Option<(DVec3, DQuat)> {
    Some((
        cam_transform.translation.as_dvec3(),
        cam_transform.rotation.as_dquat(),
    ))
}

/// Apply a movement delta to an entity.
pub fn apply_delta(cam_transform: &mut Transform, delta_translation: DVec3, delta_rotation: DQuat) {
    let delta_transform = Transform::from_translation(delta_translation.as_vec3())
        .with_rotation(delta_rotation.as_quat());
    *cam_transform = cam_transform.mul_transform(delta_transform);
}
