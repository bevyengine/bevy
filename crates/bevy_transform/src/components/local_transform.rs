use crate::math::Mat4;
use bevy_property::Properties;
use shrinkwraprs::Shrinkwrap;
use std::fmt;

#[derive(Shrinkwrap, Debug, PartialEq, Clone, Copy, Properties)]
#[shrinkwrap(mutable)]
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
