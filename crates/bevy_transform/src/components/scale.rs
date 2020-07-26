use bevy_property::Properties;
use std::{
    fmt,
    ops::{Deref, DerefMut},
};

#[derive(Debug, PartialEq, Clone, Copy, Properties)]
pub struct Scale(pub f32);

impl From<f32> for Scale {
    #[inline(always)]
    fn from(scale: f32) -> Self {
        Self(scale)
    }
}

impl Scale {
    #[inline(always)]
    pub fn identity() -> Self {
        Scale(1.0)
    }
}

impl Default for Scale {
    fn default() -> Self {
        Self::identity()
    }
}

impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Scale({})", self.0)
    }
}

impl Deref for Scale {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Scale {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
