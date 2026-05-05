//! Definitions for [`Message`] reflection.
//! This allows writing messages whose type is only known at runtime.
//!
//! This module exports two types: [`ReflectMessageFns`] and [`ReflectMessage`].
//!
//! Same as [`component`](`super::component`), but for messages.

use crate::{message::Message, reflect::from_reflect_with_fallback, world::World};
use bevy_reflect::{FromReflect, FromType, PartialReflect, Reflect, TypePath, TypeRegistry};

/// A struct used to operate on reflected [`Message`] trait of a type.
///
/// A [`ReflectMessage`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectMessage(ReflectMessageFns);

/// The raw function pointers needed to make up a [`ReflectMessage`].
///
/// This is used when creating custom implementations of [`ReflectMessage`] with
/// [`ReflectMessage::new()`].
///
/// > **Note:**
/// > Creating custom implementations of [`ReflectMessage`] is an advanced feature
/// > that most users will not need. Usually a [`ReflectMessage`] is created for a
/// > type by deriving [`Reflect`] and adding the `#[reflect(Message)]` attribute.
/// > After adding the event to the [`TypeRegistry`], its [`ReflectMessage`] can
/// > then be retrieved when needed.
///
/// Creating a custom [`ReflectMessage`] may be useful if you need to create new
/// message types at runtime, for example, for scripting implementations.
///
/// By creating a custom [`ReflectMessage`] and inserting it into a type's
/// [`TypeRegistration`][bevy_reflect::TypeRegistration], you can modify the way
/// that reflected messages of that type will be written to the Bevy world.
#[derive(Clone)]
pub struct ReflectMessageFns {
    /// Function pointer implementing [`ReflectMessage::write_message`].
    pub write_message: fn(&mut World, &dyn PartialReflect, &TypeRegistry),
}

impl ReflectMessageFns {
    /// Get the default set of [`ReflectMessageFns`] for a specific event type
    /// using its [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation
    /// before overriding some of the functions to create a custom implementation.
    pub fn new<M: Message + FromReflect + TypePath>() -> Self {
        <ReflectMessage as FromType<M>>::from_type().0
    }
}

impl ReflectMessage {
    /// Triggers a reflected [`Message`] like [`write_message()`](World::write_message).
    pub fn write_message(
        &self,
        world: &mut World,
        message: &dyn PartialReflect,
        registry: &TypeRegistry,
    ) {
        (self.0.write_message)(world, message, registry);
    }

    /// Create a custom implementation of [`ReflectMessage`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(Message)]`
    /// attribute to generate a [`ReflectMessage`] implementation automatically.
    ///
    /// See [`ReflectMessageFns`] for more information.
    pub fn new(fns: ReflectMessageFns) -> Self {
        ReflectMessage(fns)
    }

    /// The underlying function pointers implementing methods on [`ReflectMessage`].
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectMessage>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectMessage`] and keeping it
    /// between frames, cloning a `ReflectMessage` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectMessage`,
    /// use `fn_pointers` to get the underlying [`ReflectMessageFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectMessage>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
    pub fn fn_pointers(&self) -> &ReflectMessageFns {
        &self.0
    }
}

impl<M: Message + Reflect + TypePath> FromType<M> for ReflectMessage {
    fn from_type() -> Self {
        ReflectMessage(ReflectMessageFns {
            write_message: |world, reflected_message, registry| {
                let message = from_reflect_with_fallback::<M>(reflected_message, world, registry);
                world.write_message(message);
            },
        })
    }
}
