use bevy_ecs::{Entity, MapEntities};
use bevy_property::Properties;
use smallvec::SmallVec;
use std::ops::Deref;

#[derive(Default, Clone, Properties, Debug)]
pub struct Children(pub(crate) SmallVec<[Entity; 8]>);

impl MapEntities for Children {
    fn map_entities(
        &mut self,
        entity_map: &bevy_ecs::EntityMap,
    ) -> Result<(), bevy_ecs::MapEntitiesError> {
        for entity in self.0.iter_mut() {
            *entity = entity_map.get(*entity)?;
        }

        Ok(())
    }
}

impl Children {
    pub fn with(entity: &[Entity]) -> Self {
        Self(SmallVec::from_slice(entity))
    }
}

impl Deref for Children {
    type Target = [Entity];

    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}
