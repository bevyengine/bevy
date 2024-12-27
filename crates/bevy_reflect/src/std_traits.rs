use crate::{FromType, Reflect};
use alloc::boxed::Box;

/// A struct used to provide the default value of a type.
///
/// A [`ReflectDefault`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectDefault {
    default: fn() -> Box<dyn Reflect + Send + Sync>,
}

impl ReflectDefault {
    pub fn default(&self) -> Box<dyn Reflect + Send + Sync> {
        (self.default)()
    }
}

impl<T: Reflect + Send + Sync + Default> FromType<T> for ReflectDefault {
    fn from_type() -> Self {
        ReflectDefault {
            default: || Box::<T>::default(),
        }
    }
}
