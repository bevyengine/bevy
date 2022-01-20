use crate::{FromType, Reflect};

#[derive(Clone)]
pub struct ReflectDefault {
    default: fn() -> Box<dyn Reflect>,
}

impl ReflectDefault {
    pub fn default(&self) -> Box<dyn Reflect> {
        (self.default)()
    }
}

impl<T: Reflect + Default> FromType<T> for ReflectDefault {
    fn from_type() -> Self {
        ReflectDefault {
            default: || Box::new(T::default()),
        }
    }
}
