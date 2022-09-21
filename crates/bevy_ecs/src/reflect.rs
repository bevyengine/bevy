//! Types that enable reflection support.

use crate::{
    change_detection::Mut,
    component::Component,
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    system::Resource,
    world::{FromWorld, World},
};
use bevy_reflect::{
    impl_from_reflect_value, impl_reflect_value, FromType, Reflect, ReflectDeserialize,
    ReflectSerialize,
};

/// A struct used to operate on reflected [`Component`] of a type.
///
/// A [`ReflectComponent`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectComponent {
    insert: fn(&mut World, Entity, &dyn Reflect),
    apply: fn(&mut World, Entity, &dyn Reflect),
    apply_or_insert: fn(&mut World, Entity, &dyn Reflect),
    remove: fn(&mut World, Entity),
    reflect: fn(&World, Entity) -> Option<&dyn Reflect>,
    reflect_mut: unsafe fn(&World, Entity) -> Option<Mut<dyn Reflect>>,
    copy: fn(&World, &mut World, Entity, Entity),
}

impl ReflectComponent {
    /// Insert a reflected [`Component`] into the entity like [`insert()`](crate::world::EntityMut::insert).
    ///
    /// # Panics
    ///
    /// Panics if there is no such entity.
    pub fn insert(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.insert)(world, entity, component);
    }

    /// Uses reflection to set the value of this [`Component`] type in the entity to the given value.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Component`] of the given type or the `entity` does not exist.
    pub fn apply(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.apply)(world, entity, component);
    }

    /// Uses reflection to set the value of this [`Component`] type in the entity to the given value or insert a new one if it does not exist.
    ///
    /// # Panics
    ///
    /// Panics if the `entity` does not exist.
    pub fn apply_or_insert(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.apply_or_insert)(world, entity, component);
    }

    /// Removes this [`Component`] type from the entity. Does nothing if it doesn't exist.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Component`] of the given type or the `entity` does not exist.
    pub fn remove(&self, world: &mut World, entity: Entity) {
        (self.remove)(world, entity);
    }

    /// Gets the value of this [`Component`] type from the entity as a reflected reference.
    pub fn reflect<'a>(&self, world: &'a World, entity: Entity) -> Option<&'a dyn Reflect> {
        (self.reflect)(world, entity)
    }

    /// Gets the value of this [`Component`] type from the entity as a mutable reflected reference.
    pub fn reflect_mut<'a>(
        &self,
        world: &'a mut World,
        entity: Entity,
    ) -> Option<Mut<'a, dyn Reflect>> {
        // SAFETY: unique world access
        unsafe { (self.reflect_mut)(world, entity) }
    }

    /// # Safety
    /// This method does not prevent you from having two mutable pointers to the same data,
    /// violating Rust's aliasing rules. To avoid this:
    /// * Only call this method in an exclusive system to avoid sharing across threads (or use a
    ///   scheduler that enforces safe memory access).
    /// * Don't call this method more than once in the same scope for a given [`Component`].
    pub unsafe fn reflect_unchecked_mut<'a>(
        &self,
        world: &'a World,
        entity: Entity,
    ) -> Option<Mut<'a, dyn Reflect>> {
        (self.reflect_mut)(world, entity)
    }

    /// Gets the value of this [`Component`] type from entity from `source_world` and [applies](Self::apply()) it to the value of this [`Component`] type in entity in `destination_world`.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Component`] of the given type or either entity does not exist.
    pub fn copy(
        &self,
        source_world: &World,
        destination_world: &mut World,
        source_entity: Entity,
        destination_entity: Entity,
    ) {
        (self.copy)(
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
            insert: |world, entity, reflected_component| {
                let mut component = C::from_world(world);
                component.apply(reflected_component);
                world.entity_mut(entity).insert(component);
            },
            apply: |world, entity, reflected_component| {
                let mut component = world.get_mut::<C>(entity).unwrap();
                component.apply(reflected_component);
            },
            apply_or_insert: |world, entity, reflected_component| {
                if let Some(mut component) = world.get_mut::<C>(entity) {
                    component.apply(reflected_component);
                } else {
                    let mut component = C::from_world(world);
                    component.apply(reflected_component);
                    world.entity_mut(entity).insert(component);
                }
            },
            remove: |world, entity| {
                world.entity_mut(entity).remove::<C>();
            },
            copy: |source_world, destination_world, source_entity, destination_entity| {
                let source_component = source_world.get::<C>(source_entity).unwrap();
                let mut destination_component = C::from_world(destination_world);
                destination_component.apply(source_component);
                destination_world
                    .entity_mut(destination_entity)
                    .insert(destination_component);
            },
            reflect: |world, entity| {
                world
                    .get_entity(entity)?
                    .get::<C>()
                    .map(|c| c as &dyn Reflect)
            },
            reflect_mut: |world, entity| {
                // SAFETY: reflect_mut is an unsafe function pointer used by `reflect_unchecked_mut` which promises to never
                // produce aliasing mutable references, and reflect_mut, which has mutable world access
                unsafe {
                    world
                        .get_entity(entity)?
                        .get_unchecked_mut::<C>(world.last_change_tick(), world.read_change_tick())
                        .map(|c| Mut {
                            value: c.value as &mut dyn Reflect,
                            ticks: c.ticks,
                        })
                }
            },
        }
    }
}

