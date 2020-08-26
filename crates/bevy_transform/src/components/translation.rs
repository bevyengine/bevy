use bevy_math::Vec3;
use bevy_property::Properties;
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq, Copy, Clone, Properties)]
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
        Self(translation)
    }
}

impl Deref for Translation {
    type Target = Vec3;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Translation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
