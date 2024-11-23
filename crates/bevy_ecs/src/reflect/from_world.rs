//! Definitions for [`FromWorld`] reflection.
//! This allows creating instances of types that are known only at runtime and
//! require an `&mut World` to be initialized.
//!
//! This module exports two types: [`ReflectFromWorldFns`] and [`ReflectFromWorld`].
//!
//! Same as [`super::component`], but for [`FromWorld`].

use bevy_reflect::{FromType, Reflect};

use crate::world::{FromWorld, World};

/// A struct used to operate on the reflected [`FromWorld`] trait of a type.
///
/// A [`ReflectFromWorld`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectFromWorld(ReflectFromWorldFns);

/// The raw function pointers needed to make up a [`ReflectFromWorld`].
#[derive(Clone)]
pub struct ReflectFromWorldFns {
    /// Function pointer implementing [`ReflectFromWorld::from_world()`].
    pub from_world: fn(&mut World) -> Box<dyn Reflect>,
}

impl ReflectFromWorldFns {
    /// Get the default set of [`ReflectFromWorldFns`] for a specific type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Reflect + FromWorld>() -> Self {
        <ReflectFromWorld as FromType<T>>::from_type().0
    }
}

impl ReflectFromWorld {
    /// Constructs default reflected [`FromWorld`] from world using [`from_world()`](FromWorld::from_world).
    pub fn from_world(&self, world: &mut World) -> Box<dyn Reflect> {
        (self.0.from_world)(world)
    }

    /// Create a custom implementation of [`ReflectFromWorld`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(FromWorld)]` bundle
    /// to generate a [`ReflectFromWorld`] implementation automatically.
    ///
    /// See [`ReflectFromWorldFns`] for more information.
    pub fn new(fns: ReflectFromWorldFns) -> Self {
        Self(fns)
    }

    /// The underlying function pointers implementing methods on `ReflectFromWorld`.
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectFromWorld>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectFromWorld`] and keeping it
    /// between frames, cloning a `ReflectFromWorld` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectFromWorld`,
    /// use `fn_pointers` to get the underlying [`ReflectFromWorldFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectFromWorld>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
    pub fn fn_pointers(&self) -> &ReflectFromWorldFns {
        &self.0
    }
}

impl<B: Reflect + FromWorld> FromType<B> for ReflectFromWorld {
    fn from_type() -> Self {
        ReflectFromWorld(ReflectFromWorldFns {
            from_world: |world| Box::new(B::from_world(world)),
        })
    }
}
