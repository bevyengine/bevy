use std::ops::{Deref, DerefMut};

use bevy_reflect::{TypeRegistry, TypeRegistryArc};
use parking_lot::RwLockReadGuard;

use crate as bevy_ecs;
use crate::system::Resource;

/// A [`Resource`] storing [`TypeRegistry`](bevy_reflect::TypeRegistry) for
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

/// A type that can access a [`TypeRegistry`].
///
/// This is used with the [`ReflectCommandExt`] methods to identify and extract
/// a type registry to use to insert/remove [`Reflect`] components.
///
/// It is implemented for [`AppTypeRegistry`] and any `Resource` that implements
/// `AsRef<TypeRegistry>`.
///
/// [`ReflectCommandExt`]: super::ReflectCommandExt
/// [`Reflect`]: bevy_reflect::Reflect
pub trait ReadTypeRegistry {
    /// The type holding the [`TypeRegistry`] to read from.
    type Target<'t>: Deref<Target = TypeRegistry>
    where
        Self: 't;

    /// Read the [`TypeRegistry`] from `Self`.
    fn type_registry(&self) -> Self::Target<'_>;
}
impl<T: AsRef<TypeRegistry>> ReadTypeRegistry for T {
    type Target<'t> = &'t TypeRegistry where Self: 't;
    fn type_registry(&self) -> Self::Target<'_> {
        self.as_ref()
    }
}
impl ReadTypeRegistry for AppTypeRegistry {
    type Target<'t> = RwLockReadGuard<'t, TypeRegistry> where Self: 't;
    fn type_registry(&self) -> Self::Target<'_> {
        self.read()
    }
}
