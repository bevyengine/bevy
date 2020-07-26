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