/// A struct used to operate on reflected [`Resource`] of a type.
///
/// A [`ReflectResource`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectResource {
    insert: fn(&mut World, &dyn Reflect),
    apply: fn(&mut World, &dyn Reflect),
    apply_or_insert: fn(&mut World, &dyn Reflect),
    remove: fn(&mut World),
    reflect: fn(&World) -> Option<&dyn Reflect>,
    reflect_unchecked_mut: unsafe fn(&World) -> Option<Mut<dyn Reflect>>,
    copy: fn(&World, &mut World),
}

impl ReflectResource {
    /// Insert a reflected [`Resource`] into the world like [`insert()`](World::insert_resource).
    pub fn insert(&self, world: &mut World, resource: &dyn Reflect) {
        (self.insert)(world, resource);
    }

    /// Uses reflection to set the value of this [`Resource`] type in the world to the given value.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Resource`] of the given type.
    pub fn apply(&self, world: &mut World, resource: &dyn Reflect) {
        (self.apply)(world, resource);
    }

    /// Uses reflection to set the value of this [`Resource`] type in the world to the given value or insert a new one if it does not exist.
    pub fn apply_or_insert(&self, world: &mut World, resource: &dyn Reflect) {
        (self.apply_or_insert)(world, resource);
    }

    /// Removes this [`Resource`] type from the world. Does nothing if it doesn't exist.
    pub fn remove(&self, world: &mut World) {
        (self.remove)(world);
    }

    /// Gets the value of this [`Resource`] type from the world as a reflected reference.
    pub fn reflect<'a>(&self, world: &'a World) -> Option<&'a dyn Reflect> {
        (self.reflect)(world)
    }

    /// Gets the value of this [`Resource`] type from the world as a mutable reflected reference.
    pub fn reflect_mut<'a>(&self, world: &'a mut World) -> Option<Mut<'a, dyn Reflect>> {
        // SAFETY: unique world access
        unsafe { (self.reflect_unchecked_mut)(world) }
    }

    /// # Safety
    /// This method does not prevent you from having two mutable pointers to the same data,
    /// violating Rust's aliasing rules. To avoid this:
    /// * Only call this method in an exclusive system to avoid sharing across threads (or use a
    ///   scheduler that enforces safe memory access).
    /// * Don't call this method more than once in the same scope for a given [`Resource`].
    pub unsafe fn reflect_unchecked_mut<'a>(
        &self,
        world: &'a World,
    ) -> Option<Mut<'a, dyn Reflect>> {
        // SAFETY: caller promises to uphold uniqueness guarantees
        (self.reflect_unchecked_mut)(world)
    }

    /// Gets the value of this [`Resource`] type from `source_world` and [applies](Self::apply()) it to the value of this [`Resource`] type in `destination_world`.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Resource`] of the given type.
    pub fn copy(&self, source_world: &World, destination_world: &mut World) {
        (self.copy)(source_world, destination_world);
    }
}

impl<C: Resource + Reflect + FromWorld> FromType<C> for ReflectResource {
    fn from_type() -> Self {
        ReflectResource {
            insert: |world, reflected_resource| {
                let mut resource = C::from_world(world);
                resource.apply(reflected_resource);
                world.insert_resource(resource);
            },
            apply: |world, reflected_resource| {
                let mut resource = world.resource_mut::<C>();
                resource.apply(reflected_resource);
            },
            apply_or_insert: |world, reflected_resource| {
                if let Some(mut resource) = world.get_resource_mut::<C>() {
                    resource.apply(reflected_resource);
                } else {
                    let mut resource = C::from_world(world);
                    resource.apply(reflected_resource);
                    world.insert_resource(resource);
                }
            },
            remove: |world| {
                world.remove_resource::<C>();
            },
            reflect: |world| world.get_resource::<C>().map(|res| res as &dyn Reflect),
            reflect_unchecked_mut: |world| {
                // SAFETY: all usages of `reflect_unchecked_mut` guarantee that there is either a single mutable
                // reference or multiple immutable ones alive at any given point
                unsafe {
                    world.get_resource_unchecked_mut::<C>().map(|res| Mut {
                        value: res.value as &mut dyn Reflect,
                        ticks: res.ticks,
                    })
                }
            },
            copy: |source_world, destination_world| {
                let source_resource = source_world.resource::<C>();
                let mut destination_resource = C::from_world(destination_world);
                destination_resource.apply(source_resource);
                destination_world.insert_resource(destination_resource);
            },
        }
    }
}

impl_reflect_value!(Entity(Hash, PartialEq, Serialize, Deserialize));
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
