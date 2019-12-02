use crate::math::Matrix4;
use shrinkwraprs::Shrinkwrap;
use std::fmt;

#[derive(Shrinkwrap, Debug, PartialEq, Clone, Copy)]
#[shrinkwrap(mutable)]
pub struct LocalToParent(pub Matrix4<f32>);

impl LocalToParent {
    pub fn identity() -> Self {
        Self(Matrix4::identity())
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
