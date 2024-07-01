//! Definitions for [`Component`] reflection.
//! This allows inserting, updating, removing and generally interacting with components
//! whose types are only known at runtime.
//!
//! This module exports two types: [`ReflectComponentFns`] and [`ReflectComponent`].
//!
//! # Architecture
//!
//! [`ReflectComponent`] wraps a [`ReflectComponentFns`]. In fact, each method on
//! [`ReflectComponent`] wraps a call to a function pointer field in `ReflectComponentFns`.
//!
//! ## Who creates `ReflectComponent`s?
//!
//! When a user adds the `#[reflect(Component)]` attribute to their `#[derive(Reflect)]`
//! type, it tells the derive macro for `Reflect` to add the following single line to its
//! [`get_type_registration`] method (see the relevant code[^1]).
//!
//! ```
//! # use bevy_reflect::{FromType, Reflect};
//! # use bevy_ecs::prelude::{ReflectComponent, Component};
//! # #[derive(Default, Reflect, Component)]
//! # struct A;
//! # impl A {
//! #   fn foo() {
//! # let mut registration = bevy_reflect::TypeRegistration::of::<A>();
//! registration.insert::<ReflectComponent>(FromType::<Self>::from_type());
//! #   }
//! # }
//! ```
//!
//! This line adds a `ReflectComponent` to the registration data for the type in question.
//! The user can access the `ReflectComponent` for type `T` through the type registry,
//! as per the `trait_reflection.rs` example.
//!
//! The `FromType::<Self>::from_type()` in the previous line calls the `FromType<C>`
//! implementation of `ReflectComponent`.
//!
//! The `FromType<C>` impl creates a function per field of [`ReflectComponentFns`].
//! In those functions, we call generic methods on [`World`] and [`EntityWorldMut`].
//!
//! The result is a `ReflectComponent` completely independent of `C`, yet capable
//! of using generic ECS methods such as `entity.get::<C>()` to get `&dyn Reflect`
//! with underlying type `C`, without the `C` appearing in the type signature.
//!
//! ## A note on code generation
//!
//! A downside of this approach is that monomorphized code (ie: concrete code
//! for generics) is generated **unconditionally**, regardless of whether it ends
//! up used or not.
//!
//! Adding `N` fields on `ReflectComponentFns` will generate `N × M` additional
//! functions, where `M` is how many types derive `#[reflect(Component)]`.
//!
//! Those functions will increase the size of the final app binary.
//!
//! [^1]: `crates/bevy_reflect/bevy_reflect_derive/src/registration.rs`
//!
//! [`get_type_registration`]: bevy_reflect::GetTypeRegistration::get_type_registration

use super::from_reflect_with_fallback;
use crate::{
    change_detection::Mut,
    component::Component,
    entity::Entity,
    world::{
        unsafe_world_cell::UnsafeEntityCell, EntityMut, EntityWorldMut, FilteredEntityMut,
        FilteredEntityRef, World,
    },
};
use bevy_reflect::{FromReflect, FromType, Reflect, TypeRegistry};

/// A struct used to operate on reflected [`Component`] trait of a type.
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
/// > After adding the component to the [`TypeRegistry`],
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
    pub insert: fn(&mut EntityWorldMut, &dyn Reflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectComponent::apply()`].
    pub apply: fn(EntityMut, &dyn Reflect),
    /// Function pointer implementing [`ReflectComponent::apply_or_insert()`].
    pub apply_or_insert: fn(&mut EntityWorldMut, &dyn Reflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectComponent::remove()`].
    pub remove: fn(&mut EntityWorldMut),
    /// Function pointer implementing [`ReflectComponent::contains()`].
    pub contains: fn(FilteredEntityRef) -> bool,
    /// Function pointer implementing [`ReflectComponent::reflect()`].
    pub reflect: fn(FilteredEntityRef) -> Option<&dyn Reflect>,
    /// Function pointer implementing [`ReflectComponent::reflect_mut()`].
    pub reflect_mut: fn(FilteredEntityMut) -> Option<Mut<dyn Reflect>>,
    /// Function pointer implementing [`ReflectComponent::reflect_unchecked_mut()`].
    ///
    /// # Safety
    /// The function may only be called with an [`UnsafeEntityCell`] that can be used to mutably access the relevant component on the given entity.
    pub reflect_unchecked_mut: unsafe fn(UnsafeEntityCell<'_>) -> Option<Mut<'_, dyn Reflect>>,
    /// Function pointer implementing [`ReflectComponent::copy()`].
    pub copy: fn(&World, &mut World, Entity, Entity, &TypeRegistry),
}

impl ReflectComponentFns {
    /// Get the default set of [`ReflectComponentFns`] for a specific component type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Component + Reflect + FromReflect>() -> Self {
        <ReflectComponent as FromType<T>>::from_type().0
    }
}

