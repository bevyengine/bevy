//! Types that enable run-time type reflection of ECS data.

pub use crate::change_detection::ReflectMut;
use crate::{
    component::Component,
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    world::{FromWorld, World},
};
use bevy_reflect::{impl_reflect_value, FromType, Reflect, ReflectDeserialize};

/// A runtime type-reflectable component
///
/// Intended for use with [`bevy_reflect`],
/// a [`ReflectComponent`] is a type-erased version of a component's data,
/// can be transformed into  ['dyn Reflect'](Reflect) trait objects,
/// which can be worked with generically
/// and loaded from disk in a type-safe fashion.
///
/// [`ReflectComponent`] objects are created for a particular [`Component`] type (`C`) using the [`from_type`](FromType::from_type) method.
/// That type `C` is implicitly stored in the function pointers held within the private fields of this type;
/// it cannot be changed after creation.
///
/// Once a [`ReflectComponent`] object has been created, you can use that concrete struct
/// to use the methods on this type, which always implicitly affect only the component type originally used to create this struct.
#[derive(Clone)]
pub struct ReflectComponent {
    insert_component: fn(&mut World, Entity, &dyn Reflect),
    apply_component: fn(&mut World, Entity, &dyn Reflect),
    remove_component: fn(&mut World, Entity),
    reflect_component: fn(&World, Entity) -> Option<&dyn Reflect>,
    reflect_component_mut: unsafe fn(&World, Entity) -> Option<ReflectMut>,
    copy_component: fn(&World, &mut World, Entity, Entity),
}

impl ReflectComponent {
    /// Inserts the non-erased value of `component` (with type `C`) into the `entity`
    ///
    /// # Panics
    /// `component` must have the same type `C` as the type used to create this struct
    pub fn insert_component(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.insert_component)(world, entity, component);
    }

    /// Sets the existing value of type `C` found on `entity` to the non-erased value of `component`
    ///
    /// # Panics
    /// `component` must have the same type `C` as the type used to create this struct
    /// Additionally, a component of type `C` must already exist on `entity.
    pub fn apply_component(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.apply_component)(world, entity, component);
    }

    /// Removes any component of type `C` from the `entity`
    pub fn remove_component(&self, world: &mut World, entity: Entity) {
        (self.remove_component)(world, entity);
    }

    /// Fetches an immutable reference to the component of type `C` on `entity`
    ///
    /// If the `Entity` does not have a component of the specified type, `None` is returned instead.
    pub fn reflect_component<'a>(
        &self,
        world: &'a World,
        entity: Entity,
    ) -> Option<&'a dyn Reflect> {
        (self.reflect_component)(world, entity)
    }

    /// Fetches a mutable reference to the component of type `C` on `entity`
    ///
    /// If the [`Entity`] does not have a component of the specified type, `None` is returned instead.
    pub fn reflect_component_mut<'a>(
        &self,
        world: &'a mut World,
        entity: Entity,
    ) -> Option<ReflectMut<'a>> {
        // SAFE: unique world access
        unsafe { (self.reflect_component_mut)(world, entity) }
    }

    /// Fetches a mutable reference to the component of type `C` on `entity` without guaranteeing unique mutable access to the `world`
    ///
    /// This method does not require exclusive [`World`] access, and so multiple mutable references can be alive at once.
    /// If possible, please prefer the safe version of this method, [`reflect_component_mut`](Self::reflect_component_mut).
    ///
    /// If the [`Entity`] does not have a component of the specified type, [`None`] is returned instead.
    ///
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

    /// Directly copies the value of the component of type `C` to a new entity
    ///
    /// This method creates a new component of type `C` using the [`FromWorld`] trait,
    /// sets its value to the value of the component of type `C` on the `source_entity`,
    /// and then inserts the new component into the `destination_entity`.
    ///
    /// **Note**: this method uses [`Reflect`] to create a shallow value-based copy of the component and will not respect `Clone` implementations.
    /// This can have unexpected negative consequences if you are relying on ref-counting or the like.
    ///
    /// # Panics
    /// The `source_entity` in the `source_world` must have a component of type `C`
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
            insert_component: |world, entity, reflected_component| {
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

impl_reflect_value!(Entity(Hash, PartialEq, Serialize, Deserialize));

/// A reflected [EntityMap]
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
