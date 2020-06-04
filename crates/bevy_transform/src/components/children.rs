use crate::ecs::prelude::*;
use bevy_property::Properties;
use shrinkwraprs::Shrinkwrap;
use smallvec::SmallVec;

#[derive(Shrinkwrap, Default, Clone, Properties)]
#[shrinkwrap(mutable)]
pub struct Children(pub SmallVec<[Entity; 8]>);

impl Children {
    pub fn with(entity: &[Entity]) -> Self {
        Self(SmallVec::from_slice(entity))
    }
}
