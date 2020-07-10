use bevy_property::Properties;
use shrinkwraprs::Shrinkwrap;
use smallvec::SmallVec;
use bevy_ecs::Entity;

#[derive(Shrinkwrap, Default, Clone, Properties, Debug)]
#[shrinkwrap(mutable)]
pub struct Children(pub SmallVec<[Entity; 8]>);

impl Children {
    pub fn with(entity: &[Entity]) -> Self {
        Self(SmallVec::from_slice(entity))
    }
}
