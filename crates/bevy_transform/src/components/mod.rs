mod global_transform;
mod global_transform2d;
mod transform;
mod transform2d;

pub use global_transform::*;
pub use global_transform2d::*;
pub use transform::*;
pub use transform2d::*;

use bevy_ecs::query::{AnyOf, WorldQuery};

/// A [`WorldQuery`] that returns the [`GlobalTransform`] if present, otherwise returns the [`GlobalTransform2d`] as a [`GlobalTransform`].
#[derive(WorldQuery)]
pub struct AnyGlobalTransform {
    transforms: AnyOf<(&'static GlobalTransform, &'static GlobalTransform2d)>,
}

impl AnyGlobalTransformItem<'_> {
    /// Returns the [`GlobalTransform`] if present, otherwise returns the [`GlobalTransform2d`] as a [`GlobalTransform`].
    pub fn get(&self) -> GlobalTransform {
        match self.transforms {
            (Some(&transform_3d), _) => transform_3d,
            (None, Some(&transform_2d)) => transform_2d.into(),
            (None, None) => unreachable!(),
        }
    }
}
