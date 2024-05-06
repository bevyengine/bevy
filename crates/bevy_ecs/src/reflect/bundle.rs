//! Definitions for [`Bundle`] reflection.
//! This allows inserting, updating and/or removing bundles whose type is only known at runtime.
//!
//! This module exports two types: [`ReflectBundleFns`] and [`ReflectBundle`].
//!
//! Same as [`super::component`], but for bundles.
use std::any::TypeId;

use crate::{
    prelude::Bundle,
    world::{EntityMut, EntityWorldMut},
};
use bevy_reflect::{FromReflect, FromType, Reflect, ReflectRef, TypeRegistry};

use super::{from_reflect_with_fallback, ReflectComponent};

/// A struct used to operate on reflected [`Bundle`] trait of a type.
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
    /// Function pointer implementing [`ReflectBundle::insert()`].
    pub insert: fn(&mut EntityWorldMut, &dyn Reflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectBundle::apply()`].
    pub apply: fn(EntityMut, &dyn Reflect, &TypeRegistry),
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
    pub fn new<T: Bundle + Reflect + FromReflect>() -> Self {
        <ReflectBundle as FromType<T>>::from_type().0
    }
}

impl ReflectBundle {
    /// Insert a reflected [`Bundle`] into the entity like [`insert()`](EntityWorldMut::insert).
    pub fn insert(
        &self,
        entity: &mut EntityWorldMut,
        bundle: &dyn Reflect,
        registry: &TypeRegistry,
    ) {
        (self.0.insert)(entity, bundle, registry);
    }

    /// Uses reflection to set the value of this [`Bundle`] type in the entity to the given value.
    ///
    /// # Panics
    ///
    /// Panics if there is no [`Bundle`] of the given type.
    pub fn apply<'a>(
        &self,
        entity: impl Into<EntityMut<'a>>,
        bundle: &dyn Reflect,
        registry: &TypeRegistry,
    ) {
        (self.0.apply)(entity.into(), bundle, registry);
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
    pub fn fn_pointers(&self) -> &ReflectBundleFns {
        &self.0
    }
}

impl<B: Bundle + Reflect> FromType<B> for ReflectBundle {
    fn from_type() -> Self {
        ReflectBundle(ReflectBundleFns {
            insert: |entity, reflected_bundle, registry| {
                let bundle = entity.world_scope(|world| {
                    from_reflect_with_fallback::<B>(reflected_bundle, world, registry)
                });
                entity.insert(bundle);
            },
            apply: |mut entity, reflected_bundle, registry| {
                if let Some(reflect_component) =
                    registry.get_type_data::<ReflectComponent>(TypeId::of::<B>())
                {
                    reflect_component.apply(entity, reflected_bundle);
                } else {
                    match reflected_bundle.reflect_ref() {
                        ReflectRef::Struct(bundle) => bundle
                            .iter_fields()
                            .for_each(|field| apply_field(&mut entity, field, registry)),
                        ReflectRef::Tuple(bundle) => bundle
                            .iter_fields()
                            .for_each(|field| apply_field(&mut entity, field, registry)),
                        _ => panic!(
                            "expected bundle `{}` to be named struct or tuple",
                            // FIXME: once we have unique reflect, use `TypePath`.
                            std::any::type_name::<B>(),
                        ),
                    }
                }
            },
            apply_or_insert: |entity, reflected_bundle, registry| {
                if let Some(reflect_component) =
                    registry.get_type_data::<ReflectComponent>(TypeId::of::<B>())
                {
                    reflect_component.apply_or_insert(entity, reflected_bundle, registry);
                } else {
                    match reflected_bundle.reflect_ref() {
                        ReflectRef::Struct(bundle) => bundle
                            .iter_fields()
                            .for_each(|field| apply_or_insert_field(entity, field, registry)),
                        ReflectRef::Tuple(bundle) => bundle
                            .iter_fields()
                            .for_each(|field| apply_or_insert_field(entity, field, registry)),
                        _ => panic!(
                            "expected bundle `{}` to be named struct or tuple",
                            // FIXME: once we have unique reflect, use `TypePath`.
                            std::any::type_name::<B>(),
                        ),
                    }
                }
            },
            remove: |entity| {
                entity.remove::<B>();
            },
        })
    }
}

fn apply_field(entity: &mut EntityMut, field: &dyn Reflect, registry: &TypeRegistry) {
    if let Some(reflect_component) = registry.get_type_data::<ReflectComponent>(field.type_id()) {
        reflect_component.apply(entity.reborrow(), field);
    } else if let Some(reflect_bundle) = registry.get_type_data::<ReflectBundle>(field.type_id()) {
        reflect_bundle.apply(entity.reborrow(), field, registry);
    } else {
        panic!(
            "no `ReflectComponent` nor `ReflectBundle` registration found for `{}`",
            field.reflect_type_path()
        );
    }
}

fn apply_or_insert_field(
    entity: &mut EntityWorldMut,
    field: &dyn Reflect,
    registry: &TypeRegistry,
) {
    if let Some(reflect_component) = registry.get_type_data::<ReflectComponent>(field.type_id()) {
        reflect_component.apply_or_insert(entity, field, registry);
    } else if let Some(reflect_bundle) = registry.get_type_data::<ReflectBundle>(field.type_id()) {
        reflect_bundle.apply_or_insert(entity, field, registry);
    } else {
        let is_component = entity
            .world()
            .components()
            .get_id(field.type_id())
            .is_some();

        if is_component {
            panic!(
                "no `ReflectComponent` registration found for `{}`",
                field.reflect_type_path(),
            );
        } else {
            panic!(
                "no `ReflectBundle` registration found for `{}`",
                field.reflect_type_path(),
            )
        }
    }
}
