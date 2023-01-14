use crate::{FromType, PartialReflect};

/// A struct used to provide the default value of a type.
///
/// A [`ReflectDefault`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectDefault {
    default: fn() -> Box<dyn PartialReflect>,
}

impl ReflectDefault {
    pub fn default(&self) -> Box<dyn PartialReflect> {
        (self.default)()
    }
}

impl<T: PartialReflect + Default> FromType<T> for ReflectDefault {
    fn from_type() -> Self {
        ReflectDefault {
            default: || Box::<T>::default(),
        }
    }
}
