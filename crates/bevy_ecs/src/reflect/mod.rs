//! Types that enable reflection support.

mod component;
mod queryable;
mod reflect_query_structs;
mod resource;

use crate::{
    component::Component,
    entity::{Entity, EntityMap, EntityMapper, MapEntities},
    world::World,
};
use bevy_reflect::{
    impl_from_reflect_value, impl_reflect_value, FromType, ReflectDeserialize, ReflectSerialize,
};

pub use component::{ReflectComponent, ReflectComponentFns};
pub use reflect_query_structs::{
    EntityQuerydyn, EntityQuerydynIter, MutQuerydyn, MutQuerydynIter, Querydyn, QuerydynIter,
    RefQuerydyn, RefQuerydynIter,
};
pub use resource::{ReflectResource, ReflectResourceFns};

impl_reflect_value!((in bevy_ecs) Entity(Hash, PartialEq, Serialize, Deserialize));
impl_from_reflect_value!(Entity);

/// For a specific type of component, this maps any fields with values of type [`Entity`] to a new world.
/// Since a given `Entity` ID is only valid for the world it came frome, when performing deserialization
/// any stored IDs need to be re-allocated in the destination world.
///
/// See [`MapEntities`] for more information.
#[derive(Clone)]
pub struct ReflectMapEntities {
    map_all_entities: fn(&mut World, &mut EntityMapper),
    map_entities: fn(&mut World, &mut EntityMapper, &[Entity]),
}

impl ReflectMapEntities {
    /// A general method for applying [`MapEntities`] behavior to all elements in an [`EntityMap`].
    ///
    /// Be mindful in its usage: Works best in situations where the entities in the [`EntityMap`] are newly
    /// created, before systems have a chance to add new components. If some of the entities referred to
    /// by the [`EntityMap`] might already contain valid entity references, you should use [`map_entities`](Self::map_entities).
    ///
    /// An example of this: A scene can be loaded with `Parent` components, but then a `Parent` component can be added
    /// to these entities after they have been loaded. If you reload the scene using [`map_all_entities`](Self::map_all_entities), those `Parent`
    /// components with already valid entity references could be updated to point at something else entirely.
    pub fn map_all_entities(&self, world: &mut World, entity_map: &mut EntityMap) {
        entity_map.world_scope(world, self.map_all_entities);
    }

    /// A general method for applying [`MapEntities`] behavior to elements in an [`EntityMap`]. Unlike
    /// [`map_all_entities`](Self::map_all_entities), this is applied to specific entities, not all values
    /// in the [`EntityMap`].
    ///
    /// This is useful mostly for when you need to be careful not to update components that already contain valid entity
    /// values. See [`map_all_entities`](Self::map_all_entities) for more details.
    pub fn map_entities(&self, world: &mut World, entity_map: &mut EntityMap, entities: &[Entity]) {
        entity_map.world_scope(world, |world, mapper| {
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
                let entities = entity_mapper.get_map().values().collect::<Vec<Entity>>();
                for entity in &entities {
                    if let Some(mut component) = world.get_mut::<C>(*entity) {
                        component.map_entities(entity_mapper);
                    }
                }
            },
        }
    }
}
