//! Types that enable reflection support.

use core::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

use crate::{resource::Resource, world::World};
use bevy_reflect::{
    dynamic::DynamicValue, std_traits::ReflectDefault, Reflect, ReflectFromDynamic, TypePath,
    TypeRegistry, TypeRegistryArc,
};

mod bundle;
mod component;
mod entity_commands;
mod event;
mod from_world;
mod map_entities;
mod message;
mod resource;

use bevy_utils::prelude::DebugName;
pub use bundle::{ReflectBundle, ReflectBundleFns};
pub use component::{ReflectComponent, ReflectComponentFns};
pub use entity_commands::ReflectCommandExt;
pub use event::{ReflectEvent, ReflectEventFns};
pub use from_world::{ReflectFromWorld, ReflectFromWorldFns};
pub use map_entities::ReflectMapEntities;
pub use message::{ReflectMessage, ReflectMessageFns};
pub use resource::ReflectResource;
use thiserror::Error;

/// A [`Resource`] storing [`TypeRegistry`] for
/// type registrations relevant to a whole app.
#[derive(Resource, Clone, Default)]
pub struct AppTypeRegistry(pub TypeRegistryArc);

impl Deref for AppTypeRegistry {
    type Target = TypeRegistryArc;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AppTypeRegistry {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AppTypeRegistry {
    /// Creates [`AppTypeRegistry`] and automatically registers all types deriving [`Reflect`].
    ///
    /// See [`TypeRegistry::register_derived_types`] for more details.
    #[cfg(feature = "reflect_auto_register")]
    pub fn new_with_derived_types() -> Self {
        let app_registry = AppTypeRegistry::default();
        app_registry.write().register_derived_types();
        app_registry
    }
}

/// A [`Resource`] storing [`FunctionRegistry`] for
/// function registrations relevant to a whole app.
///
/// [`FunctionRegistry`]: bevy_reflect::func::FunctionRegistry
#[cfg(feature = "reflect_functions")]
#[derive(Resource, Clone, Default)]
pub struct AppFunctionRegistry(pub bevy_reflect::func::FunctionRegistryArc);

#[cfg(feature = "reflect_functions")]
impl Deref for AppFunctionRegistry {
    type Target = bevy_reflect::func::FunctionRegistryArc;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "reflect_functions")]
impl DerefMut for AppFunctionRegistry {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Creates a reflected value from a correponding [`DynamicValue`].
///
/// This will try the following strategies, in this order:
///
/// - for concrete values, tries to downcast the value, and fails if the downcast fails;
/// - use the reflected `FromDynamic`, if it's present and doesn't fail;
/// - use the reflected `Default`, if it's present, and then call `apply` on the result;
/// - use the reflected `FromWorld`, just like the `Default`.
///
/// The first one that is present and doesn't fail will be used.
#[inline(never)]
pub fn from_dynamic_with_fallback_erased(
    dynamic_value: DynamicValue,
    world: &mut World,
    registry: &TypeRegistry,
    id: TypeId,
    name: DebugName,
) -> Result<alloc::boxed::Box<dyn Reflect>, FromDynamicWithFallbackError> {
    if let DynamicValue::Concrete(concrete) = dynamic_value {
        if concrete.as_any().type_id() != id {
            return Err(FromDynamicWithFallbackError::FailedDowncast {
                concrete_type: concrete.reflect_type_info().type_path().into(),
                expected_type: name,
            });
        }
        return Ok(concrete);
    }

    // First, try `FromDynamic`. This is handled differently from the others because
    // it doesn't need a subsequent `apply` and may fail.
    // If it fails it's ok, we can continue checking `Default` and `FromWorld`.
    let (value, source) = if let Some(value) = registry
        .get_type_data::<ReflectFromDynamic>(id)
        .and_then(|reflect_from_dynamic| {
            reflect_from_dynamic.from_dynamic(dynamic_value.dynamic_clone().ok()?)
        }) {
        (value, "FromDynamic")
    }
    // Create an instance of `T` using either the reflected `Default` or `FromWorld`.
    else if let Some(reflect_default) = registry.get_type_data::<ReflectDefault>(id) {
        let mut value = reflect_default.default();
        dynamic_value.try_apply(&mut *value).unwrap();
        (value, "Default")
    } else if let Some(reflect_from_world) = registry.get_type_data::<ReflectFromWorld>(id) {
        let mut value = reflect_from_world.from_world(world);
        dynamic_value.try_apply(&mut *value).unwrap();
        (value, "FromWorld")
    } else {
        panic!(
            "Couldn't create an instance of `{name}` using the reflected `FromDynamic`, \
                `Default` or `FromWorld` traits. Are you perhaps missing a `#[reflect(Default)]` \
                or `#[reflect(FromWorld)]`?",
        );
    };
    if value.as_any().type_id() != id {
        return Err(FromDynamicWithFallbackError::BadTypeId {
            concrete_type: value.reflect_type_info().type_path().into(),
            expected_type: name,
            trait_source: source,
        });
    }
    Ok(value)
}

/// Creates a `T` from a [`DynamicValue`].
///
/// This will try the following strategies, in this order:
///
/// - for concrete values, tries to downcast the value, and fails if the downcast fails;
/// - use the reflected `FromDynamic`, if it's present and doesn't fail;
/// - use the reflected `Default`, if it's present, and then call `apply` on the result;
/// - use the reflected `FromWorld`, just like the `Default`.
///
/// The first one that is present and doesn't fail will be used.
///
/// # Panics
///
/// If any strategy produces a `Box<dyn Reflect>` that doesn't store a value of type `T`
/// this method will panic.
///
/// If none of the strategies succeed, this method will panic.
pub fn from_dynamic_with_fallback<T: Reflect + TypePath>(
    dynamic_value: DynamicValue,
    world: &mut World,
    registry: &TypeRegistry,
) -> T {
    *from_dynamic_with_fallback_erased(
        dynamic_value,
        world,
        registry,
        TypeId::of::<T>(),
        // FIXME: once we have unique reflect, use `TypePath`.
        DebugName::type_name::<T>(),
    )
    .unwrap()
    .downcast::<T>()
    .unwrap()
}

/// Error type for [`from_dynamic_with_fallback_erased`].
#[derive(Error, Debug, Clone)]
pub enum FromDynamicWithFallbackError {
    /// The concrete value failed to downcast.
    #[error("the provided concrete value was the wrong type. Expected {expected_type}, got {concrete_type}")]
    FailedDowncast {
        /// The concrete type passed in.
        concrete_type: DebugName,
        /// The value that was expected to be casted into.
        expected_type: DebugName,
    },
    /// One of the reflected traits successfully converted, but produced a different type.
    #[error("the reflected trait {trait_source} produced the wrong type. Expected {expected_type}, got {concrete_type}")]
    BadTypeId {
        /// The concrete type passed in.
        concrete_type: DebugName,
        /// The value that was expected to be casted into.
        expected_type: DebugName,
        /// The trait that produced the wrong type.
        trait_source: &'static str,
    },
}
