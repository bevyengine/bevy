use crate::{FromType, PartialReflect, Reflect};
use alloc::boxed::Box;

/// A struct used to provide the default value of a type.
///
/// A [`ReflectDefault`] for type `T` can be obtained via [`FromType::from_type`].
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
            default: || Box::<T>::default(),
        }
    }
}

/// Type data for the [`Clone`] trait.
///
/// This type data can be used to attempt to clone a [`PartialReflect`] value
/// using the concrete type's [`Clone`] implementation.
#[derive(Clone)]
pub struct ReflectClone {
    try_clone: fn(&dyn PartialReflect) -> Option<Box<dyn Reflect>>,
}

impl ReflectClone {
    /// Clones a [`PartialReflect`] value using the concrete type's [`Clone`] implementation.
    ///
    /// # Panics
    ///
    /// This function will panic if the provided value is not the same type as the type this [`ReflectClone`] was created for.
    ///
    /// For a non-panicking version, see [`ReflectClone::try_clone`].
    pub fn clone(&self, value: &dyn PartialReflect) -> Box<dyn Reflect> {
        self.try_clone(value).unwrap()
    }

    /// Attempts to clone a [`PartialReflect`] value using the concrete type's [`Clone`] implementation.
    ///
    /// If the provided value is not the same type as the type this [`ReflectClone`] was created for,
    /// this function will return `None`.
    ///
    /// For a panicking version, see [`ReflectClone::clone`].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::{Reflect, std_traits::ReflectClone, FromType, PartialReflect};
    /// # #[derive(Clone, Reflect, Debug, PartialEq)]
    /// # #[reflect(Clone)]
    /// # struct AnotherStruct(i32);
    /// #[derive(Clone, Reflect, Debug, PartialEq)]
    /// #[reflect(Clone)]
    /// struct MyStruct(i32);
    ///
    /// let reflect_clone = <ReflectClone as FromType<MyStruct>>::from_type();
    /// let value: Box<dyn PartialReflect> = Box::new(MyStruct(123));
    ///
    /// let cloned_value = reflect_clone.try_clone(&*value);
    /// assert!(cloned_value.is_some());
    /// assert_eq!(MyStruct(123), cloned_value.unwrap().take::<MyStruct>().unwrap());
    ///
    /// // Attempting to clone a value of a different type will return None
    /// let another_value: Box<dyn PartialReflect> = Box::new(AnotherStruct(123));
    /// assert!(reflect_clone.try_clone(&*another_value).is_none());
    /// ```
    pub fn try_clone(&self, value: &dyn PartialReflect) -> Option<Box<dyn Reflect>> {
        (self.try_clone)(value)
    }
}

impl<T: Reflect + Clone> FromType<T> for ReflectClone {
    fn from_type() -> Self {
        ReflectClone {
            try_clone: |value| Some(Box::new(value.try_downcast_ref::<T>()?.clone())),
        }
    }
}
