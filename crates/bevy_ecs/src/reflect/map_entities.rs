use crate::{
    component::Component,
    entity::{Entity, EntityMapper, MapEntities},
    world::World,
};
use bevy_reflect::FromType;
use bevy_utils::HashMap;

/// For a specific type of component, this maps any fields with values of type [`Entity`] to a new world.
/// Since a given `Entity` ID is only valid for the world it came from, when performing deserialization
/// any stored IDs need to be re-allocated in the destination world.
///
/// See [`MapEntities`] for more information.
#[derive(Clone)]
pub struct ReflectMapEntities {
    map_all_entities: fn(&mut World, &mut EntityMapper),
    map_entities: fn(&mut World, &mut EntityMapper, &[Entity]),
}

impl ReflectMapEntities {
    /// A general method for applying [`MapEntities`] behavior to all elements in an [`HashMap<Entity, Entity>`].
    ///
    /// Be mindful in its usage: Works best in situations where the entities in the [`HashMap<Entity, Entity>`] are newly
    /// created, before systems have a chance to add new components. If some of the entities referred to
    /// by the [`HashMap<Entity, Entity>`] might already contain valid entity references, you should use [`map_entities`](Self::map_entities).
    ///
    /// An example of this: A scene can be loaded with `Parent` components, but then a `Parent` component can be added
    /// to these entities after they have been loaded. If you reload the scene using [`map_all_entities`](Self::map_all_entities), those `Parent`
    /// components with already valid entity references could be updated to point at something else entirely.
    pub fn map_all_entities(&self, world: &mut World, entity_map: &mut HashMap<Entity, Entity>) {
        EntityMapper::world_scope(entity_map, world, self.map_all_entities);
    }

    /// A general method for applying [`MapEntities`] behavior to elements in an [`HashMap<Entity, Entity>`]. Unlike
    /// [`map_all_entities`](Self::map_all_entities), this is applied to specific entities, not all values
    /// in the [`HashMap<Entity, Entity>`].
    ///
    /// This is useful mostly for when you need to be careful not to update components that already contain valid entity
    /// values. See [`map_all_entities`](Self::map_all_entities) for more details.
    pub fn map_entities(
        &self,
        world: &mut World,
        entity_map: &mut HashMap<Entity, Entity>,
        entities: &[Entity],
    ) {
        EntityMapper::world_scope(entity_map, world, |world, mapper| {
            (self.map_entities)(world, mapper, entities);
        });
    }
}

impl<C: Component + MapEntities> FromType<C> for ReflectMapEntities {
    fn from_type() -> Self {
        ReflectMapEntities {
            map_entities: |world, entity_mapper, entities| {
                for &entity in entities {
                    if let Some(mut component) = world.get_mut::<C>(entity) {
                        component.map_entities(entity_mapper);
                    }
                }
            },
            map_all_entities: |world, entity_mapper| {
                let entities = entity_mapper
                    .get_map()
                    .values()
                    .copied()
                    .collect::<Vec<Entity>>();
                for entity in &entities {
                    if let Some(mut component) = world.get_mut::<C>(*entity) {
                        component.map_entities(entity_mapper);
                    }
                }
            },
        }
    }
}
