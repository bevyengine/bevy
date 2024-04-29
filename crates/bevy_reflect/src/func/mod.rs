//! Reflection-based dynamic functions.
//!
//! This module provides a way to pass around and call functions dynamically
//! using the [`Function`] type.
//!
//! Many simple functions and closures can be automatically converted to [`Function`]
//! using the [`IntoFunction`] trait.
//!
//! Once the [`Function`] is created, it can be called with a set of arguments provided
//! via an [`ArgList`].
//!
//! This returns a [`FunctionResult`] containing the [`Return`] value,
//! which can be used to extract a [`Reflect`] trait object.
//!
//!
//! # Example
//!
//! ```
//! # use bevy_reflect::func::args::ArgList;
//! # use bevy_reflect::func::{Function, FunctionResult, IntoFunction, Return};
//! fn add(a: i32, b: i32) -> i32 {
//!   a + b
//! }
//!
//! let mut func: Function = add.into_function();
//! let args: ArgList = ArgList::default().push_owned(25_i32).push_owned(75_i32);
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

        crate::func::args::impl_from_arg!(Foo);
        crate::func::args::impl_get_ownership!(Foo);
        crate::func::return_type::impl_into_return!(Foo);

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
    fn should_error_on_missing_args() {
        fn foo(_: i32) {}

        let mut func = foo.into_function();
        let args = ArgList::new();
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FuncError::ArgCount {
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
            FuncError::ArgCount {
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
            FuncError::Arg(ArgError::UnexpectedType {
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
            FuncError::Arg(ArgError::InvalidOwnership {
                id: ArgId::Index(0),
                expected: Ownership::Ref,
                received: Ownership::Owned
            })
        );
    }
}
