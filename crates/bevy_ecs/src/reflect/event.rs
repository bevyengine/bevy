//! Definitions for [`Event`] reflection.
//! This allows triggering events whose type is only known at runtime.
//!
//! This module exports two types: [`ReflectEventFns`] and [`ReflectEvent`].
//!
//! Same as [`component`](`super::component`), but for events.

use crate::{event::Event, reflect::from_reflect_with_fallback, world::World};
use bevy_reflect::{FromReflect, FromType, PartialReflect, Reflect, TypePath, TypeRegistry};

/// A struct used to operate on reflected [`Event`] trait of a type.
///
/// A [`ReflectEvent`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectEvent(ReflectEventFns);

/// The raw function pointers needed to make up a [`ReflectEvent`].
///
/// This is used when creating custom implementations of [`ReflectEvent`] with
/// [`ReflectEvent::new()`].
///
/// > **Note:**
/// > Creating custom implementations of [`ReflectEvent`] is an advanced feature
/// > that most users will not need. Usually a [`ReflectEvent`] is created for a
/// > type by deriving [`Reflect`] and adding the `#[reflect(Event)]` attribute.
/// > After adding the event to the [`TypeRegistry`], its [`ReflectEvent`] can
/// > then be retrieved when needed.
///
/// Creating a custom [`ReflectEvent`] may be useful if you need to create new
/// event types at runtime, for example, for scripting implementations.
///
/// By creating a custom [`ReflectEvent`] and inserting it into a type's
/// [`TypeRegistration`][bevy_reflect::TypeRegistration], you can modify the way
/// that reflected event of that type will be triggered in the Bevy world.
#[derive(Clone)]
pub struct ReflectEventFns {
    /// Function pointer implementing [`ReflectEvent::trigger`].
    pub trigger: fn(&mut World, &dyn PartialReflect, &TypeRegistry),
}

impl ReflectEventFns {
    /// Get the default set of [`ReflectEventFns`] for a specific event type
    /// using its [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation
    /// before overriding some of the functions to create a custom implementation.
    pub fn new<'a, T: Event + FromReflect + TypePath>() -> Self
    where
        T::Trigger<'a>: Default,
    {
        <ReflectEvent as FromType<T>>::from_type().0
    }
}

impl ReflectEvent {
    /// Triggers a reflected [`Event`] like [`trigger()`](World::trigger).
    pub fn trigger(&self, world: &mut World, event: &dyn PartialReflect, registry: &TypeRegistry) {
        (self.0.trigger)(world, event, registry);
    }

    /// Create a custom implementation of [`ReflectEvent`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(Event)]`
    /// attribute to generate a [`ReflectEvent`] implementation automatically.
    ///
    /// See [`ReflectEventFns`] for more information.
    pub fn new(fns: ReflectEventFns) -> Self {
        ReflectEvent(fns)
    }

    /// The underlying function pointers implementing methods on [`ReflectEvent`].
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectEvent>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectEvent`] and keeping it
    /// between frames, cloning a `ReflectEvent` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectEvent`,
    /// use `fn_pointers` to get the underlying [`ReflectEventFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectEvent>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
    pub fn fn_pointers(&self) -> &ReflectEventFns {
        &self.0
    }
}

impl<'a, E: Event + Reflect + TypePath> FromType<E> for ReflectEvent
where
    <E as Event>::Trigger<'a>: Default,
{
    fn from_type() -> Self {
        ReflectEvent(ReflectEventFns {
            trigger: |world, reflected_event, registry| {
                let event = from_reflect_with_fallback::<E>(reflected_event, world, registry);
                world.trigger(event);
            },
        })
    }
}
