//! Types that enable reflection support.

use core::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

use crate::{resource::Resource, world::World};
use bevy_reflect::{
    std_traits::ReflectDefault, PartialReflect, Reflect, ReflectFromReflect, TypePath,
    TypeRegistry, TypeRegistryArc,
};

mod bundle;
mod component;
mod entity_commands;
mod from_world;
mod map_entities;
mod resource;

use bevy_utils::prelude::DebugName;
pub use bundle::{ReflectBundle, ReflectBundleFns};
pub use component::{ReflectComponent, ReflectComponentFns};
pub use entity_commands::ReflectCommandExt;
pub use from_world::{ReflectFromWorld, ReflectFromWorldFns};
pub use map_entities::ReflectMapEntities;
pub use resource::{ReflectResource, ReflectResourceFns};

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

/// Creates a `T` from a `&dyn PartialReflect`.
///
/// This will try the following strategies, in this order:
///
/// - use the reflected `FromReflect`, if it's present and doesn't fail;
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
pub fn from_reflect_with_fallback<T: Reflect + TypePath>(
    reflected: &dyn PartialReflect,
    world: &mut World,
    registry: &TypeRegistry,
) -> T {
    #[inline(never)]
    fn type_erased(
        reflected: &dyn PartialReflect,
        world: &mut World,
        registry: &TypeRegistry,
        id: TypeId,
        name: DebugName,
    ) -> alloc::boxed::Box<dyn core::any::Any> {
        // First, try `FromReflect`. This is handled differently from the others because
        // it doesn't need a subsequent `apply` and may fail.
        // If it fails it's ok, we can continue checking `Default` and `FromWorld`.
        let (value, source) = if let Some(value) = registry
            .get_type_data::<ReflectFromReflect>(id)
            .and_then(|reflect_from_reflect| reflect_from_reflect.from_reflect(reflected))
        {
            (value, "FromReflect")
        }
        // Create an instance of `T` using either the reflected `Default` or `FromWorld`.
        else if let Some(reflect_default) = registry.get_type_data::<ReflectDefault>(id) {
            let mut value = reflect_default.default();
            value.apply(reflected);
            (value, "Default")
        } else if let Some(reflect_from_world) = registry.get_type_data::<ReflectFromWorld>(id) {
            let mut value = reflect_from_world.from_world(world);
            value.apply(reflected);
            (value, "FromWorld")
        } else {
            panic!(
                "Couldn't create an instance of `{name}` using the reflected `FromReflect`, \
                `Default` or `FromWorld` traits. Are you perhaps missing a `#[reflect(Default)]` \
                or `#[reflect(FromWorld)]`?",
            );
        };
        assert_eq!(
            value.as_any().type_id(),
            id,
            "The registration for the reflected `{source}` trait for the type `{name}` produced \
            a value of a different type",
        );
        value
    }
    *type_erased(
        reflected,
        world,
        registry,
        TypeId::of::<T>(),
        // FIXME: once we have unique reflect, use `TypePath`.
        DebugName::type_name::<T>(),
    )
    .downcast::<T>()
    .unwrap()
}
