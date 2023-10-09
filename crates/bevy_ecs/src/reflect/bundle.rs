//! Definitions for [`Bundle`] reflection.
//!
//! This module exports two types: [`ReflectBundleFns`] and [`ReflectBundle`].
//!
//! Same as [`super::component`], but for bundles.
use std::any::TypeId;

use crate::{
    prelude::Bundle,
    world::{EntityWorldMut, FromWorld, World},
};
use bevy_reflect::{FromType, Reflect, ReflectRef, TypeRegistry};

use super::ReflectComponent;

/// A struct used to operate on reflected [`Bundle`] of a type.
///
/// A [`ReflectBundle`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectBundle(ReflectBundleFns);

/// The raw function pointers needed to make up a [`ReflectBundle`].
///
/// The also [`super::component::ReflectComponentFns`].
#[derive(Clone)]
pub struct ReflectBundleFns {
    /// Function pointer implementing [`ReflectBundle::from_world()`].
    pub from_world: fn(&mut World) -> Box<dyn Reflect>,
    /// Function pointer implementing [`ReflectBundle::insert()`].
    pub insert: fn(&mut EntityWorldMut, &dyn Reflect),
    /// Function pointer implementing [`ReflectBundle::apply()`].
    pub apply: fn(&mut EntityWorldMut, &dyn Reflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectBundle::apply_or_insert()`].
    pub apply_or_insert: fn(&mut EntityWorldMut, &dyn Reflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectBundle::remove()`].
    pub remove: fn(&mut EntityWorldMut),
}

impl ReflectBundleFns {
    /// Get the default set of [`ReflectBundleFns`] for a specific bundle type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Bundle + Reflect + FromWorld>() -> Self {
        <ReflectBundle as FromType<T>>::from_type().0
    }
}

impl ReflectBundle {
    /// Constructs default reflected [`Bundle`] from world using [`from_world()`](FromWorld::from_world).
    pub fn from_world(&self, world: &mut World) -> Box<dyn Reflect> {
        (self.0.from_world)(world)
    }

    /// Insert a reflected [`Bundle`] into the entity like [`insert()`](crate::world::EntityWorldMut::insert).
    pub fn insert(&self, entity: &mut EntityWorldMut, bundle: &dyn Reflect) {
        (self.0.insert)(entity, bundle);
    }

    /// Uses reflection to set the value of this [`Bundle`] type in the entity to the given value.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Bundle`] of the given type.
    pub fn apply(
        &self,
        entity: &mut EntityWorldMut,
        bundle: &dyn Reflect,
        registry: &TypeRegistry,
    ) {
        (self.0.apply)(entity, bundle, registry);
    }

    /// Uses reflection to set the value of this [`Bundle`] type in the entity to the given value or insert a new one if it does not exist.
    pub fn apply_or_insert(
        &self,
        entity: &mut EntityWorldMut,
        bundle: &dyn Reflect,
        registry: &TypeRegistry,
    ) {
        (self.0.apply_or_insert)(entity, bundle, registry);
    }

    /// Removes this [`Bundle`] type from the entity. Does nothing if it doesn't exist.
    pub fn remove(&self, entity: &mut EntityWorldMut) {
        (self.0.remove)(entity);
    }

    /// Create a custom implementation of [`ReflectBundle`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(Bundle)]` bundle
    /// to generate a [`ReflectBundle`] implementation automatically.
    ///
    /// See [`ReflectBundleFns`] for more information.
    pub fn new(fns: ReflectBundleFns) -> Self {
        Self(fns)
    }

    /// The underlying function pointers implementing methods on `ReflectBundle`.
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectBundle>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectBundle`] and keeping it
    /// between frames, cloning a `ReflectBundle` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectBundle`,
    /// use `fn_pointers` to get the underlying [`ReflectBundleFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectBundle>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
    pub fn fn_pointers(&self) -> &ReflectBundleFns {
        &self.0
    }
}

impl<B: Bundle + Reflect + FromWorld> FromType<B> for ReflectBundle {
    fn from_type() -> Self {
        ReflectBundle(ReflectBundleFns {
            from_world: |world| Box::new(B::from_world(world)),
            insert: |entity, reflected_bundle| {
                let mut bundle = entity.world_scope(|world| B::from_world(world));
                bundle.apply(reflected_bundle);
                entity.insert(bundle);
            },
            apply: |entity, reflected_bundle, registry| {
                let mut bundle = entity.world_scope(|world| B::from_world(world));
                bundle.apply(reflected_bundle);

                match bundle.reflect_ref() {
                    ReflectRef::Struct(bundle) => bundle
                        .iter_fields()
                        .for_each(|field| insert_field::<B>(entity, field, registry)),
                    ReflectRef::Tuple(bundle) => bundle
                        .iter_fields()
                        .for_each(|field| insert_field::<B>(entity, field, registry)),
                    _ => panic!(
                        "expected bundle `{}` to be named struct or tuple",
                        std::any::type_name::<B>()
                    ),
                }
            },
            apply_or_insert: |entity, reflected_bundle, registry| {
                let mut bundle = entity.world_scope(|world| B::from_world(world));
                bundle.apply(reflected_bundle);

                match bundle.reflect_ref() {
                    ReflectRef::Struct(bundle) => bundle
                        .iter_fields()
                        .for_each(|field| apply_or_insert_field::<B>(entity, field, registry)),
                    ReflectRef::Tuple(bundle) => bundle
                        .iter_fields()
                        .for_each(|field| apply_or_insert_field::<B>(entity, field, registry)),
                    _ => panic!(
                        "expected bundle `{}` to be named struct or tuple",
                        std::any::type_name::<B>()
                    ),
                }
            },
            remove: |entity| {
                entity.remove::<B>();
            },
        })
    }
}

fn insert_field<B: 'static>(
    entity: &mut EntityWorldMut,
    field: &dyn Reflect,
    registry: &TypeRegistry,
) {
    if let Some(reflect_component) = registry.get_type_data::<ReflectComponent>(field.type_id()) {
        reflect_component.apply(entity, field);
    } else if let Some(reflect_bundle) = registry.get_type_data::<ReflectBundle>(field.type_id()) {
        reflect_bundle.apply(entity, field, registry);
    } else {
        entity.world_scope(|world| {
            if world.components().get_id(TypeId::of::<B>()).is_some() {
                panic!(
                    "no `ReflectComponent` registration found for `{}`",
                    field.type_name()
                );
            };
        });

        panic!(
            "no `ReflectBundle` registration found for `{}`",
            field.type_name()
        )
    }
}

fn apply_or_insert_field<B: 'static>(
    entity: &mut EntityWorldMut,
    field: &dyn Reflect,
    registry: &TypeRegistry,
) {
    if let Some(reflect_component) = registry.get_type_data::<ReflectComponent>(field.type_id()) {
        reflect_component.apply_or_insert(entity, field);
    } else if let Some(reflect_bundle) = registry.get_type_data::<ReflectBundle>(field.type_id()) {
        reflect_bundle.apply_or_insert(entity, field, registry);
    } else {
        entity.world_scope(|world| {
            if world.components().get_id(TypeId::of::<B>()).is_some() {
                panic!(
                    "no `ReflectComponent` registration found for `{}`",
                    field.type_name()
                );
            };
        });

        panic!(
            "no `ReflectBundle` registration found for `{}`",
            field.type_name()
        )
    }
}
