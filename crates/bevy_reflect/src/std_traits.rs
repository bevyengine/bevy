use crate::{Reflect, TypeData};

/// A struct used to provide the default value of a type.
///
/// A [`ReflectDefault`] for type `T` can be obtained via [`TypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectDefault {
    default: fn() -> Box<dyn Reflect>,
}

impl ReflectDefault {
    pub fn default(&self) -> Box<dyn Reflect> {
        (self.default)()
    }
}

impl<T: Reflect + Default> TypeData<T> for ReflectDefault {
    fn create_type_data() -> Self {
        ReflectDefault {
            default: || Box::<T>::default(),
        }
    }
}
