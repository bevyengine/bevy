pub mod components;
pub mod hierarchy;
pub mod transform_propagate_system;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{components::*, hierarchy::*, TransformBundle, TransformPlugin};
}

use bevy_app::prelude::*;
use bevy_ecs::{
    bundle::Bundle,
    schedule::{ParallelSystemDescriptorCoercion, SystemLabel},
};
use bevy_math::{Mat4, Quat, Vec3};
use prelude::{parent_update_system, Children, GlobalTransform, Parent, PreviousParent, Transform};

#[derive(Default, Bundle, Clone, Debug)]
pub struct TransformBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl TransformBundle {
    /// Creates a new [`TransformBundle`] at the position `(x, y, z)`. In 2d, the `z` component
    /// is used for z-ordering elements: higher `z`-value will be in front of lower
    /// `z`-value.
    #[inline]
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        TransformBundle {
            transform: Transform::from_xyz(x, y, z),
            ..Default::default()
        }
    }

    /// Creates a new identity [`TransformBundle`], with no translation, rotation, and a scale of 1
    /// on all axes.
    #[inline]
    pub const fn identity() -> Self {
        TransformBundle {
            transform: Transform::identity(),
            global_transform: GlobalTransform::identity(),
        }
    }

    /// Extracts the translation, rotation, and scale from `matrix`. It must be a 3d affine
    /// transformation matrix.
    #[inline]
    pub fn from_matrix(matrix: Mat4) -> Self {
        TransformBundle {
            transform: Transform::from_matrix(matrix),
            ..Default::default()
        }
    }

    /// Creates a new [`TransformBundle`], with `translation`. Rotation will be 0 and scale 1 on
    /// all axes.
    #[inline]
    pub fn from_translation(translation: Vec3) -> Self {
        TransformBundle {
            transform: Transform::from_translation(translation),
            ..Default::default()
        }
    }

    /// Creates a new [`TransformBundle`], with `rotation`. Translation will be 0 and scale 1 on
    /// all axes.
    #[inline]
    pub fn from_rotation(rotation: Quat) -> Self {
        TransformBundle {
            transform: Transform::from_rotation(rotation),
            ..Default::default()
        }
    }

    /// Creates a new [`TransformBundle`], with `scale`. Translation will be 0 and rotation 0 on
    /// all axes.
    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        TransformBundle {
            transform: Transform::from_scale(scale),
            ..Default::default()
        }
    }
}

impl From<Transform> for TransformBundle {
    fn from(transform: Transform) -> Self {
        TransformBundle {
            transform,
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct TransformPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum TransformSystem {
    TransformPropagate,
    ParentUpdate,
}

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Children>()
            .register_type::<Parent>()
            .register_type::<PreviousParent>()
            .register_type::<Transform>()
            .register_type::<GlobalTransform>()
            // add transform systems to startup so the first update is "correct"
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                parent_update_system.label(TransformSystem::ParentUpdate),
            )
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                transform_propagate_system::transform_propagate_system
                    .label(TransformSystem::TransformPropagate)
                    .after(TransformSystem::ParentUpdate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                parent_update_system.label(TransformSystem::ParentUpdate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                transform_propagate_system::transform_propagate_system
                    .label(TransformSystem::TransformPropagate)
                    .after(TransformSystem::ParentUpdate),
            );
    }
}
