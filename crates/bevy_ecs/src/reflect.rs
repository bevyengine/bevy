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
pub struct ReflectComponent(ReflectComponentFns);

/// The raw function pointers needed to make up a [`ReflectComponent`].
///
/// This is used when creating custom implementations of [`ReflectComponent`] with
/// [`ReflectComponent::new()`].
///
/// > **Note:**
/// > Creating custom implementations of [`ReflectComponent`] is an advanced feature that most users
/// > will not need.
/// > Usually a [`ReflectComponent`] is created for a type by deriving [`Reflect`]
/// > and adding the `#[reflect(Component)]` attribute.
/// > After adding the component to the [`TypeRegistry`][bevy_reflect::TypeRegistry],
/// > its [`ReflectComponent`] can then be retrieved when needed.
///
/// Creating a custom [`ReflectComponent`] may be useful if you need to create new component types
/// at runtime, for example, for scripting implementations.
///
/// By creating a custom [`ReflectComponent`] and inserting it into a type's
/// [`TypeRegistration`][bevy_reflect::TypeRegistration],
/// you can modify the way that reflected components of that type will be inserted into the Bevy
/// world.
#[derive(Clone)]
pub struct ReflectComponentFns {
    /// Function pointer implementing [`ReflectComponent::insert()`].
    pub insert: fn(&mut World, Entity, &dyn Reflect),
    /// Function pointer implementing [`ReflectComponent::apply()`].
    pub apply: fn(&mut World, Entity, &dyn Reflect),
    /// Function pointer implementing [`ReflectComponent::apply_or_insert()`].
    pub apply_or_insert: fn(&mut World, Entity, &dyn Reflect),
    /// Function pointer implementing [`ReflectComponent::remove()`].
    pub remove: fn(&mut World, Entity),
    /// Function pointer implementing [`ReflectComponent::reflect()`].
    pub reflect: fn(&World, Entity) -> Option<&dyn Reflect>,
    /// Function pointer implementing [`ReflectComponent::reflect_mut()`].
    pub reflect_mut: unsafe fn(&World, Entity) -> Option<Mut<dyn Reflect>>,
    /// Function pointer implementing [`ReflectComponent::copy()`].
    pub copy: fn(&World, &mut World, Entity, Entity),
}

impl ReflectComponentFns {
    /// Get the default set of [`ReflectComponentFns`] for a specific component type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Component + Reflect + FromWorld>() -> Self {
        <ReflectComponent as FromType<T>>::from_type().0
    }
}

impl ReflectComponent {
    /// Insert a reflected [`Component`] into the entity like [`insert()`](crate::world::EntityMut::insert).
    ///
    /// # Panics
    ///
    /// Panics if there is no such entity.
    pub fn insert(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.0.insert)(world, entity, component);
    }

    /// Uses reflection to set the value of this [`Component`] type in the entity to the given value.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Component`] of the given type or the `entity` does not exist.
    pub fn apply(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.0.apply)(world, entity, component);
    }

    /// Uses reflection to set the value of this [`Component`] type in the entity to the given value or insert a new one if it does not exist.
    ///
    /// # Panics
    ///
    /// Panics if the `entity` does not exist.
    pub fn apply_or_insert(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.0.apply_or_insert)(world, entity, component);
    }

    /// Removes this [`Component`] type from the entity. Does nothing if it doesn't exist.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Component`] of the given type or the `entity` does not exist.
    pub fn remove(&self, world: &mut World, entity: Entity) {
        (self.0.remove)(world, entity);
    }

    /// Gets the value of this [`Component`] type from the entity as a reflected reference.
    pub fn reflect<'a>(&self, world: &'a World, entity: Entity) -> Option<&'a dyn Reflect> {
        (self.0.reflect)(world, entity)
    }

    /// Gets the value of this [`Component`] type from the entity as a mutable reflected reference.
    pub fn reflect_mut<'a>(
        &self,
        world: &'a mut World,
        entity: Entity,
    ) -> Option<Mut<'a, dyn Reflect>> {
        // SAFETY: unique world access
        unsafe { (self.0.reflect_mut)(world, entity) }
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
        (self.0.reflect_mut)(world, entity)
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
        (self.0.copy)(
            source_world,
            destination_world,
            source_entity,
            destination_entity,
        );
    }

    /// Create a custom implementation of [`ReflectComponent`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(Component)]` component
    /// to generate a [`ReflectComponent`] implementation automatically.
    ///
    /// See [`ReflectComponentFns`] for more information.
    pub fn new(fns: ReflectComponentFns) -> Self {
        Self(fns)
    }
}

impl<C: Component + Reflect + FromWorld> FromType<C> for ReflectComponent {
    fn from_type() -> Self {
        ReflectComponent(ReflectComponentFns {
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
        })
    }
}

