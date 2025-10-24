//! Definitions for [`Resource`] reflection.
//!
//! # Architecture
//!
//! See the module doc for [`reflect::component`](`crate::reflect::component`).

use crate::{
    reflect::{ReflectComponent, ReflectComponentFns},
    resource::Resource,
};
use bevy_reflect::{FromReflect, FromType, TypePath};

/// A struct used to operate on reflected [`Resource`] of a type.
///
/// A [`ReflectResource`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectResource(ReflectComponentFns);

impl ReflectResource {
    /// Use as a [`ReflectComponent`].
    pub fn as_reflect_component(self) -> ReflectComponent {
        ReflectComponent::new(self.0)
    }
}

impl<R: Resource + FromReflect + TypePath> FromType<R> for ReflectResource {
    fn from_type() -> Self {
        ReflectResource(ReflectComponentFns::new::<R>())
    }
}
