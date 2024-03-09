//! Types that enable reflection support.
//!
//! # Automatic Reflect Type Registration
//! The [`Component`] and [`Resource`] resource derive macros will automatically register
//! its types that implement [`Reflect`] into the [`AppTypeRegistry`] resource when first
//! seen by the ECS [`World`].
//!
//! If you find that your component or resource is not registered, they may need to be manually
//! registered. There are a few exceptions:
//!
//!  * Automatic registration is only supported via the derive macros. Manual implementations of
//!    `Component`, `Resource`, or `Reflect` must be manually registered.
//!  * The associated ECS trait must be reflected (via `reflect(Component)` or `reflect(Resource)`).
//!  * Generic types are not supported, and must be manually registered.
//!  * Types are registered when the World first initializes the type. This may cause registrations
//!    to be missing due to mistiming. These initialization points include but are not limited to:
//!    - spawning an entity with the component or inserting the resource
//!    - inserting the component existing entity
//!    - attempting to remove the component or resource, even if it's not present.
//!    - a system that references the component or resource is added to a schedule
//!
//! ```rust
//! use bevy_ecs::prelude::*;
//! use bevy_reflect::Reflect;
//!
//! // This will automatically register upon first use!
//! #[derive(Component, Reflect)]
//! #[reflect(Component)]
//! pub struct MyComponent {
//!     a: usize,
//!     b: (u32, u8)
//! }
//!
//! // This won't!
//! #[derive(Component, Reflect)]
//! #[reflect(Component)]
//! pub struct GenericComponent<T>(T);
//!
//! // This won't!
//! #[derive(Component, Reflect)]
//! pub struct NoReflectComponent;
//! ```
//!
//! [`Component`]: crate::prelude::Component
//! [`Resource`]: crate::prelude::Resource

use std::any::TypeId;
use std::ops::{Deref, DerefMut};

use crate as bevy_ecs;
use crate::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_reflect::{FromReflect, GetTypeRegistration, Reflect, TypeRegistry, TypeRegistryArc};

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

#[doc(hidden)]
pub fn register_type_shim<T: GetTypeRegistration>(registry: &TypeRegistryArc) {
    if let Ok(mut registry) = registry.internal.try_write() {
        registry.register::<T>();
        return;
    }
    if let Ok(registry) = registry.internal.try_read() {
        if registry.contains(::core::any::TypeId::of::<T>()) {
            return;
        }
    }
    panic!(
        "Deadlock while registering <{}>.",
        ::std::any::type_name::<T>()
    );
}

/// A [`Resource`] storing [`TypeRegistry`] for
/// type registrations relevant to a whole app.
#[derive(Resource, Clone)]
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

impl FromWorld for AppTypeRegistry {
    fn from_world(world: &mut World) -> Self {
        Self(world.__type_registry().clone())
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
