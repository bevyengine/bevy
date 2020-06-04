use crate::{impl_property, property_serde::Serializable, Property, PropertyTypeRegistry};
use erased_serde::Deserializer;
use legion::prelude::Entity;
use serde::Deserialize;
use std::num::Wrapping;

impl_property!(Entity, serialize_entity, deserialize_entity);

mod private {
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize)]
    pub(super) struct Entity(pub(super) u32);
}

fn serialize_entity(entity: &Entity) -> Serializable {
    Serializable::Owned(Box::new(private::Entity(entity.index())))
}

fn deserialize_entity(
    deserializer: &mut dyn Deserializer,
    _registry: &PropertyTypeRegistry,
) -> Result<Box<dyn Property>, erased_serde::Error> {
    let entity = private::Entity::deserialize(deserializer)?;
    Ok(Box::new(Entity::new(entity.0, Wrapping(1))))
}
