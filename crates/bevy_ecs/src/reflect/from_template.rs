//! Definitions for [`FromTemplate`] reflection.
//! This allows fetching the Registration for the associated [`Template`].
//!
//! This module exports two types: [`ReflectFromTemplateFns`] and [`ReflectFromTemplate`].
//!
//! Same as [`component`](`super::component`), but for [`FromTemplate`].

use crate::template::FromTemplate;
use bevy_reflect::{CreateTypeData, Reflect, TypeRegistration, TypeRegistry};
use core::any::TypeId;

/// A struct used to operate on the reflected [`FromTemplate`] trait of a type.
///
/// A [`ReflectFromTemplate`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectFromTemplate(ReflectFromTemplateFns);

/// The raw function pointers needed to make up a [`ReflectFromTemplate`].
#[derive(Clone)]
pub struct ReflectFromTemplateFns {
    /// Function pointer implementing [`ReflectFromTemplate::get_template`].
    pub get_template: fn(&TypeRegistry) -> Option<&TypeRegistration>,
}

impl ReflectFromTemplateFns {
    /// Get the default set of [`ReflectFromTemplateFns`] for a specific type using its
    /// [`CreateTypeData`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: Reflect + FromTemplate>() -> Self {
        <ReflectFromTemplate as CreateTypeData<T>>::create_type_data(()).0
    }
}

impl ReflectFromTemplate {
    /// fetches the Registration for the associated [`Template`]
    pub fn get_template<'a>(&self, registry: &'a TypeRegistry) -> Option<&'a TypeRegistration> {
        (self.0.get_template)(registry)
    }

    /// Create a custom implementation of [`ReflectFromTemplate`].
    ///
    /// This is an advanced feature,
    /// useful for scripting implementations,
    /// that should not be used by most users
    /// unless you know what you are doing.
    ///
    /// Usually you should derive [`Reflect`] and add the `#[reflect(FromTemplate)]` bundle
    /// to generate a [`ReflectFromTemplate`] implementation automatically.
    ///
    /// See [`ReflectFromTemplateFns`] for more information.
    pub fn new(fns: ReflectFromTemplateFns) -> Self {
        Self(fns)
    }

    /// The underlying function pointers implementing methods on `ReflectFromTemplate`.
    ///
    /// This is useful when you want to keep track locally of an individual
    /// function pointer.
    ///
    /// Calling [`TypeRegistry::get`] followed by
    /// [`TypeRegistration::data::<ReflectFromTemplate>`] can be costly if done several
    /// times per frame. Consider cloning [`ReflectFromTemplate`] and keeping it
    /// between frames, cloning a `ReflectFromTemplate` is very cheap.
    ///
    /// If you only need a subset of the methods on `ReflectFromTemplate`,
    /// use `fn_pointers` to get the underlying [`ReflectFromTemplateFns`]
    /// and copy the subset of function pointers you care about.
    ///
    /// [`TypeRegistration::data::<ReflectFromTemplate>`]: bevy_reflect::TypeRegistration::data
    /// [`TypeRegistry::get`]: bevy_reflect::TypeRegistry::get
    pub fn fn_pointers(&self) -> &ReflectFromTemplateFns {
        &self.0
    }
}

impl<T: Reflect + FromTemplate> CreateTypeData<T> for ReflectFromTemplate {
    fn create_type_data(_input: ()) -> Self {
        ReflectFromTemplate(ReflectFromTemplateFns {
            get_template: |registry: &TypeRegistry| {
                let registration = registry.get(TypeId::of::<T::Template>());

                registration
            },
        })
    }
}
