//! Module containing the [`ReflectDefault`] type.

use crate::{FromType, Reflect};
use alloc::boxed::Box;

/// A struct used to provide the default value of a type.
///
/// A [`ReflectDefault`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectDefault {
    default: fn() -> Box<dyn Reflect>,
}

impl ReflectDefault {
    /// Returns the default value for a type.
    pub fn default(&self) -> Box<dyn Reflect> {
        (self.default)()
    }
}

impl<T: Reflect + Default> FromType<T> for ReflectDefault {
    fn from_type() -> Self {
        ReflectDefault {
            default: || Box::<T>::default(),
        }
    }
}
