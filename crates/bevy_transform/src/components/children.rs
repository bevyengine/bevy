use bevy_ecs::Entity;
use bevy_property::Properties;
use smallvec::SmallVec;

#[derive(Default, Clone, Properties, Debug)]
pub struct Children(pub SmallVec<[Entity; 8]>);

impl Children {
    pub fn with(entity: &[Entity]) -> Self {
        Self(SmallVec::from_slice(entity))
    }
}
