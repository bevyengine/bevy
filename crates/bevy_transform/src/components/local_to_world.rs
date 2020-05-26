use crate::math::Mat4;
use shrinkwraprs::Shrinkwrap;
use std::fmt;
use bevy_property::Properties;

#[derive(Shrinkwrap, Debug, PartialEq, Clone, Copy, Properties)]
#[shrinkwrap(mutable)]
pub struct LocalToWorld(pub Mat4);

impl LocalToWorld {
    #[inline(always)]
    pub fn identity() -> Self {
        Self(Mat4::identity())
    }
}

impl Default for LocalToWorld {
    fn default() -> Self {
        Self::identity()
    }
}

impl fmt::Display for LocalToWorld {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