impl ReflectComponent {
    /// Insert a reflected [`Component`] into the entity like [`insert()`](EntityWorldMut::insert).
    pub fn insert(
        &self,
        entity: &mut EntityWorldMut,
        component: &dyn Reflect,
        registry: &TypeRegistry,
    ) {
        (self.0.insert)(entity, component, registry);
    }

    /// Uses reflection to set the value of this [`Component`] type in the entity to the given value.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Component`] of the given type.
    pub fn apply<'a>(&self, entity: impl Into<EntityMut<'a>>, component: &dyn Reflect) {
        (self.0.apply)(entity.into(), component);
    }

    /// Uses reflection to set the value of this [`Component`] type in the entity to the given value or insert a new one if it does not exist.
    pub fn apply_or_insert(
        &self,
        entity: &mut EntityWorldMut,
        component: &dyn Reflect,
        registry: &TypeRegistry,
    ) {
        (self.0.apply_or_insert)(entity, component, registry);
    }

    /// Removes this [`Component`] type from the entity. Does nothing if it doesn't exist.
    pub fn remove(&self, entity: &mut EntityWorldMut) {
        (self.0.remove)(entity);
    }

    /// Returns whether entity contains this [`Component`]
    pub fn contains<'a>(&self, entity: impl Into<FilteredEntityRef<'a>>) -> bool {
        (self.0.contains)(entity.into())
    }

    /// Gets the value of this [`Component`] type from the entity as a reflected reference.
    pub fn reflect<'a>(&self, entity: impl Into<FilteredEntityRef<'a>>) -> Option<&'a dyn Reflect> {
        (self.0.reflect)(entity.into())
    }

    /// Gets the value of this [`Component`] type from the entity as a mutable reflected reference.
    pub fn reflect_mut<'a>(
        &self,
        entity: impl Into<FilteredEntityMut<'a>>,
    ) -> Option<Mut<'a, dyn Reflect>> {
        (self.0.reflect_mut)(entity.into())
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
        unsafe { (self.0.reflect_unchecked_mut)(entity) }
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
        registry: &TypeRegistry,
    ) {
        (self.0.copy)(
            source_world,
            destination_world,
            source_entity,
            destination_entity,
            registry,
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

    /// The underlying function pointers implementing methods on `ReflectComponent`.
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectComponent>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectComponent`] and keeping it
    /// between frames, cloning a `ReflectComponent` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectComponent`,
    /// use `fn_pointers` to get the underlying [`ReflectComponentFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectComponent>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
    pub fn fn_pointers(&self) -> &ReflectComponentFns {
        &self.0
    }
}

impl<C: Component + Reflect> FromType<C> for ReflectComponent {
    fn from_type() -> Self {
        ReflectComponent(ReflectComponentFns {
            insert: |entity, reflected_component, registry| {
                let component = entity.world_scope(|world| {
                    from_reflect_with_fallback::<C>(reflected_component, world, registry)
                });
                entity.insert(component);
            },
            apply: |mut entity, reflected_component| {
                let mut component = entity.get_mut::<C>().unwrap();
                component.apply(reflected_component);
            },
            apply_or_insert: |entity, reflected_component, registry| {
                if let Some(mut component) = entity.get_mut::<C>() {
                    component.apply(reflected_component);
                } else {
                    let component = entity.world_scope(|world| {
                        from_reflect_with_fallback::<C>(reflected_component, world, registry)
                    });
                    entity.insert(component);
                }
            },
            remove: |entity| {
                entity.remove::<C>();
            },
            contains: |entity| entity.contains::<C>(),
            copy: |source_world, destination_world, source_entity, destination_entity, registry| {
                let source_component = source_world.get::<C>(source_entity).unwrap();
                let destination_component =
                    from_reflect_with_fallback::<C>(source_component, destination_world, registry);
                destination_world
                    .entity_mut(destination_entity)
                    .insert(destination_component);
            },
            reflect: |entity| entity.get::<C>().map(|c| c as &dyn Reflect),
            reflect_mut: |entity| {
                entity.into_mut::<C>().map(|c| Mut {
                    value: c.value as &mut dyn Reflect,
                    ticks: c.ticks,
                })
            },
            reflect_unchecked_mut: |entity| {
                // SAFETY: reflect_unchecked_mut is an unsafe function pointer used by
                // `reflect_unchecked_mut` which must be called with an UnsafeEntityCell with access to the component `C` on the `entity`
                unsafe {
                    entity.get_mut::<C>().map(|c| Mut {
                        value: c.value as &mut dyn Reflect,
                        ticks: c.ticks,
                    })
                }
            },
        })
    }
}
