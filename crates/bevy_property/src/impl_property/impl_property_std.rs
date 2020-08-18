use crate::{
    impl_property,
    property_serde::{SeqSerializer, Serializable},
    Properties, Property, PropertyIter, PropertyType, PropertyTypeRegistry,
};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{BTreeMap, HashMap, HashSet},
    hash::Hash,
    ops::Range,
};

impl<T> Properties for Vec<T>
where
    T: Property + Clone + Default,
{
    fn prop(&self, _name: &str) -> Option<&dyn Property> {
        None
    }

    fn prop_mut(&mut self, _name: &str) -> Option<&mut dyn Property> {
        None
    }

    fn prop_with_index(&self, index: usize) -> Option<&dyn Property> {
        Some(&self[index])
    }

    fn prop_with_index_mut(&mut self, index: usize) -> Option<&mut dyn Property> {
        Some(&mut self[index])
    }

    fn prop_name(&self, _index: usize) -> Option<&str> {
        None
    }

    fn prop_len(&self) -> usize {
        self.len()
    }

    fn iter_props(&self) -> PropertyIter {
        PropertyIter::new(self)
    }
}

impl<T> Property for Vec<T>
where
    T: Property + Clone + Default,
{
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn any(&self) -> &dyn Any {
        self
    }

    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clone_prop(&self) -> Box<dyn Property> {
        Box::new(self.clone())
    }

    fn set(&mut self, value: &dyn Property) {
        if let Some(properties) = value.as_properties() {
            let len = properties.prop_len();
            self.resize_with(len, || T::default());

            if properties.property_type() != self.property_type() {
                panic!(
                    "Properties type mismatch. This type is {:?} but the applied type is {:?}",
                    self.property_type(),
                    properties.property_type()
                );
            }
            for (i, prop) in properties.iter_props().enumerate() {
                self.prop_with_index_mut(i).map(|p| p.apply(prop));
            }
        } else {
            panic!("attempted to apply non-Properties type to Properties type");
        }
    }

    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn as_properties(&self) -> Option<&dyn Properties> {
        Some(self)
    }

    fn serializable<'a>(&'a self, registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Owned(Box::new(SeqSerializer::new(self, registry)))
    }

    fn property_type(&self) -> PropertyType {
        PropertyType::Seq
    }
}

// impl_property!(SEQUENCE, VecDeque<T> where T: Clone + Send + Sync + Serialize + 'static);
impl_property!(Option<T> where T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static);
impl_property!(HashSet<T> where T: Clone + Eq + Send + Sync + Hash + Serialize + for<'de> Deserialize<'de> + 'static);
impl_property!(HashMap<K, V> where
    K: Clone + Eq + Send + Sync + Hash + Serialize + for<'de> Deserialize<'de> + 'static,
    V: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,);
impl_property!(BTreeMap<K, V> where
    K: Clone + Ord + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
    V: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static);
impl_property!(Range<T> where T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static);

// TODO: Implement lossless primitive types in RON and remove all of these primitive "cast checks"
impl Property for String {
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

    fn set(&mut self, property: &dyn Property) {
        let value = property.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = prop.clone();
        } else {
            panic!(
                "prop value is not {}, but {}",
                std::any::type_name::<Self>(),
                property.type_name()
            );
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for bool {
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

    fn set(&mut self, property: &dyn Property) {
        let value = property.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else {
            panic!(
                "prop value is not {}, but {}",
                std::any::type_name::<Self>(),
                property.type_name()
            );
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

macro_rules! set_integer {
    ($this:expr, $value:expr, $else_body:expr) => {{
        if let Some(prop) = ($value).downcast_ref::<usize>() {
            *($this) = *prop as Self;
        } else if let Some(prop) = ($value).downcast_ref::<u64>() {
            *($this) = *prop as Self;
        } else if let Some(prop) = ($value).downcast_ref::<u32>() {
            *($this) = *prop as Self;
        } else if let Some(prop) = ($value).downcast_ref::<u16>() {
            *($this) = *prop as Self;
        } else if let Some(prop) = ($value).downcast_ref::<u8>() {
            *($this) = *prop as Self;
        } else if let Some(prop) = ($value).downcast_ref::<isize>() {
            *($this) = *prop as Self;
        } else if let Some(prop) = ($value).downcast_ref::<i64>() {
            *($this) = *prop as Self;
        } else if let Some(prop) = ($value).downcast_ref::<i32>() {
            *($this) = *prop as Self;
        } else if let Some(prop) = ($value).downcast_ref::<i16>() {
            *($this) = *prop as Self;
        } else if let Some(prop) = ($value).downcast_ref::<i8>() {
            *($this) = *prop as Self;
        } else {
            $else_body
        }
    }};
}

macro_rules! integer_property {
    ($integer_type:ty) => {
        impl Property for $integer_type {
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

            fn set(&mut self, property: &dyn Property) {
                let value = property.any();
                set_integer!(
                    self,
                    value,
                    panic!(
                        "prop value is not {}, but {}",
                        std::any::type_name::<Self>(),
                        property.type_name()
                    )
                );
            }

            fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
                Serializable::Borrowed(self)
            }
        }
    };
}

macro_rules! float_property {
    ($float_type:ty) => {
        impl Property for $float_type {
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

            fn set(&mut self, property: &dyn Property) {
                let value = property.any();
                if let Some(prop) = value.downcast_ref::<Self>() {
                    *self = *prop as Self;
                } else if let Some(prop) = value.downcast_ref::<f64>() {
                    *self = *prop as Self;
                } else {
                    panic!(
                        "prop value is not {}, but {}",
                        std::any::type_name::<Self>(),
                        property.type_name()
                    );
                }
            }

            fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
                Serializable::Borrowed(self)
            }
        }
    };
}

integer_property!(usize);
integer_property!(isize);
integer_property!(u8);
integer_property!(u16);
integer_property!(u32);
integer_property!(u64);
integer_property!(i8);
integer_property!(i16);
integer_property!(i32);
integer_property!(i64);

float_property!(f32);
float_property!(f64);
