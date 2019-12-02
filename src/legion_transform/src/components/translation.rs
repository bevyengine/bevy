use crate::math::{Translation3, Vector3};
use shrinkwraprs::Shrinkwrap;

#[derive(Shrinkwrap, Debug, PartialEq, Clone, Copy)]
#[shrinkwrap(mutable)]
pub struct Translation(pub Translation3<f32>);

impl Translation {
    #[inline(always)]
    pub fn identity() -> Self {
        Self(Translation3::identity())
    }

    #[inline(always)]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Translation3::new(x, y, z))
    }
}

impl Default for Translation {
    fn default() -> Self {
        Self::identity()
    }
}

impl From<Vector3<f32>> for Translation {
    fn from(translation: Vector3<f32>) -> Self {
        Self(Translation3::from(translation))
    }
}

impl From<Translation3<f32>> for Translation {
    fn from(translation: Translation3<f32>) -> Self {
        Self(translation)
    }
}
