use crate::math::Mat4;
use bevy_property::Properties;
use shrinkwraprs::Shrinkwrap;
use std::fmt;

#[derive(Shrinkwrap, Debug, PartialEq, Clone, Copy, Properties)]
#[shrinkwrap(mutable)]
pub struct LocalToParent(pub Mat4);

impl LocalToParent {
    pub fn identity() -> Self {
        Self(Mat4::identity())
    }
}

impl Default for LocalToParent {
    fn default() -> Self {
        Self::identity()
    }
}

impl fmt::Display for LocalToParent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
