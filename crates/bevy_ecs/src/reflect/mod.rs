//! Types that enable reflection support.

use std::any::TypeId;
use std::ops::{Deref, DerefMut};

use crate as bevy_ecs;
use crate::{system::Resource, world::World};
use bevy_reflect::{FromReflect, Reflect, TypeRegistry, TypeRegistryArc};

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

/// Creates a `T` from a `&dyn Reflect`.
///
/// The first approach uses `T`'s implementation of `FromReflect`.
/// If this fails, it falls back to default-initializing a new instance of `T` using its
/// `ReflectFromWorld` data from the `world`'s `AppTypeRegistry` and `apply`ing the
/// `&dyn Reflect` on it.
///
/// Panics if both approaches fail.
fn from_reflect_or_world<T: FromReflect>(
    reflected: &dyn Reflect,
    world: &mut World,
    registry: &TypeRegistry,
) -> T {
    if let Some(value) = T::from_reflect(reflected) {
        return value;
    }

    // Clone the `ReflectFromWorld` because it's cheap and "frees"
    // the borrow of `world` so that it can be passed to `from_world`.
    let Some(reflect_from_world) = registry.get_type_data::<ReflectFromWorld>(TypeId::of::<T>())
    else {
        panic!(
            "`FromReflect` failed and no `ReflectFromWorld` registration found for `{}`",
            // FIXME: once we have unique reflect, use `TypePath`.
            std::any::type_name::<T>(),
        );
    };

    let Ok(mut value) = reflect_from_world.from_world(world).take::<T>() else {
        panic!(
            "the `ReflectFromWorld` registration for `{}` produced a value of a different type",
            // FIXME: once we have unique reflect, use `TypePath`.
            std::any::type_name::<T>(),
        );
    };

    value.apply(reflected);
    value
}
