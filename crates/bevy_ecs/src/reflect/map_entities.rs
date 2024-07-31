use crate::{
    component::Component,
    entity::{DynEntityMapper, Entity, EntityHashMap, MapEntities, SceneEntityMapper},
    world::World,
};
use bevy_reflect::{FromReflect, FromType, PartialReflect};

/// For a specific type of component, this maps any fields with values of type [`Entity`] to a new world.
///
/// Since a given `Entity` ID is only valid for the world it came from, when performing deserialization
/// any stored IDs need to be re-allocated in the destination world.
///
/// See [`SceneEntityMapper`] and [`MapEntities`] for more information.
#[derive(Clone)]
pub struct ReflectMapEntities {
    map_entities: fn(&mut dyn PartialReflect, &mut dyn DynEntityMapper),
    map_all_world_entities: fn(&mut World, &mut SceneEntityMapper),
    map_world_entities: fn(&mut World, &mut SceneEntityMapper, &[Entity]),
}

impl ReflectMapEntities {
    /// A general method for applying [`MapEntities`] behavior to a reflected component.
    ///
    /// Be mindful in its usage: Works best in situations where the source entities
    /// in the [`EntityHashMap<Entity>`] have already been populated by spawning empty
    /// entities in the destination world if needed. For example, when spawning
    /// entities in a scene, if this is used on a component before ensuring that
    /// all entities in the scene have been allocated, a new mapping will be created
    /// with a "dead" entity.
    pub fn map_entities(
        &self,
        component: &mut dyn PartialReflect,
        mapper: &mut dyn DynEntityMapper,
    ) {
        (self.map_entities)(component, mapper);
    }

    /// A general method for applying [`MapEntities`] behavior to all elements in an [`EntityHashMap<Entity>`].
    ///
    /// Be mindful in its usage: Works best in situations where the entities in the [`EntityHashMap<Entity>`] are newly
    /// created, before systems have a chance to add new components. If some of the entities referred to
    /// by the [`EntityHashMap<Entity>`] might already contain valid entity references, you should use [`map_world_entities`](Self::map_world_entities).
    ///
    /// An example of this: A scene can be loaded with `Parent` components, but then a `Parent` component can be added
    /// to these entities after they have been loaded. If you reload the scene using [`map_all_world_entities`](Self::map_all_world_entities), those `Parent`
    /// components with already valid entity references could be updated to point at something else entirely.
    #[deprecated = "map_all_world_entities doesn't play well with Observers. Use map_entities instead."]
    pub fn map_all_world_entities(
        &self,
        world: &mut World,
        entity_map: &mut EntityHashMap<Entity>,
    ) {
        SceneEntityMapper::world_scope(entity_map, world, self.map_all_world_entities);
    }

    /// A general method for applying [`MapEntities`] behavior to elements in an [`EntityHashMap<Entity>`]. Unlike
    /// [`map_all_world_entities`](Self::map_all_world_entities), this is applied to specific entities, not all values
    /// in the [`EntityHashMap<Entity>`].
    ///
    /// This is useful mostly for when you need to be careful not to update components that already contain valid entity
    /// values. See [`map_all_world_entities`](Self::map_all_world_entities) for more details.
    #[deprecated = "map_world_entities doesn't play well with Observers. Use map_entities instead."]
    pub fn map_world_entities(
        &self,
        world: &mut World,
        entity_map: &mut EntityHashMap<Entity>,
        entities: &[Entity],
    ) {
        SceneEntityMapper::world_scope(entity_map, world, |world, mapper| {
            (self.map_world_entities)(world, mapper, entities);
        });
    }
}

impl<C: Component + MapEntities + FromReflect> FromType<C> for ReflectMapEntities {
    fn from_type() -> Self {
        ReflectMapEntities {
            map_entities: |component, mut entity_mapper| {
                let mut concrete = C::from_reflect(component.as_partial_reflect()).unwrap();
                concrete.map_entities(&mut entity_mapper);
                component.apply(&concrete);
            },
            map_world_entities: |world, entity_mapper, entities| {
                for &entity in entities {
                    if let Some(mut component) = world.get_mut::<C>(entity) {
                        component.map_entities(entity_mapper);
                    }
                }
            },
            map_all_world_entities: |world, entity_mapper| {
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

/// For a specific type of resource, this maps any fields with values of type [`Entity`] to a new world.
///
/// Since a given `Entity` ID is only valid for the world it came from, when performing deserialization
/// any stored IDs need to be re-allocated in the destination world.
///
/// See [`SceneEntityMapper`] and [`MapEntities`] for more information.
#[derive(Clone)]
pub struct ReflectMapEntitiesResource {
    map_entities: fn(&mut World, &mut SceneEntityMapper),
}

impl ReflectMapEntitiesResource {
    /// A method for applying [`MapEntities`] behavior to elements in an [`EntityHashMap<Entity>`].
    pub fn map_entities(&self, world: &mut World, entity_map: &mut EntityHashMap<Entity>) {
        SceneEntityMapper::world_scope(entity_map, world, |world, mapper| {
            (self.map_entities)(world, mapper);
        });
    }
}

impl<R: crate::system::Resource + MapEntities> FromType<R> for ReflectMapEntitiesResource {
    fn from_type() -> Self {
        ReflectMapEntitiesResource {
            map_entities: |world, entity_mapper| {
                if let Some(mut resource) = world.get_resource_mut::<R>() {
                    resource.map_entities(entity_mapper);
                }
            },
        }
    }
}
