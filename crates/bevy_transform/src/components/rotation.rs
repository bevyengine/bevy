use bevy_math::Quat;
use bevy_property::Properties;
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq, Clone, Copy, Properties)]
pub struct Rotation(pub Quat);
impl Rotation {
    #[inline(always)]
    pub fn identity() -> Self {
        Self(Quat::identity())
    }

    #[inline(always)]
    pub fn from_rotation_yxz(yaw: f32, pitch: f32, roll: f32) -> Self {
        Self(Quat::from_rotation_ypr(yaw, pitch, roll))
    }

    #[inline(always)]
    pub fn from_rotation_xyz(x: f32, y: f32, z: f32) -> Self {
        Self(Quat::from_rotation_ypr(y, x, z))
    }

    #[inline(always)]
    pub fn from_rotation_x(x: f32) -> Self {
        Self(Quat::from_rotation_x(x))
    }

    #[inline(always)]
    pub fn from_rotation_y(y: f32) -> Self {
        Self(Quat::from_rotation_y(y))
    }

    #[inline(always)]
    pub fn from_rotation_z(z: f32) -> Self {
        Self(Quat::from_rotation_z(z))
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

impl Deref for Rotation {
    type Target = Quat;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Rotation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
