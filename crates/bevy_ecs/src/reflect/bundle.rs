//! Definitions for [`Bundle`] reflection.
//! This allows inserting, updating and/or removing bundles whose type is only known at runtime.
//!
//! This module exports two types: [`ReflectBundleFns`] and [`ReflectBundle`].
//!
//! Same as [`super::component`], but for bundles.
use alloc::boxed::Box;
use bevy_utils::prelude::DebugName;
use core::any::{Any, TypeId};

use crate::{
    bundle::BundleFromComponents,
    entity::EntityMapper,
    prelude::Bundle,
    relationship::RelationshipHookMode,
    world::{EntityMut, EntityWorldMut},
};
use bevy_reflect::{
    FromReflect, FromType, PartialReflect, Reflect, ReflectRef, TypePath, TypeRegistry,
};

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
    /// Function pointer implementing [`ReflectBundle::insert`].
    pub insert: fn(&mut EntityWorldMut, &dyn PartialReflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectBundle::apply`].
    pub apply: fn(EntityMut, &dyn PartialReflect, &TypeRegistry),
    /// Function pointer implementing [`ReflectBundle::apply_or_insert_mapped`].
    pub apply_or_insert_mapped: fn(
        &mut EntityWorldMut,
        &dyn PartialReflect,
        &TypeRegistry,
        &mut dyn EntityMapper,
        RelationshipHookMode,
    ),
    /// Function pointer implementing [`ReflectBundle::remove`].
    pub remove: fn(&mut EntityWorldMut),
    /// Function pointer implementing [`ReflectBundle::take`].
    pub take: fn(&mut EntityWorldMut) -> Option<Box<dyn Reflect>>,
}

impl ReflectBundleFns {
    /// Get the default set of [`ReflectBundleFns`] for a specific bundle type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Bundle + FromReflect + TypePath + BundleFromComponents>() -> Self {
        <ReflectBundle as FromType<T>>::from_type().0
    }
}

impl ReflectBundle {
    /// Insert a reflected [`Bundle`] into the entity like [`insert()`](EntityWorldMut::insert).
    pub fn insert(
        &self,
        entity: &mut EntityWorldMut,
        bundle: &dyn PartialReflect,
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
        bundle: &dyn PartialReflect,
        registry: &TypeRegistry,
    ) {
        (self.0.apply)(entity.into(), bundle, registry);
    }

    /// Uses reflection to set the value of this [`Bundle`] type in the entity to the given value or insert a new one if it does not exist.
    pub fn apply_or_insert_mapped(
        &self,
        entity: &mut EntityWorldMut,
        bundle: &dyn PartialReflect,
        registry: &TypeRegistry,
        mapper: &mut dyn EntityMapper,
        relationship_hook_mode: RelationshipHookMode,
    ) {
        (self.0.apply_or_insert_mapped)(entity, bundle, registry, mapper, relationship_hook_mode);
    }

    /// Removes this [`Bundle`] type from the entity. Does nothing if it doesn't exist.
    pub fn remove(&self, entity: &mut EntityWorldMut) -> &ReflectBundle {
        (self.0.remove)(entity);
        self
    }

    /// Removes all components in the [`Bundle`] from the entity and returns their previous values.
    ///
    /// **Note:** If the entity does not have every component in the bundle, this method will not remove any of them.
    #[must_use]
    pub fn take(&self, entity: &mut EntityWorldMut) -> Option<Box<dyn Reflect>> {
        (self.0.take)(entity)
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

impl<B: Bundle + Reflect + TypePath + BundleFromComponents> FromType<B> for ReflectBundle {
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
                            DebugName::type_name::<B>(),
                        ),
                    }
                }
            },
            apply_or_insert_mapped: |entity,
                                     reflected_bundle,
                                     registry,
                                     mapper,
                                     relationship_hook_mode| {
                if let Some(reflect_component) =
                    registry.get_type_data::<ReflectComponent>(TypeId::of::<B>())
                {
                    reflect_component.apply_or_insert_mapped(
                        entity,
                        reflected_bundle,
                        registry,
                        mapper,
                        relationship_hook_mode,
                    );
                } else {
                    match reflected_bundle.reflect_ref() {
                        ReflectRef::Struct(bundle) => bundle.iter_fields().for_each(|field| {
                            apply_or_insert_field_mapped(
                                entity,
                                field,
                                registry,
                                mapper,
                                relationship_hook_mode,
                            );
                        }),
                        ReflectRef::Tuple(bundle) => bundle.iter_fields().for_each(|field| {
                            apply_or_insert_field_mapped(
                                entity,
                                field,
                                registry,
                                mapper,
                                relationship_hook_mode,
                            );
                        }),
                        _ => panic!(
                            "expected bundle `{}` to be a named struct or tuple",
                            // FIXME: once we have unique reflect, use `TypePath`.
                            DebugName::type_name::<B>(),
                        ),
                    }
                }
            },
            remove: |entity| {
                entity.remove::<B>();
            },
            take: |entity| {
                entity
                    .take::<B>()
                    .map(|bundle| Box::new(bundle).into_reflect())
            },
        })
    }
}

fn apply_field(entity: &mut EntityMut, field: &dyn PartialReflect, registry: &TypeRegistry) {
    let Some(type_id) = field.try_as_reflect().map(Any::type_id) else {
        panic!(
            "`{}` did not implement `Reflect`",
            field.reflect_type_path()
        );
    };
    if let Some(reflect_component) = registry.get_type_data::<ReflectComponent>(type_id) {
        reflect_component.apply(entity.reborrow(), field);
    } else if let Some(reflect_bundle) = registry.get_type_data::<ReflectBundle>(type_id) {
        reflect_bundle.apply(entity.reborrow(), field, registry);
    } else {
        panic!(
            "no `ReflectComponent` nor `ReflectBundle` registration found for `{}`",
            field.reflect_type_path()
        );
    }
}

fn apply_or_insert_field_mapped(
    entity: &mut EntityWorldMut,
    field: &dyn PartialReflect,
    registry: &TypeRegistry,
    mapper: &mut dyn EntityMapper,
    relationship_hook_mode: RelationshipHookMode,
) {
    let Some(type_id) = field.try_as_reflect().map(Any::type_id) else {
        panic!(
            "`{}` did not implement `Reflect`",
            field.reflect_type_path()
        );
    };

    if let Some(reflect_component) = registry.get_type_data::<ReflectComponent>(type_id) {
        reflect_component.apply_or_insert_mapped(
            entity,
            field,
            registry,
            mapper,
            relationship_hook_mode,
        );
    } else if let Some(reflect_bundle) = registry.get_type_data::<ReflectBundle>(type_id) {
        reflect_bundle.apply_or_insert_mapped(
            entity,
            field,
            registry,
            mapper,
            relationship_hook_mode,
        );
    } else {
        let is_component = entity.world().components().get_id(type_id).is_some();

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
