use bevy_ecs::Entity;
use bevy_property::Properties;
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut};

#[derive(Default, Clone, Properties, Debug)]
pub struct Children(pub SmallVec<[Entity; 8]>);

impl Children {
    pub fn with(entity: &[Entity]) -> Self {
        Self(SmallVec::from_slice(entity))
    }
}

impl Deref for Children {
    type Target = SmallVec<[Entity; 8]>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Children {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
