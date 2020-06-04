use std::any::{type_name, Any};

pub trait DowncastTypename {
    fn downcast_typename_mut<T: Any>(&mut self) -> Option<&mut T>;
    fn downcast_typename_ref<T: Any>(&self) -> Option<&T>;
    fn is_typename<T: Any>(&self) -> bool;
}

pub fn type_name_of_val<T: ?Sized>(_val: &T) -> &'static str { type_name::<T>() }
