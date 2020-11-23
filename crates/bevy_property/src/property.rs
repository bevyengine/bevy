use crate::{property_serde::Serializable, Properties, PropertyTypeRegistry};
use erased_serde::Deserializer;
use std::any::Any;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum PropertyType {
    Map,
    Seq,
    Value,
}

// TODO: consider removing send + sync requirements
pub trait Property: Send + Sync + Any + 'static {
    fn type_name(&self) -> &str;
    fn any(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
    fn clone_prop(&self) -> Box<dyn Property>;
    fn set(&mut self, value: &dyn Property);
    fn apply(&mut self, value: &dyn Property);
    fn property_type(&self) -> PropertyType {
        PropertyType::Value
    }
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
    fn serializable<'a>(&'a self, registry: &'a PropertyTypeRegistry) -> Serializable<'a>;
}

pub trait DeserializeProperty {
    fn deserialize(
        deserializer: &mut dyn Deserializer,
        property_type_registry: &PropertyTypeRegistry,
    ) -> Result<Box<dyn Property>, erased_serde::Error>;
}

pub trait PropertyVal {
    fn val<T: 'static>(&self) -> Option<&T>;
    fn set_val<T: 'static>(&mut self, value: T);
}

impl PropertyVal for dyn Property {
    #[inline]
    fn val<T: 'static>(&self) -> Option<&T> {
        self.any().downcast_ref::<T>()
    }

    #[inline]
    fn set_val<T: 'static>(&mut self, value: T) {
        if let Some(prop) = self.any_mut().downcast_mut::<T>() {
            *prop = value;
        } else {
            panic!("prop value is not {}", std::any::type_name::<T>());
        }
    }
}
