use super::queryable;
use super::reflect_query_structs::{EntityQuerydyn, MutQuerydyn, Querydyn, RefQuerydyn};
use crate::{
    change_detection::{Mut, Ref},
    component::Component,
    entity::Entity,
    query::QuerySingleError,
    world::{unsafe_world_cell::UnsafeEntityCell, EntityMut, EntityRef, FromWorld, World},
};
use bevy_reflect::{FromType, Reflect};

#[rustfmt::skip] // skip: this makes reading those generated doc string a bit easier.
macro_rules! docs {
    (fn $queryable_method:literal) => {
        concat!("Function pointer implementing [`ReflectComponent::", $queryable_method, "`].")
    };
    (single $query_equivalent:literal, $output:literal, $output_link:literal) => {
        concat!(
"Get a single [`", $output, "`](", $output_link, r#") of the underyling
[`Component`] from `World`, failing if there isn't exactly one `Entity`
matching this description.

Consider using [`ReflectComponent::"#, $query_equivalent, r#"`] followed
by `.next()` if you want to get a value even if there is more than one
`Entity` with the underlying `Component`.

# Errors

This will return an `Err` if:

 - There is no `Entity` with the underyling `Component` in `world`.
 - There is more than one `Entity` with the underyling `Component` in `world`."#
        )
    };

    (query
        $querydyn:literal, $single_equivalent:literal, $method_name:literal,
        $item:literal, $item_link:literal $(,)?
    ) => {
        concat!(
"Get a [`", $querydyn, "`] to iterate over all\n\
[`", $item, "`](", $item_link, r#") with the underlying
[`Component`] from `world`.

Use [`ReflectComponent::"#, $single_equivalent, r#"`] for a version that returns
a single element directly."#
        )
    };
}

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
    /// Function pointer implementing [`ReflectComponent::from_world()`].
    pub from_world: fn(&mut World) -> Box<dyn Reflect>,
    /// Function pointer implementing [`ReflectComponent::insert()`].
    pub insert: fn(&mut EntityMut, &dyn Reflect),
    /// Function pointer implementing [`ReflectComponent::apply()`].
    pub apply: fn(&mut EntityMut, &dyn Reflect),
    /// Function pointer implementing [`ReflectComponent::apply_or_insert()`].
    pub apply_or_insert: fn(&mut EntityMut, &dyn Reflect),
    /// Function pointer implementing [`ReflectComponent::remove()`].
    pub remove: fn(&mut EntityMut),
    /// Function pointer implementing [`ReflectComponent::contains()`].
    pub contains: fn(EntityRef) -> bool,
    /// Function pointer implementing [`ReflectComponent::reflect()`].
    pub reflect: fn(EntityRef) -> Option<&dyn Reflect>,
    /// Function pointer implementing [`ReflectComponent::reflect_mut()`].
    pub reflect_mut: for<'a> fn(&'a mut EntityMut<'_>) -> Option<Mut<'a, dyn Reflect>>,
    /// Function pointer implementing [`ReflectComponent::reflect_unchecked_mut()`].
    ///
    /// # Safety
    /// The function may only be called with an [`UnsafeEntityCell`] that can be used to mutably access the relevant component on the given entity.
    pub reflect_unchecked_mut: unsafe fn(UnsafeEntityCell<'_>) -> Option<Mut<'_, dyn Reflect>>,
    /// Function pointer implementing [`ReflectComponent::copy()`].
    pub copy: fn(&World, &mut World, Entity, Entity),

    /// Function pointer implementing [`ReflectComponent::reflect_ref()`].
    pub reflect_ref: fn(EntityRef) -> Option<Ref<dyn Reflect>>,

    /// Function pointer implementing [`ReflectComponent::get_single()`].
    pub get_single: fn(&mut World) -> Result<&dyn Reflect, QuerySingleError>,
    /// Function pointer implementing [`ReflectComponent::get_single_entity()`].
    pub get_single_entity: fn(&mut World) -> Result<Entity, QuerySingleError>,
    /// Function pointer implementing [`ReflectComponent::get_single_ref()`].
    pub get_single_ref: fn(&mut World) -> Result<Ref<dyn Reflect>, QuerySingleError>,
    /// Function pointer implementing [`ReflectComponent::get_single_mut()`].
    pub get_single_mut: fn(&mut World) -> Result<Mut<dyn Reflect>, QuerySingleError>,

    /// Function pointer implementing [`ReflectComponent::query()`].
    pub query: fn(&mut World) -> Querydyn,
    /// Function pointer implementing [`ReflectComponent::query_entities()`].
    pub query_entities: fn(&mut World) -> EntityQuerydyn,
    /// Function pointer implementing [`ReflectComponent::query_ref()`].
    pub query_ref: fn(&mut World) -> RefQuerydyn,
    /// Function pointer implementing [`ReflectComponent::query_mut()`].
    pub query_mut: fn(&mut World) -> MutQuerydyn,
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
    /// Constructs default reflected [`Component`] from world using [`from_world()`](FromWorld::from_world).
    pub fn from_world(&self, world: &mut World) -> Box<dyn Reflect> {
        (self.0.from_world)(world)
    }

    /// Insert a reflected [`Component`] into the entity like [`insert()`](crate::world::EntityMut::insert).
    pub fn insert(&self, entity: &mut EntityMut, component: &dyn Reflect) {
        (self.0.insert)(entity, component);
    }

    /// Uses reflection to set the value of this [`Component`] type in the entity to the given value.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Component`] of the given type.
    pub fn apply(&self, entity: &mut EntityMut, component: &dyn Reflect) {
        (self.0.apply)(entity, component);
    }

    /// Uses reflection to set the value of this [`Component`] type in the entity to the given value or insert a new one if it does not exist.
    pub fn apply_or_insert(&self, entity: &mut EntityMut, component: &dyn Reflect) {
        (self.0.apply_or_insert)(entity, component);
    }

    /// Removes this [`Component`] type from the entity. Does nothing if it doesn't exist.
    pub fn remove(&self, entity: &mut EntityMut) {
        (self.0.remove)(entity);
    }

    /// Returns whether entity contains this [`Component`]
    pub fn contains(&self, entity: EntityRef) -> bool {
        (self.0.contains)(entity)
    }

    /// Gets the value of this [`Component`] type from the entity as a reflected reference.
    pub fn reflect<'a>(&self, entity: EntityRef<'a>) -> Option<&'a dyn Reflect> {
        (self.0.reflect)(entity)
    }

    /// Gets the value of this [`Component`] type from the entity as a mutable reflected reference.
    pub fn reflect_mut<'a>(&self, entity: &'a mut EntityMut<'_>) -> Option<Mut<'a, dyn Reflect>> {
        (self.0.reflect_mut)(entity)
    }

    /// # Safety
    /// This method does not prevent you from having two mutable pointers to the same data,
    /// violating Rust's aliasing rules. To avoid this:
    /// * Only call this method with a [`UnsafeEntityCell`] that may be used to mutably access the component on the entity `entity`
    /// * Don't call this method more than once in the same scope for a given [`Component`].
    pub unsafe fn reflect_unchecked_mut<'a>(
        &self,
        entity: UnsafeEntityCell<'a>,
    ) -> Option<Mut<'a, dyn Reflect>> {
        // SAFETY: safety requirements deferred to caller
        (self.0.reflect_unchecked_mut)(entity)
    }

    #[doc = docs!(single "query", "&dyn Reflect", "Reflect")]
    pub fn get_single<'a>(
        &self,
        world: &'a mut World,
    ) -> Result<&'a dyn Reflect, QuerySingleError> {
        (self.0.get_single)(world)
    }
    #[doc = docs!(single "query_ref", "Ref<dyn Reflect>", "Ref")]
    pub fn get_single_ref<'a>(
        &self,
        world: &'a mut World,
    ) -> Result<Ref<'a, dyn Reflect>, QuerySingleError> {
        (self.0.get_single_ref)(world)
    }
    #[doc = docs!(single "query_mut", "Mut<dyn Reflect>", "Mut")]
    pub fn get_single_mut<'a>(
        &self,
        world: &'a mut World,
    ) -> Result<Mut<'a, dyn Reflect>, QuerySingleError> {
        (self.0.get_single_mut)(world)
    }
    #[doc = docs!(single "query_entities", "Entity", "Entity")]
    pub fn get_single_entity(&self, world: &mut World) -> Result<Entity, QuerySingleError> {
        (self.0.get_single_entity)(world)
    }

    #[doc = docs!(query "Querydyn", "get_single", "query", "&dyn Reflect", "Reflect")]
    pub fn query(&self, world: &mut World) -> Querydyn {
        (self.0.query)(world)
    }
    #[doc = docs!(query "EntityQuerydyn", "get_single_entity", "query_entities", "Entity", "Entity")]
    pub fn query_entities(&self, world: &mut World) -> EntityQuerydyn {
        (self.0.query_entities)(world)
    }
    #[doc = docs!(query "RefQuerydyn", "get_single_ref", "query_ref", "Ref<dyn Reflect>", "Ref")]
    pub fn query_ref(&self, world: &mut World) -> RefQuerydyn {
        (self.0.query_ref)(world)
    }
    #[doc = docs!(query "MutQuerydyn", "get_single_mut", "query_mut", "Mut<dyn Reflect>", "Mut")]
    pub fn query_mut(&self, world: &mut World) -> MutQuerydyn {
        (self.0.query_mut)(world)
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
            from_world: |world| Box::new(C::from_world(world)),
            insert: |entity, reflected_component| {
                let mut component = entity.world_scope(|world| C::from_world(world));
                component.apply(reflected_component);
                entity.insert(component);
            },
            apply: |entity, reflected_component| {
                let mut component = entity.get_mut::<C>().unwrap();
                component.apply(reflected_component);
            },
            apply_or_insert: |entity, reflected_component| {
                if let Some(mut component) = entity.get_mut::<C>() {
                    component.apply(reflected_component);
                } else {
                    let mut component = entity.world_scope(|world| C::from_world(world));
                    component.apply(reflected_component);
                    entity.insert(component);
                }
            },
            remove: |entity| {
                entity.remove::<C>();
            },
            contains: |entity| entity.contains::<C>(),
            copy: |source_world, destination_world, source_entity, destination_entity| {
                let source_component = source_world.get::<C>(source_entity).unwrap();
                let mut destination_component = C::from_world(destination_world);
                destination_component.apply(source_component);
                destination_world
                    .entity_mut(destination_entity)
                    .insert(destination_component);
            },
            reflect: |entity| entity.get::<C>().map(|c| c as &dyn Reflect),
            reflect_mut: |entity| {
                entity.get_mut::<C>().map(|c| Mut {
                    value: c.value as &mut dyn Reflect,
                    ticks: c.ticks,
                })
            },
            reflect_unchecked_mut: |entity| {
                // SAFETY: reflect_unchecked_mut is an unsafe function pointer used by
                // `reflect_unchecked_mut` which must be called with an UnsafeEntityCell with access to the the component `C` on the `entity`
                unsafe {
                    entity.get_mut::<C>().map(|c| Mut {
                        value: c.value as &mut dyn Reflect,
                        ticks: c.ticks,
                    })
                }
            },
            reflect_ref: queryable::reflect_ref::<C>,
            get_single: queryable::get_single::<C>,
            get_single_entity: queryable::get_single_entity::<C>,
            get_single_ref: queryable::get_single_ref::<C>,
            get_single_mut: queryable::get_single_mut::<C>,
            query: queryable::query::<C>,
            query_entities: queryable::query_entities::<C>,
            query_ref: queryable::query_ref::<C>,
            query_mut: queryable::query_mut::<C>,
        })
    }
}
