//! Definitions for [`Resource`] reflection.
//!
//! # Architecture
//!
//! See the module doc for [`reflect::component`](`crate::reflect::component`).

use crate::{reflect::ReflectComponent, resource::Resource};
use bevy_reflect::{FromReflect, FromType, TypePath, TypeRegistration};

/// A struct that marks a reflected [`Resource`] of a type.
///
/// This is struct does not provide any functionality.
/// It implies the existence of a reflected [`Component`](crate::component::Component) of the same type,
/// which is meant to be used instead.
///
/// ```rust,ignore
/// #[derive(Resource, Reflect)]
/// #[reflect(Resource)]
/// struct ResA;
///
/// // is the same as:
///
/// #[derive(Resource, Component, Reflect)]
/// #[reflect(Resource, Component)]
/// struct ResA;
/// ```
///
/// A [`ReflectResource`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectResource;

impl<R: Resource + FromReflect + TypePath> FromType<R> for ReflectResource {
    fn from_type() -> Self {
        ReflectResource
    }

    fn insert_dependencies(type_registration: &mut TypeRegistration) {
        type_registration.register_type_data::<ReflectComponent, R>();
    }
}
