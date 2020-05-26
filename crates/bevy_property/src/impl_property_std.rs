use crate::{AsProperties, Properties, Property, impl_property};
use serde::Serialize;
use std::{
    any::Any,
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    hash::Hash,
};

impl Property for usize {
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
}

impl AsProperties for usize {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for u64 {
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
}

impl AsProperties for u64 {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for u32 {
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
}

impl AsProperties for u32 {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for u16 {
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
}

impl AsProperties for u16 {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for u8 {
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
}

impl AsProperties for u8 {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for isize {
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
}

impl AsProperties for isize {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for i64 {
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
}

impl AsProperties for i64 {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for i32 {
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
}

impl AsProperties for i32 {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for i16 {
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
}

impl AsProperties for i16 {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for i8 {
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
}

impl AsProperties for i8 {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for f32 {
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
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<f64>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }
}

impl AsProperties for f32 {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for f64 {
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
            *self = *prop;
        } else if let Some(prop) = value.downcast_ref::<f32>() {
            *self = *prop as Self;
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }
}

impl AsProperties for f64 {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl_property!(String);
impl_property!(bool);
impl_property!(Vec<T> where T: Clone + Send + Sync + Serialize + 'static);
impl_property!(VecDeque<T> where T: Clone + Send + Sync + Serialize + 'static);
impl_property!(HashSet<T> where T: Clone + Eq + Send + Sync + Hash + Serialize + 'static);
impl_property!(HashMap<K, V> where
    K: Clone + Eq + Send + Sync + Hash + Serialize + 'static,
    V: Clone + Send + Sync + Serialize + 'static,);
impl_property!(BTreeMap<K, V> where
    K: Clone + Ord + Send + Sync + Serialize + 'static,
    V: Clone + Send + Sync + Serialize + 'static);

impl<T> AsProperties for  Vec<T> {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl<T> AsProperties for VecDeque<T> {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl<K> AsProperties for HashSet<K> {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl<K, V> AsProperties for HashMap<K, V> {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl<K, V> AsProperties for BTreeMap<K, V> {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}
