use std::ops::{Deref, DerefMut};

use crate::{
    component::{Component, ComponentTicks},
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    world::{FromWorld, World},
};
use bevy_reflect::{impl_reflect_value, FromType, Reflect, ReflectDeserialize};

#[derive(Clone)]
pub struct ReflectComponent {
    add_component: fn(&mut World, Entity, &dyn Reflect),
    apply_component: fn(&mut World, Entity, &dyn Reflect),
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
                        component_ticks: c.component_ticks,
                        last_change_tick: c.last_change_tick,
                        change_tick: c.change_tick,
                    })
            },
        }
    }
}

/// Unique borrow of a Reflected component
pub struct ReflectMut<'a> {
    pub(crate) value: &'a mut dyn Reflect,
    pub(crate) component_ticks: &'a mut ComponentTicks,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

impl<'a> Deref for ReflectMut<'a> {
    type Target = dyn Reflect;

    #[inline]
    fn deref(&self) -> &dyn Reflect {
        self.value
    }
}

impl<'a> DerefMut for ReflectMut<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut dyn Reflect {
        self.component_ticks.set_changed(self.change_tick);
        self.value
    }
}

impl<'a> ReflectMut<'a> {
    /// Returns true if (and only if) this component been added since the last execution of this
    /// system.
    pub fn is_added(&self) -> bool {
        self.component_ticks
            .is_added(self.last_change_tick, self.change_tick)
    }

    /// Returns true if (and only if) this component been changed since the last execution of this
    /// system.
    pub fn is_changed(&self) -> bool {
        self.component_ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }
}

impl_reflect_value!(Entity(Hash, PartialEq, Serialize, Deserialize));

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
