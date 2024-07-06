//! Reflection-based dynamic functions.
//!
//! This module provides a way to pass around and call functions dynamically
//! using the [`DynamicFunction`], [`DynamicClosure`], and [`DynamicClosureMut`] types.
//!
//! Many simple functions and closures can be automatically converted to these types
//! using the [`IntoFunction`], [`IntoClosure`], and [`IntoClosureMut`] traits, respectively.
//!
//! Once this dynamic representation is created, it can be called with a set of arguments provided
//! via an [`ArgList`].
//!
//! This returns a [`FunctionResult`] containing the [`Return`] value,
//! which can be used to extract a [`Reflect`] trait object.
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
//! # Functions vs Closures
//!
//! In Rust, a "function" is any callable that does not capture its environment.
//! These are typically defined with the `fn` keyword, but may also use anonymous function syntax.
//!
//! ```rust
//! // This is a standard Rust function:
//! fn add(a: i32, b: i32) -> i32 {
//!   a + b
//! }
//!
//! // This is an anonymous Rust function:
//! let add = |a: i32, b: i32| a + b;
//! ```
//!
//! Rust also has the concept of "closures", which are special functions that capture their environment.
//! These are always defined with anonymous function syntax.
//!
//! ```rust
//! // A closure that captures an immutable reference to a variable
//! let c = 123;
//! let add = |a: i32, b: i32| a + b + c;
//!
//! // A closure that captures a mutable reference to a variable
//! let mut total = 0;
//! let add = |a: i32, b: i32| total += a + b;
//!
//! // A closure that takes ownership of its captured variables by moving them
//! let c = 123;
//! let add = move |a: i32, b: i32| a + b + c;
//! ```
//!
//! Each callable may be considered a subset of the other:
//! functions are a subset of immutable closures which are a subset of mutable closures.
//!
//! This means that, in terms of traits, you could imagine that any type that implements
//! [`IntoFunction`], also implements [`IntoClosure`] and [`IntoClosureMut`].
//! And every type that implements [`IntoClosure`] also implements [`IntoClosureMut`].
//!
//! # Valid Signatures
//!
//! Many of the traits in this module have default blanket implementations over a specific set of function signatures.
//!
//! These signatures are:
//! - `(...) -> R`
//! - `for<'a> (&'a arg, ...) -> &'a R`
//! - `for<'a> (&'a mut arg, ...) -> &'a R`
//! - `for<'a> (&'a mut arg, ...) -> &'a mut R`
//!
//! Where `...` represents 0 to 15 arguments (inclusive) of the form `T`, `&T`, or `&mut T`.
//! The lifetime of any reference to the return type `R`, must be tied to a "receiver" argument
//! (i.e. the first argument in the signature, normally `self`).
//!
//! Each trait will also have its own requirements for what traits are required for both arguments and return types,
//! but a good rule-of-thumb is that all types should derive [`Reflect`].
//!
//! The reason for such a small subset of valid signatures is due to limitations in Rust—
//! namely the [lack of variadic generics] and certain [coherence issues].
//!
//! For other functions that don't conform to one of the above signatures,
//! [`DynamicFunction`] and [`DynamicClosure`] can instead be created manually.
//!
//! [`Reflect`]: crate::Reflect
//! [lack of variadic generics]: https://poignardazur.github.io/2024/05/25/report-on-rustnl-variadics/
//! [coherence issues]: https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#coherence-leak-check

pub use args::{ArgError, ArgList, ArgValue};
pub use closures::*;
pub use error::*;
pub use function::*;
pub use info::*;
pub use into_function::*;
pub use reflect_fn::*;
pub use reflect_fn_mut::*;
pub use return_type::*;

pub mod args;
mod closures;
mod error;
mod function;
mod info;
mod into_function;
pub(crate) mod macros;
mod reflect_fn;
mod reflect_fn_mut;
mod return_type;

#[cfg(test)]
mod tests {
    use alloc::borrow::Cow;

    use crate::func::args::{ArgError, ArgList, Ownership};
    use crate::TypePath;

    use super::*;

    #[test]
    fn should_error_on_missing_args() {
        fn foo(_: i32) {}

        let func = foo.into_function();
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

        let func = foo.into_function();
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

        let func = foo.into_function();
        let args = ArgList::new().push_owned(123_u32);
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FunctionError::ArgError(ArgError::UnexpectedType {
                index: 0,
                expected: Cow::Borrowed(i32::type_path()),
                received: Cow::Borrowed(u32::type_path())
            })
        );
    }

    #[test]
    fn should_error_on_invalid_arg_ownership() {
        fn foo(_: &i32) {}

        let func = foo.into_function();
        let args = ArgList::new().push_owned(123_i32);
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FunctionError::ArgError(ArgError::InvalidOwnership {
                index: 0,
                expected: Ownership::Ref,
                received: Ownership::Owned
            })
        );
    }
}