/// A struct used to operate on reflected [`Resource`] of a type.
///
/// A [`ReflectResource`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectResource(ReflectResourceFns);

/// The raw function pointers needed to make up a [`ReflectResource`].
///
/// This is used when creating custom implementations of [`ReflectResource`] with
/// [`ReflectResource::new()`].
///
/// > **Note:**
/// > Creating custom implementations of [`ReflectResource`] is an advanced feature that most users
/// > will not need.
/// > Usually a [`ReflectResource`] is created for a type by deriving [`Reflect`]
/// > and adding the `#[reflect(Resource)]` attribute.
/// > After adding the component to the [`TypeRegistry`][bevy_reflect::TypeRegistry],
/// > its [`ReflectResource`] can then be retrieved when needed.
///
/// Creating a custom [`ReflectResource`] may be useful if you need to create new resource types at
/// runtime, for example, for scripting implementations.
///
/// By creating a custom [`ReflectResource`] and inserting it into a type's
/// [`TypeRegistration`][bevy_reflect::TypeRegistration],
/// you can modify the way that reflected resources of that type will be inserted into the bevy
/// world.
#[derive(Clone)]
pub struct ReflectResourceFns {
    /// Function pointer implementing [`ReflectResource::insert()`].
    pub insert: fn(&mut World, &dyn Reflect),
    /// Function pointer implementing [`ReflectResource::apply()`].
    pub apply: fn(&mut World, &dyn Reflect),
    /// Function pointer implementing [`ReflectResource::apply_or_insert()`].
    pub apply_or_insert: fn(&mut World, &dyn Reflect),
    /// Function pointer implementing [`ReflectResource::remove()`].
    pub remove: fn(&mut World),
    /// Function pointer implementing [`ReflectResource::reflect()`].
    pub reflect: fn(&World) -> Option<&dyn Reflect>,
    /// Function pointer implementing [`ReflectResource::reflect_unchecked_mut()`].
    pub reflect_unchecked_mut: unsafe fn(&World) -> Option<Mut<dyn Reflect>>,
    /// Function pointer implementing [`ReflectResource::copy()`].
    pub copy: fn(&World, &mut World),
}

impl ReflectResourceFns {
    /// Get the default set of [`ReflectResourceFns`] for a specific resource type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Resource + Reflect + FromWorld>() -> Self {
        <ReflectResource as FromType<T>>::from_type().0
    }
}

impl ReflectResource {
    /// Insert a reflected [`Resource`] into the world like [`insert()`](World::insert_resource).
    pub fn insert(&self, world: &mut World, resource: &dyn Reflect) {
        (self.0.insert)(world, resource);
    }

    /// Uses reflection to set the value of this [`Resource`] type in the world to the given value.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Resource`] of the given type.
    pub fn apply(&self, world: &mut World, resource: &dyn Reflect) {
        (self.0.apply)(world, resource);
    }

    /// Uses reflection to set the value of this [`Resource`] type in the world to the given value or insert a new one if it does not exist.
    pub fn apply_or_insert(&self, world: &mut World, resource: &dyn Reflect) {
        (self.0.apply_or_insert)(world, resource);
    }

    /// Removes this [`Resource`] type from the world. Does nothing if it doesn't exist.
    pub fn remove(&self, world: &mut World) {
        (self.0.remove)(world);
    }

    /// Gets the value of this [`Resource`] type from the world as a reflected reference.
    pub fn reflect<'a>(&self, world: &'a World) -> Option<&'a dyn Reflect> {
        (self.0.reflect)(world)
    }

    /// Gets the value of this [`Resource`] type from the world as a mutable reflected reference.
    pub fn reflect_mut<'a>(&self, world: &'a mut World) -> Option<Mut<'a, dyn Reflect>> {
        // SAFETY: unique world access
        unsafe { (self.0.reflect_unchecked_mut)(world) }
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
        (self.0.reflect_unchecked_mut)(world)
    }

    /// Gets the value of this [`Resource`] type from `source_world` and [applies](Self::apply()) it to the value of this [`Resource`] type in `destination_world`.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Resource`] of the given type.
    pub fn copy(&self, source_world: &World, destination_world: &mut World) {
        (self.0.copy)(source_world, destination_world);
    }

    /// Create a custom implementation of [`ReflectResource`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(Resource)]` component
    /// to generate a [`ReflectResource`] implementation automatically.
    ///
    /// See [`ReflectResourceFns`] for more information.
    pub fn new(&self, fns: ReflectResourceFns) -> Self {
        Self(fns)
    }
}

impl<C: Resource + Reflect + FromWorld> FromType<C> for ReflectResource {
    fn from_type() -> Self {
        ReflectResource(ReflectResourceFns {
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
        })
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
