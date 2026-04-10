//! The [`ReflectConvert`] type, which allows types to register conversions to
//! and from one another.

use alloc::boxed::Box;
use core::any::TypeId;

use bevy_utils::TypeIdMap;

use crate::{Reflect, TypePath};

/// Provides a mechanism for converting values of one type to another.
///
/// This [`crate::type_registry::TypeData`] is associated with the type to be
/// converted *into*, not the type to be converted *from*. To convert a value,
/// use code like the following:
///
/// ```rust
/// # use bevy_reflect::{convert::ReflectConvert, GetTypeRegistration, TypeRegistry};
/// # use std::any::TypeId;
/// #
/// # let mut registry = TypeRegistry::default();
/// # registry.add_registration(i32::get_type_registration());
/// # registry.add_registration(String::get_type_registration());
/// # registry.register_type_conversion(|x: i32| Ok(x.to_string()));
///
/// let reflect_convert = registry
///     .get_type_data::<ReflectConvert>(TypeId::of::<String>())
///     .unwrap();
/// let converted: String = *reflect_convert
///     .try_convert_from(Box::new(12345i32))
///     .unwrap()
///     .downcast::<String>()
///     .unwrap();
/// ```
#[derive(Default)]
pub struct ReflectConvert {
    /// A mapping from the type to be converted *from* to its associated
    /// [`Converter`].
    conversions: TypeIdMap<Box<dyn Converter>>,
}

/// An internal trait that wraps a conversion function in an untyped interface.
trait Converter: Send + Sync {
    /// Converts the value to the appropriate type.
    ///
    /// This returns the converted value if the conversion succeeds or the
    /// original value if the conversion fails.
    fn convert(&self, input: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>>;

    /// Returns a new boxed instance wrapping the same [`Converter`].
    fn clone_converter(&self) -> Box<dyn Converter>;
}

/// A wrapper that contains a conversion function and implements [`Converter`].
struct TypedConverter<T, U>(fn(T) -> Result<U, T>)
where
    T: Reflect + TypePath,
    U: Reflect + TypePath;

impl ReflectConvert {
    /// Creates a new, empty [`ReflectConvert`] with no conversions registered.
    pub fn new() -> ReflectConvert {
        Self::default()
    }

    /// Attempts to construct an instance of this type from the provided
    /// `input`.
    ///
    /// If the conversion fails, either because no conversion has been
    /// registered from the type of `input` or because the conversion function
    /// itself returned `Err`, the `input` value is returned as an error.
    pub fn try_convert_from(
        &self,
        input: Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        let type_id = (*input.as_any()).type_id();
        match self.conversions.get(&type_id) {
            Some(converter) => converter.convert(input),
            None => Err(input),
        }
    }

    /// Adds a conversion function from the type `T` to this type.
    ///
    /// If the conversion succeeds, the function should return the converted
    /// value. If the conversion fails, the function should return the original
    /// input value.
    pub fn register_type_conversion<T, U>(&mut self, function: fn(T) -> Result<U, T>)
    where
        T: Reflect + TypePath,
        U: Reflect + TypePath,
    {
        self.conversions
            .insert(TypeId::of::<T>(), Box::new(TypedConverter(function)));
    }
}

impl Clone for ReflectConvert {
    fn clone(&self) -> Self {
        ReflectConvert {
            conversions: self
                .conversions
                .iter()
                .map(|(type_id, converter)| (*type_id, converter.clone_converter()))
                .collect(),
        }
    }
}

impl<T, U> Clone for TypedConverter<T, U>
where
    T: Reflect + TypePath,
    U: Reflect + TypePath,
{
    fn clone(&self) -> Self {
        TypedConverter(self.0)
    }
}

impl<T, U> Converter for TypedConverter<T, U>
where
    T: Reflect + TypePath,
    U: Reflect + TypePath,
{
    fn convert(&self, input: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        match (self.0)(input.take()?) {
            Ok(value) => Ok(Box::new(value)),
            Err(value) => Err(Box::new(value)),
        }
    }

    fn clone_converter(&self) -> Box<dyn Converter> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        borrow::ToOwned as _,
        boxed::Box,
        string::{String, ToString},
    };
    use core::any::TypeId;

    use crate::{convert::ReflectConvert, type_registry::GetTypeRegistration, TypeRegistry};

    /// Tests that `i32` can be converted to `String` if the appropriate
    /// conversion is registered.
    #[test]
    fn convert_from_i32_to_string() {
        // Register the types and the conversion.
        let mut registry = TypeRegistry::default();
        registry.add_registration(i32::get_type_registration());
        registry.add_registration(String::get_type_registration());
        registry.register_type_conversion(|x: i32| Ok(x.to_string()));

        let reflect_convert = registry
            .get_type_data::<ReflectConvert>(TypeId::of::<String>())
            .unwrap();

        // Test that a successful conversion works.
        let converted = reflect_convert
            .try_convert_from(Box::new(12345i32))
            .unwrap()
            .downcast::<String>()
            .unwrap();
        assert_eq!(&**converted, "12345");
    }

    /// Tests that `String` can be fallibly converted to `i32` if the
    /// appropriate conversion is registered.
    ///
    /// This also tests that the behavior of returning the original string on
    /// error is correct.
    #[test]
    fn convert_from_string_to_i32() {
        // Register the types and the conversion.
        let mut registry = TypeRegistry::default();
        registry.add_registration(i32::get_type_registration());
        registry.add_registration(String::get_type_registration());
        registry.register_type_conversion(|x: String| match x.parse::<i32>() {
            Ok(value) => Ok(value),
            Err(_) => Err(x),
        });

        let reflect_convert = registry
            .get_type_data::<ReflectConvert>(TypeId::of::<i32>())
            .unwrap();

        // Test a successful conversion from string to integer.
        let converted = reflect_convert
            .try_convert_from(Box::new("12345".to_owned()))
            .unwrap()
            .downcast::<i32>()
            .unwrap();
        assert_eq!(*converted, 12345);

        // Test an unsuccessful conversion from string to integer.
        let error = reflect_convert
            .try_convert_from(Box::new("qqqqq".to_owned()))
            .unwrap_err()
            .downcast::<String>()
            .unwrap();
        assert_eq!(&**error, "qqqqq");
    }

    /// Tests that the error-handling behavior is correct when attempting a
    /// conversion that hasn't been registered.
    #[test]
    fn no_such_conversion() {
        let mut registry = TypeRegistry::default();
        registry.add_registration(i32::get_type_registration());
        registry.add_registration(String::get_type_registration());

        let reflect_convert = registry
            .get_type_data::<ReflectConvert>(TypeId::of::<i32>())
            .unwrap();

        // Test that we get the original value back on error.
        let error = reflect_convert
            .try_convert_from(Box::new("12345".to_owned()))
            .unwrap_err()
            .downcast::<String>()
            .unwrap();
        assert_eq!(&**error, "12345");
    }
}
