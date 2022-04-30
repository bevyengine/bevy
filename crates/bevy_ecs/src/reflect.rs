//! Types that enable reflection support.

pub use crate::change_detection::ReflectMut;
use crate::{
    component::Component,
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    world::{FromWorld, World},
};
use bevy_reflect::{
    impl_from_reflect_value, impl_reflect_value, FromType, Reflect, ReflectDeserialize,
};

#[derive(Clone)]
pub struct ReflectComponent {
    add_component: fn(&mut World, Entity, &dyn Reflect),
    apply_component: fn(&mut World, Entity, &dyn Reflect),
    remove_component: fn(&mut World, Entity),
    reflect_component: fn(&World, Entity) -> Option<&dyn Reflect>,
    reflect_component_mut: unsafe fn(&World, Entity) -> Option<ReflectMut>,
    copy_component: fn(&World, &mut World, Entity, Entity),
}

impl ReflectComponent {
    pub fn add_component(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.add_component)(world, entity, component);
    }

    pub fn apply_component(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.apply_component)(world, entity, component);
    }

    pub fn remove_component(&self, world: &mut World, entity: Entity) {
        (self.remove_component)(world, entity);
    }

    pub fn reflect_component<'a>(
        &self,
        world: &'a World,
        entity: Entity,
    ) -> Option<&'a dyn Reflect> {
        (self.reflect_component)(world, entity)
    }

    pub fn reflect_component_mut<'a>(
        &self,
        world: &'a mut World,
        entity: Entity,
    ) -> Option<ReflectMut<'a>> {
        // SAFE: unique world access
        unsafe { (self.reflect_component_mut)(world, entity) }
    }

    /// # Safety
    /// This method does not prevent you from having two mutable pointers to the same data,
    /// violating Rust's aliasing rules. To avoid this:
    /// * Only call this method in an exclusive system to avoid sharing across threads (or use a
    ///   scheduler that enforces safe memory access).
    /// * Don't call this method more than once in the same scope for a given component.
    pub unsafe fn reflect_component_unchecked_mut<'a>(
        &self,
        world: &'a World,
        entity: Entity,
    ) -> Option<ReflectMut<'a>> {
        (self.reflect_component_mut)(world, entity)
    }

    pub fn copy_component(
        &self,
        source_world: &World,
        destination_world: &mut World,
        source_entity: Entity,
        destination_entity: Entity,
    ) {
        (self.copy_component)(
            source_world,
            destination_world,
            source_entity,
            destination_entity,
        );
    }
}

impl<C: Component + Reflect + FromWorld> FromType<C> for ReflectComponent {
    fn from_type() -> Self {
        ReflectComponent {
            add_component: |world, entity, reflected_component| {
                let mut component = C::from_world(world);
                component.apply(reflected_component);
                world.entity_mut(entity).insert(component);
            },
            apply_component: |world, entity, reflected_component| {
                let mut component = world.get_mut::<C>(entity).unwrap();
                component.apply(reflected_component);
            },
            remove_component: |world, entity| {
                world.entity_mut(entity).remove::<C>();
            },
            copy_component: |source_world, destination_world, source_entity, destination_entity| {
                let source_component = source_world.get::<C>(source_entity).unwrap();
                let mut destination_component = C::from_world(destination_world);
                destination_component.apply(source_component);
                destination_world
                    .entity_mut(destination_entity)
                    .insert(destination_component);
            },
            reflect_component: |world, entity| {
                world
                    .get_entity(entity)?
                    .get::<C>()
                    .map(|c| c as &dyn Reflect)
            },
            reflect_component_mut: |world, entity| unsafe {
                world
                    .get_entity(entity)?
                    .get_unchecked_mut::<C>(world.last_change_tick(), world.read_change_tick())
                    .map(|c| ReflectMut {
                        value: c.value as &mut dyn Reflect,
                        ticks: c.ticks,
                    })
            },
        }
    }
}

impl_reflect_value!(Entity(Hash, PartialEq, Deserialize));
impl_from_reflect_value!(Entity);

#[derive(Clone)]
pub struct ReflectMapEntities {
    map_entities: fn(&mut World, &EntityMap) -> Result<(), MapEntitiesError>,
}

impl ReflectMapEntities {
    pub fn map_entities(
        &self,
        world: &mut World,
        entity_map: &EntityMap,
    ) -> Result<(), MapEntitiesError> {
        (self.map_entities)(world, entity_map)
    }
}

impl<C: Component + MapEntities> FromType<C> for ReflectMapEntities {
    fn from_type() -> Self {
        ReflectMapEntities {
            map_entities: |world, entity_map| {
                for entity in entity_map.values() {
                    if let Some(mut component) = world.get_mut::<C>(entity) {
                        component.map_entities(entity_map)?;
                    }
                }
                Ok(())
            },
        }
    }
}
