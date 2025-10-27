//! Definitions for [`Resource`] reflection.
//!
//! # Architecture
//!
//! See the module doc for [`reflect::component`](`crate::reflect::component`).

use core::ops::Deref;

use crate::{reflect::ReflectComponent, resource::Resource};
use bevy_reflect::{FromReflect, FromType, TypePath};

/// A struct used to operate on reflected [`Resource`] of a type.
///
/// A [`ReflectResource`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectResource(ReflectComponent);

impl Deref for ReflectResource {
    type Target = ReflectComponent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<R: Resource + FromReflect + TypePath> FromType<R> for ReflectResource {
    fn from_type() -> Self {
        ReflectResource(<ReflectComponent as FromType<R>>::from_type())
    }
}
