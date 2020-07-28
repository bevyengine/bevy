use bevy_math::Mat4;
use bevy_property::Properties;
use std::{
    fmt,
    ops::{Deref, DerefMut},
};

#[derive(Debug, PartialEq, Clone, Copy, Properties)]
pub struct LocalTransform(pub Mat4);

impl LocalTransform {
    pub fn identity() -> Self {
        Self(Mat4::identity())
    }
}

impl Default for LocalTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl fmt::Display for LocalTransform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for LocalTransform {
    type Target = Mat4;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LocalTransform {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
