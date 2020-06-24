use crate::math::Quat;
use bevy_property::Properties;
use shrinkwraprs::Shrinkwrap;

#[derive(Shrinkwrap, Debug, PartialEq, Clone, Copy, Properties)]
#[shrinkwrap(mutable)]
pub struct Rotation(pub Quat);
impl Rotation {
    #[inline(always)]
    pub fn identity() -> Self {
        Self(Quat::identity())
    }
}

impl Default for Rotation {
    fn default() -> Self {
        Self::identity()
    }
}

impl From<Quat> for Rotation {
    fn from(rotation: Quat) -> Self {
        Self(rotation)
    }
}
