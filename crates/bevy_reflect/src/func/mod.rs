//! Reflection-based dynamic functions.
//!
//! This module provides a way to pass around and call functions dynamically
//! using the [`DynamicFunction`] type.
//!
//! Many simple functions and closures can be automatically converted to [`DynamicFunction`]
//! using the [`IntoFunction`] trait.
//!
//! Once the [`DynamicFunction`] is created, it can be called with a set of arguments provided
//! via an [`ArgList`].
//!
//! This returns a [`FunctionResult`] containing the [`Return`] value,
//! which can be used to extract a [`Reflect`] trait object.
//!
//!
//! # Example
//!
//! ```
//! # use bevy_reflect::Reflect;
//! # use bevy_reflect::func::args::ArgList;
//! # use bevy_reflect::func::{DynamicFunction, FunctionResult, IntoFunction, Return};
//! fn add(a: i32, b: i32) -> i32 {
//!   a + b
//! }
//!
//! let mut func: DynamicFunction = add.into_function();
//! let args: ArgList = ArgList::default()
//!   // Pushing a known type with owned ownership
//!   .push_owned(25_i32)
//!   // Pushing a reflected type with owned ownership
//!   .push_boxed(Box::new(75_i32) as Box<dyn Reflect>);
//! let result: FunctionResult = func.call(args);
//! let value: Return = result.unwrap();
//! assert_eq!(value.unwrap_owned().downcast_ref::<i32>(), Some(&100));
//! ```
//!
//! [`Reflect`]: crate::Reflect

pub use error::*;
pub use function::*;
pub use info::*;
pub use into_function::*;
pub use return_type::*;

pub use args::{Arg, ArgError, ArgList};

pub mod args;
mod error;
mod function;
mod info;
mod into_function;
pub(crate) mod macros;
mod return_type;

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::func::args::{ArgError, ArgId, ArgList, Ownership};
    use crate::{Reflect, TypePath};
    use alloc::borrow::Cow;

    #[test]
    fn should_create_dynamic_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let mut func = add.into_function();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_create_dynamic_closure() {
        let mut func = (|a: i32, b: i32| a + b).into_function();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_create_dynamic_method() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo(i32);

        impl Foo {
            pub fn add(&self, other: &Foo) -> Foo {
                Foo(self.0 + other.0)
            }
        }

        let foo_a = Foo(25);
        let foo_b = Foo(75);

        let mut func = Foo::add.into_function();
        let args = ArgList::new().push_ref(&foo_a).push_ref(&foo_b);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.downcast_ref::<Foo>(), Some(&Foo(100)));
    }

    #[test]
    fn should_allow_zero_args() {
        fn foo() -> String {
            String::from("Hello, World!")
        }

        let mut func = foo.into_function();
        let args = ArgList::new();
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(
            result.downcast_ref::<String>(),
            Some(&String::from("Hello, World!"))
        );
    }

    #[test]
    fn should_allow_unit_return() {
        fn foo(_: i32) {}

        let mut func = foo.into_function();
        let args = ArgList::new().push_owned(123_i32);
        let result = func.call(args).unwrap();
        assert!(result.is_unit());
    }

    #[test]
    fn should_allow_reference_return() {
        fn foo<'a>(value: &'a i32, _: String, _: &bool) -> &'a i32 {
            value
        }

        let value: i32 = 123;
        let mut func = foo.into_function();
        let args = ArgList::new()
            .push_ref(&value)
            .push_owned(String::from("Hello, World!"))
            .push_ref(&true);
        let result = func.call(args).unwrap().unwrap_ref();
        assert_eq!(result.downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_allow_mutable_reference_return() {
        fn foo<'a>(value: &'a mut i32, _: String, _: &bool) -> &'a mut i32 {
            value
        }

        let mut value: i32 = 123;
        let mut func = foo.into_function();
        let args = ArgList::new()
            .push_mut(&mut value)
            .push_owned(String::from("Hello, World!"))
            .push_ref(&true);
        let result = func.call(args).unwrap().unwrap_mut();
        assert_eq!(result.downcast_mut::<i32>(), Some(&mut 123));
    }

    #[test]
    fn should_default_with_function_type_name() {
        fn foo() {}

        let func = foo.into_function();
        assert_eq!(
            func.info().name(),
            Some("bevy_reflect::func::tests::should_default_with_function_type_name::foo")
        );
    }

    #[test]
    fn should_default_with_closure_type_name() {
        let bar = |_: i32| {};

        let func = bar.into_function();
        assert_eq!(
            func.info().name(),
            Some("bevy_reflect::func::tests::should_default_with_closure_type_name::{{closure}}")
        );
    }

    #[test]
    fn should_overwrite_function_name() {
        fn foo() {}

        let func = foo.into_function().with_name("my_function");
        assert_eq!(func.info().name(), Some("my_function"));
    }

    #[test]
    fn should_error_on_missing_args() {
        fn foo(_: i32) {}

        let mut func = foo.into_function();
        let args = ArgList::new();
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FunctionError::InvalidArgCount {
                expected: 1,
                received: 0
            }
        );
    }

    #[test]
    fn should_error_on_too_many_args() {
        fn foo() {}

        let mut func = foo.into_function();
        let args = ArgList::new().push_owned(123_i32);
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FunctionError::InvalidArgCount {
                expected: 0,
                received: 1
            }
        );
    }

    #[test]
    fn should_error_on_invalid_arg_type() {
        fn foo(_: i32) {}

        let mut func = foo.into_function();
        let args = ArgList::new().push_owned(123_u32);
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FunctionError::ArgError(ArgError::UnexpectedType {
                id: ArgId::Index(0),
                expected: Cow::Borrowed(i32::type_path()),
                received: Cow::Borrowed(u32::type_path())
            })
        );
    }

    #[test]
    fn should_error_on_invalid_arg_ownership() {
        fn foo(_: &i32) {}

        let mut func = foo.into_function();
        let args = ArgList::new().push_owned(123_i32);
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FunctionError::ArgError(ArgError::InvalidOwnership {
                id: ArgId::Index(0),
                expected: Ownership::Ref,
                received: Ownership::Owned
            })
        );
    }

    #[test]
    fn should_convert_dynamic_function_with_into_function() {
        fn make_function<'a, F: IntoFunction<'a, M>, M>(f: F) -> DynamicFunction<'a> {
            f.into_function()
        }

        let function: DynamicFunction = make_function(|| {});
        let _: DynamicFunction = make_function(function);
    }
}
