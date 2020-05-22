use serde::Serialize;
use std::any::Any;

pub trait Property: erased_serde::Serialize + Send + Sync + Any + 'static {
    fn any(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
    fn clone_prop(&self) -> Box<dyn Property>;
    fn set(&mut self, value: &dyn Property);
}

erased_serde::serialize_trait_object!(Property);

pub trait PropertyVal {
    fn val<T: 'static>(&self) -> Option<&T>;
    fn set_val<T: 'static>(&mut self, value: T);
}

impl PropertyVal for dyn Property {
    // #[inline]
    default fn val<T: 'static>(&self) -> Option<&T> {
        self.any().downcast_ref::<T>()
    }
    
    // #[inline]
    default fn set_val<T: 'static>(&mut self, value: T) {
        if let Some(prop) = self.any_mut().downcast_mut::<T>() {
            *prop = value;
        } else {
            panic!("prop value is not {}", std::any::type_name::<T>());
        }
    }
}

impl<T> Property for T
where
    T: Clone + Serialize + Send + Sync + Any + 'static,
{
    #[inline]
    default fn any(&self) -> &dyn Any {
        self
    }
    #[inline]
    default fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
    #[inline]
    default fn clone_prop(&self) -> Box<dyn Property> {
        Box::new(self.clone())
    }
    #[inline]
    default fn set(&mut self, value: &dyn Property) {
        if let Some(prop) = value.any().downcast_ref::<T>() {
            *self = prop.clone();
        } else {
            panic!("prop value is not {}", std::any::type_name::<T>());
        }
    }
}

impl Property for usize {
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

impl Property for u64 {
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

impl Property for u32 {
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

impl Property for u16 {
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

impl Property for u8 {
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

impl Property for isize {
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

impl Property for i64 {
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

impl Property for i32 {
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

impl Property for i16 {
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

impl Property for i8 {
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


impl Property for f32 {
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

impl Property for f64 {
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