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

    #[inline(always)]
    pub fn from_euler_angles(yaw: f32, pitch: f32, roll: f32) -> Self {
        Self(Quat::from_rotation_ypr(yaw, pitch, roll))
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
