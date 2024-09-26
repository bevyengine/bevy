use crate::{
    component::Component,
    entity::{Entity, EntityHashMap, EntityMapper, MapEntities, SceneEntityMapper},
    world::World,
};
use bevy_reflect::{FromReflect, FromType, PartialReflect};

/// For a specific type of component, this maps any fields with values of type [`Entity`] to a new world.
///
/// Since a given `Entity` ID is only valid for the world it came from, when performing deserialization
/// any stored IDs need to be re-allocated in the destination world.
///
/// See [`SceneEntityMapper`] and [`MapEntities`] for more information.
///
/// [`MapEntities`]: crate::entity::MapEntities
#[derive(Clone)]
pub struct ReflectMapEntities {
    map_all_world_entities: fn(&mut World, &mut SceneEntityMapper),
    map_world_entities: fn(&mut World, &mut SceneEntityMapper, &[Entity]),
    map_entities_mut: fn(&mut dyn PartialReflect, &mut dyn FnMut(&mut Entity)),
    map_entities: fn(&dyn PartialReflect, &mut dyn FnMut(Entity)),
}

impl ReflectMapEntities {
    /// A general method for applying [`MapEntities`] behavior to all elements in an [`EntityHashMap<Entity>`].
    ///
    /// Be mindful in its usage: Works best in situations where the entities in the [`EntityHashMap<Entity>`] are newly
    /// created, before systems have a chance to add new components. If some of the entities referred to
    /// by the [`EntityHashMap<Entity>`] might already contain valid entity references, you should use
    /// [`map_world_entities`](Self::map_entities).
    ///
    /// An example of this: A scene can be loaded with `Parent` components, but then a `Parent` component can be added
    /// to these entities after they have been loaded. If you reload the scene using [`map_all_world_entities`](Self::map_all_world_entities), those `Parent`
    /// components with already valid entity references could be updated to point at something else entirely.
    ///
    /// [`MapEntities`]: crate::entity::MapEntities
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
    ///
    /// [`MapEntities`]: crate::entity::MapEntities
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

    /// A general method for applying an operation to all entities in a
    /// reflected component.
    pub fn map_entities(&self, component: &dyn PartialReflect, f: &mut dyn FnMut(Entity)) {
        (self.map_entities)(component, f);
    }

    /// A general method for applying an operation that may modify entities in a
    /// reflected component.
    pub fn map_entities_mut(
        &self,
        component: &mut dyn PartialReflect,
        f: &mut dyn FnMut(&mut Entity),
    ) {
        (self.map_entities_mut)(component, f);
    }
}

impl<C: Component + FromReflect + MapEntities> FromType<C> for ReflectMapEntities {
    fn from_type() -> Self {
        ReflectMapEntities {
            map_world_entities: |world, entity_mapper, entities| {
                for &entity in entities {
                    if let Some(mut component) = world.get_mut::<C>(entity) {
                        component.map_entities_mut(|entity| {
                            *entity = entity_mapper.map_entity(*entity);
                        });
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
                        component.map_entities_mut(|entity| {
                            *entity = entity_mapper.map_entity(*entity);
                        });
                    }
                }
            },
            map_entities: |component, f| {
                let mut concrete = C::from_reflect(component).unwrap();
                concrete.map_entities_mut(|entity| f(*entity));
            },
            map_entities_mut: |component, f| {
                let mut concrete = C::from_reflect(component).unwrap();
                concrete.map_entities_mut(f);
                component.apply(&concrete);
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
///
/// [`MapEntities`]: crate::entity::MapEntities
#[derive(Clone)]
pub struct ReflectMapEntitiesResource {
    map_world_entities: fn(&mut World, &mut SceneEntityMapper),
    map_entities_mut: fn(&mut dyn PartialReflect, &mut dyn FnMut(&mut Entity)),
    map_entities: fn(&dyn PartialReflect, &mut dyn FnMut(Entity)),
}

impl ReflectMapEntitiesResource {
    /// A method for applying [`MapEntities`] behavior to elements in a [`EntityHashMap<Entity>`].
    ///
    /// [`MapEntities`]: crate::entity::MapEntities
    pub fn map_world_entities(&self, world: &mut World, entity_map: &mut EntityHashMap<Entity>) {
        SceneEntityMapper::world_scope(entity_map, world, |world, mapper| {
            (self.map_world_entities)(world, mapper);
        });
    }

    /// A general method for applying an operation to all entities in a
    /// reflected component.
    pub fn map_entities(&self, component: &dyn PartialReflect, f: &mut dyn FnMut(Entity)) {
        (self.map_entities)(component, f);
    }

    /// A general method for applying an operation that may modify entities in a
    /// reflected component.
    pub fn map_entities_mut(
        &self,
        component: &mut dyn PartialReflect,
        f: &mut dyn FnMut(&mut Entity),
    ) {
        (self.map_entities_mut)(component, f);
    }
}

impl<R: crate::system::Resource + FromReflect + MapEntities> FromType<R>
    for ReflectMapEntitiesResource
{
    fn from_type() -> Self {
        ReflectMapEntitiesResource {
            map_world_entities: |world, entity_mapper| {
                if let Some(mut resource) = world.get_resource_mut::<R>() {
                    resource.map_entities_mut(|entity| {
                        *entity = entity_mapper.map_entity(*entity);
                    });
                }
            },
            map_entities: |component, f| {
                let mut concrete = R::from_reflect(component).unwrap();
                concrete.map_entities_mut(|entity| f(*entity));
            },
            map_entities_mut: |component, f| {
                let mut concrete = R::from_reflect(component).unwrap();
                concrete.map_entities_mut(f);
                component.apply(&concrete);
            },
        }
    }
}
