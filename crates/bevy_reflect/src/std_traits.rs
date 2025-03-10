use crate::{CreateTypeData, Reflect};
use alloc::boxed::Box;

/// A struct used to provide the default value of a type.
///
/// A [`ReflectDefault`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectDefault {
    default: fn() -> Box<dyn Reflect>,
}

impl ReflectDefault {
    pub fn default(&self) -> Box<dyn Reflect> {
        (self.default)()
    }
}

impl<T: Reflect + Default> CreateTypeData<T> for ReflectDefault {
    fn create_type_data(_input: ()) -> Self {
        ReflectDefault {
            default: || Box::<T>::default(),
        }
    }
}
