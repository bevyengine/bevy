use crate::math::Vec3;
use bevy_property::Properties;
use shrinkwraprs::Shrinkwrap;

#[derive(Shrinkwrap, Debug, PartialEq, Clone, Copy, Properties)]
#[shrinkwrap(mutable)]
pub struct Translation(pub Vec3);

impl Translation {
    #[inline(always)]
    pub fn identity() -> Self {
        Self(Vec3::default())
    }

    #[inline(always)]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }
}

impl Default for Translation {
    fn default() -> Self {
        Self::identity()
    }
}

impl From<Vec3> for Translation {
    fn from(translation: Vec3) -> Self {
        Self(Vec3::from(translation))
    }
}
