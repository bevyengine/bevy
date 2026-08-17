//! Defines how the camera controller reads and writes transform data.
//!
//! By default, [`TransformAdapter`] reads and writes Bevy's built-in [`Transform`] component.
//! To use a different transform representation (e.g. a 64-bit transform), replace this resource
//! with custom `read` and `apply_delta` callbacks.

use bevy_ecs::prelude::*;
use bevy_log::error_once;
use bevy_math::{DQuat, DVec3};
use bevy_transform::prelude::*;

/// Resource that defines how the camera controller reads and writes transform data.
///
/// By default, this reads and writes Bevy's built-in [`Transform`] component. Replace this
/// resource to use a different transform component (e.g. a 64-bit transform) via [`Self::new`].
#[derive(Resource)]
pub struct TransformAdapter {
    read_fn: Box<dyn Fn(&EntityRef) -> Option<(DVec3, DQuat)> + Send + Sync>,
    apply_delta_fn: Box<dyn Fn(&mut EntityMut, DVec3, DQuat) + Send + Sync>,
}

impl TransformAdapter {
    /// Create a new `TransformAdapter` with custom read and apply_delta callbacks.
    pub fn new(
        read: impl Fn(&EntityRef) -> Option<(DVec3, DQuat)> + Send + Sync + 'static,
        apply_delta: impl Fn(&mut EntityMut, DVec3, DQuat) + Send + Sync + 'static,
    ) -> Self {
        Self {
            read_fn: Box::new(read),
            apply_delta_fn: Box::new(apply_delta),
        }
    }

    /// Read the translation and rotation of an entity.
    pub fn read(&self, entity: &EntityRef) -> Option<(DVec3, DQuat)> {
        (self.read_fn)(entity)
    }

    /// Apply a movement delta to an entity.
    pub fn apply_delta(
        &self,
        entity: &mut EntityMut,
        delta_translation: DVec3,
        delta_rotation: DQuat,
    ) {
        (self.apply_delta_fn)(entity, delta_translation, delta_rotation)
    }
}

impl Default for TransformAdapter {
    fn default() -> Self {
        Self::new(
            |entity| {
                let Some(cam_transform) = entity.get::<Transform>() else {
                    error_once!("Unable to retrieve Transform from PanOrbitCamera entity.");
                    return None;
                };
                Some((
                    cam_transform.translation.as_dvec3(),
                    cam_transform.rotation.as_dquat(),
                ))
            },
            |entity, delta_translation, delta_rotation| {
                let Some(mut cam_transform) = entity.get_mut::<Transform>() else {
                    error_once!("Unable to retrieve Transform from PanOrbitCamera entity.");
                    return;
                };
                let delta_transform = Transform::from_translation(delta_translation.as_vec3())
                    .with_rotation(delta_rotation.as_quat());
                *cam_transform = cam_transform.mul_transform(delta_transform);
            },
        )
    }
}
