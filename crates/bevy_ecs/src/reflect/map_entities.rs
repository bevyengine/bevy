use crate::{
    component::Component,
    entity::{DynEntityMapper, Entity, EntityHashMap, MapEntities, SceneEntityMapper},
    world::World,
};
use bevy_reflect::{FromReflect, FromType, Reflect};

/// For a specific type of component, this maps any fields with values of type [`Entity`] to a new world.
/// Since a given `Entity` ID is only valid for the world it came from, when performing deserialization
/// any stored IDs need to be re-allocated in the destination world.
///
/// See [`SceneEntityMapper`] and [`MapEntities`] for more information.
#[derive(Clone)]
pub struct ReflectMapEntities {
    map_entities: fn(&mut dyn Reflect, &mut dyn DynEntityMapper),
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
    pub fn map_entities(&self, component: &mut dyn Reflect, mapper: &mut dyn DynEntityMapper) {
        (self.map_entities)(component, mapper);
    }
}

impl<C: Component + MapEntities + FromReflect> FromType<C> for ReflectMapEntities {
    fn from_type() -> Self {
        ReflectMapEntities {
            map_entities: |component, mut entity_mapper| {
                let mut concrete = C::from_reflect(&*component).unwrap();
                concrete.map_entities(&mut entity_mapper);
                component.apply(&concrete);
            },
        }
    }
}

/// For a specific type of resource, this maps any fields with values of type [`Entity`] to a new world.
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
