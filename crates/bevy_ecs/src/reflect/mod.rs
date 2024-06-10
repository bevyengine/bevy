//! Types that enable reflection support.

use std::any::TypeId;
use std::ops::{Deref, DerefMut};

use crate as bevy_ecs;
use crate::{system::Resource, world::World};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::{Reflect, ReflectFromReflect, TypeRegistry, TypeRegistryArc};

mod bundle;
mod component;
mod entity_commands;
mod from_world;
mod map_entities;
mod resource;

pub use bundle::{ReflectBundle, ReflectBundleFns};
pub use component::{ReflectComponent, ReflectComponentFns};
pub use entity_commands::ReflectCommandExt;
pub use from_world::{ReflectFromWorld, ReflectFromWorldFns};
pub use map_entities::{ReflectMapEntities, ReflectMapEntitiesResource};
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

/// Creates a `T` from a `&dyn Reflect`.
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
fn from_reflect_with_fallback<T: Reflect>(
    reflected: &dyn Reflect,
    world: &mut World,
    registry: &TypeRegistry,
) -> T {
    fn different_type_error<T>(reflected: &str) -> ! {
        panic!(
            "The registration for the reflected `{}` trait for the type `{}` produced \
            a value of a different type",
            reflected,
            // FIXME: once we have unique reflect, use `TypePath`.
            std::any::type_name::<T>(),
        );
    }

    // First, try `FromReflect`. This is handled differently from the others because
    // it doesn't need a subsequent `apply` and may fail.
    if let Some(reflect_from_reflect) =
        registry.get_type_data::<ReflectFromReflect>(TypeId::of::<T>())
    {
        // If it fails it's ok, we can continue checking `Default` and `FromWorld`.
        if let Some(value) = reflect_from_reflect.from_reflect(reflected) {
            return value
                .take::<T>()
                .unwrap_or_else(|_| different_type_error::<T>("FromReflect"));
        }
    }

    // Create an instance of `T` using either the reflected `Default` or `FromWorld`.
    let mut value = if let Some(reflect_default) =
        registry.get_type_data::<ReflectDefault>(TypeId::of::<T>())
    {
        reflect_default
            .default()
            .take::<T>()
            .unwrap_or_else(|_| different_type_error::<T>("Default"))
    } else if let Some(reflect_from_world) =
        registry.get_type_data::<ReflectFromWorld>(TypeId::of::<T>())
    {
        reflect_from_world
            .from_world(world)
            .take::<T>()
            .unwrap_or_else(|_| different_type_error::<T>("FromWorld"))
    } else {
        panic!(
            "Couldn't create an instance of `{}` using the reflected `FromReflect`, \
            `Default` or `FromWorld` traits. Are you perhaps missing a `#[reflect(Default)]` \
            or `#[reflect(FromWorld)]`?",
            // FIXME: once we have unique reflect, use `TypePath`.
            std::any::type_name::<T>(),
        );
    };

    value.apply(reflected);
    value
}
