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
            self.resize_with(len, T::default);

            if properties.property_type() != self.property_type() {
                panic!(
                    "Properties type mismatch. This type is {:?} but the applied type is {:?}",
                    self.property_type(),
                    properties.property_type()
                );
            }
            for (i, prop) in properties.iter_props().enumerate() {
                if let Some(p) = self.prop_with_index_mut(i) {
                    p.apply(prop)
                }
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

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = prop.clone();
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for usize {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<u64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u8>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<isize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i8>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for u64 {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<usize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u8>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<isize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i8>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for u32 {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<u64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<usize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u8>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<isize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i8>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for u16 {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<u64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<usize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u8>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<isize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i8>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for u8 {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<u64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<usize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<isize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i8>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for isize {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<i64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i8>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<usize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u8>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for i64 {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<isize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i8>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<usize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u8>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for i32 {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<i64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<isize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i8>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<usize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u8>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for i16 {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<i64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<isize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i8>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<usize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u8>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for i8 {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<i64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<i16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<isize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<usize>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u64>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u32>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u16>() {
            *self = *prop as Self;
        } else if let Some(prop) = value.downcast_ref::<u8>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for f32 {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<f64>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}

impl Property for f64 {
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
        Box::new(*self)
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<f32>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }

    fn serializable<'a>(&'a self, _registry: &'a PropertyTypeRegistry) -> Serializable<'a> {
        Serializable::Borrowed(self)
    }
}
