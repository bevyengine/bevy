use crate::Properties;
use std::any::Any;

pub trait Property: erased_serde::Serialize + Send + Sync + Any + AsProperties + 'static {
    fn any(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
    fn clone_prop(&self) -> Box<dyn Property>;
    fn set(&mut self, value: &dyn Property);
    fn apply(&mut self, value: &dyn Property);
}

erased_serde::serialize_trait_object!(Property);

pub trait AsProperties {
    fn as_properties(&self) -> Option<&dyn Properties>;
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

impl AsProperties for usize
{
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

impl AsProperties for u64
{
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

impl AsProperties for u32
{
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

impl AsProperties for u16
{
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

impl AsProperties for u8
{
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

impl AsProperties for isize
{
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

impl AsProperties for i64
{
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

impl AsProperties for i32
{
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

impl AsProperties for i16
{
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

impl AsProperties for i8
{
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

impl AsProperties for f32
{
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

impl AsProperties for f64
{
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

impl Property for String {
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
}

impl AsProperties for String
{
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}