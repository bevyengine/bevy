use bevy_property::Properties;
use std::ops::{Deref, DerefMut};

// NOTE: This is used by the animation system to find the right entity to animate

/// Component containing the name used to uniquely identify a entity
#[derive(Default, Debug, Properties)]
pub struct Name(pub String);

impl Deref for Name {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Name {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
