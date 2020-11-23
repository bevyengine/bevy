use crate::{property_serde::Serializable, Property, PropertyType, PropertyTypeRegistry};
use serde::Serialize;
use smallvec::{Array, SmallVec};
use std::any::Any;

impl<T, I> Property for SmallVec<T>
where
    T: Clone + Send + Sync + Serialize + 'static + Array<Item = I>,
    I: Send + Sync + Clone + Serialize + 'static,
{
    #[inline]
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    #[inline]
    fn any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn clone_prop(&self) -> Box<dyn Property> {
        Box::new(self.clone())
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = prop.clone();
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }

    fn property_type(&self) -> PropertyType {
        PropertyType::Value
    }
}
