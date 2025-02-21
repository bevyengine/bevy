use variadics_please::all_tuples;

use crate::{
    func::{
        args::{ArgCount, FromArg},
        macros::count_tokens,
        ArgList, FunctionError, FunctionResult, IntoReturn, ReflectFnMut,
    },
    Reflect, TypePath,
};

/// A reflection-based version of the [`Fn`] trait.
///
/// This allows functions to be called dynamically through [reflection].
///
/// # Blanket Implementation
///
/// This trait has a blanket implementation that covers:
/// - Functions and methods defined with the `fn` keyword
/// - Anonymous functions
/// - Function pointers
/// - Closures that capture immutable references to their environment
/// - Closures that take ownership of captured variables
///
/// For each of the above cases, the function signature may only have up to 15 arguments,
/// not including an optional receiver argument (often `&self` or `&mut self`).
/// This optional receiver argument may be either a mutable or immutable reference to a type.
/// If the return type is also a reference, its lifetime will be bound to the lifetime of this receiver.
///
/// See the [module-level documentation] for more information on valid signatures.
///
/// To handle functions that capture mutable references to their environment,
/// see the [`ReflectFnMut`] trait instead.
///
/// Arguments are expected to implement [`FromArg`], and the return type is expected to implement [`IntoReturn`].
/// Both of these traits are automatically implemented when using the `Reflect` [derive macro].
///
/// # Example
///
/// ```
/// # use bevy_reflect::func::{ArgList, FunctionInfo, ReflectFn};
/// #
/// fn add(a: i32, b: i32) -> i32 {
///   a + b
/// }
///
/// let args = ArgList::new().with_owned(25_i32).with_owned(75_i32);
///
/// let value = add.reflect_call(args).unwrap().unwrap_owned();
/// assert_eq!(value.try_take::<i32>().unwrap(), 100);
/// ```
///
/// # Trait Parameters
///
/// This trait has a `Marker` type parameter that is used to get around issues with
/// [unconstrained type parameters] when defining impls with generic arguments or return types.
/// This `Marker` can be any type, provided it doesn't conflict with other implementations.
///
/// Additionally, it has a lifetime parameter, `'env`, that is used to bound the lifetime of the function.
/// For named functions and some closures, this will end up just being `'static`,
/// however, closures that borrow from their environment will have a lifetime bound to that environment.
///
/// [reflection]: crate
/// [module-level documentation]: crate::func
/// [derive macro]: derive@crate::Reflect
/// [unconstrained type parameters]: https://doc.rust-lang.org/error_codes/E0207.html
pub trait ReflectFn<'env, Marker>: ReflectFnMut<'env, Marker> {
    /// Call the function with the given arguments and return the result.
    fn reflect_call<'a>(&self, args: ArgList<'a>) -> FunctionResult<'a>;
}

/// Helper macro for implementing [`ReflectFn`] on Rust functions.
///
/// This currently implements it for the following signatures (where `argX` may be any of `T`, `&T`, or `&mut T`):
/// - `Fn(arg0, arg1, ..., argN) -> R`
/// - `Fn(&Receiver, arg0, arg1, ..., argN) -> &R`
/// - `Fn(&mut Receiver, arg0, arg1, ..., argN) -> &mut R`
/// - `Fn(&mut Receiver, arg0, arg1, ..., argN) -> &R`
macro_rules! impl_reflect_fn {
    ($(($Arg:ident, $arg:ident)),*) => {
        // === (...) -> ReturnType === //
        impl<'env, $($Arg,)* ReturnType, Function> ReflectFn<'env, fn($($Arg),*) -> [ReturnType]> for Function
        where
            $($Arg: FromArg,)*
            // This clause allows us to convert `ReturnType` into `Return`
            ReturnType: IntoReturn + Reflect,
            Function: Fn($($Arg),*) -> ReturnType + 'env,
            // This clause essentially asserts that `Arg::This` is the same type as `Arg`
            Function: for<'a> Fn($($Arg::This<'a>),*) -> ReturnType + 'env,
        {
            #[expect(
                clippy::allow_attributes,
                reason = "This lint is part of a macro, which may not always trigger the `unused_mut` lint."
            )]
            #[allow(
                unused_mut,
                reason = "Some invocations of this macro may trigger the `unused_mut` lint, where others won't."
            )]
            fn reflect_call<'a>(&self, mut args: ArgList<'a>) -> FunctionResult<'a> {
                const COUNT: usize = count_tokens!($($Arg)*);

                if args.len() != COUNT {
                    return Err(FunctionError::ArgCountMismatch {
                        expected: ArgCount::new(COUNT).unwrap(),
                        received: args.len(),
                    });
                }

                // Extract all arguments (in order)
                $(let $arg = args.take::<$Arg>()?;)*

                Ok((self)($($arg,)*).into_return())
            }
        }

        // === (&self, ...) -> &ReturnType === //
        impl<'env, Receiver, $($Arg,)* ReturnType, Function> ReflectFn<'env, fn(&Receiver, $($Arg),*) -> &ReturnType> for Function
        where
            Receiver: Reflect + TypePath,
            $($Arg: FromArg,)*
            ReturnType: Reflect,
            // This clause allows us to convert `&ReturnType` into `Return`
            for<'a> &'a ReturnType: IntoReturn,
            Function: for<'a> Fn(&'a Receiver, $($Arg),*) -> &'a ReturnType + 'env,
            // This clause essentially asserts that `Arg::This` is the same type as `Arg`
            Function: for<'a> Fn(&'a Receiver, $($Arg::This<'a>),*) -> &'a ReturnType + 'env,
        {
            fn reflect_call<'a>(&self, mut args: ArgList<'a>) -> FunctionResult<'a> {
                const COUNT: usize = count_tokens!(Receiver $($Arg)*);

                if args.len() != COUNT {
                    return Err(FunctionError::ArgCountMismatch {
                        expected: ArgCount::new(COUNT).unwrap(),
                        received: args.len(),
                    });
                }

                // Extract all arguments (in order)
                let receiver = args.take_ref::<Receiver>()?;
                $(let $arg = args.take::<$Arg>()?;)*

                Ok((self)(receiver, $($arg,)*).into_return())
            }
        }

        // === (&mut self, ...) -> &mut ReturnType === //
        impl<'env, Receiver, $($Arg,)* ReturnType, Function> ReflectFn<'env, fn(&mut Receiver, $($Arg),*) -> &mut ReturnType> for Function
        where
            Receiver: Reflect + TypePath,
            $($Arg: FromArg,)*
            ReturnType: Reflect,
            // This clause allows us to convert `&mut ReturnType` into `Return`
            for<'a> &'a mut ReturnType: IntoReturn,
            Function: for<'a> Fn(&'a mut Receiver, $($Arg),*) -> &'a mut ReturnType + 'env,
            // This clause essentially asserts that `Arg::This` is the same type as `Arg`
            Function: for<'a> Fn(&'a mut Receiver, $($Arg::This<'a>),*) -> &'a mut ReturnType + 'env,
        {
            fn reflect_call<'a>(&self, mut args: ArgList<'a>) -> FunctionResult<'a> {
                const COUNT: usize = count_tokens!(Receiver $($Arg)*);

                if args.len() != COUNT {
                    return Err(FunctionError::ArgCountMismatch {
                        expected: ArgCount::new(COUNT).unwrap(),
                        received: args.len(),
                    });
                }

                // Extract all arguments (in order)
                let receiver = args.take_mut::<Receiver>()?;
                $(let $arg = args.take::<$Arg>()?;)*

                Ok((self)(receiver, $($arg,)*).into_return())
            }
        }

        // === (&mut self, ...) -> &ReturnType === //
        impl<'env, Receiver, $($Arg,)* ReturnType, Function> ReflectFn<'env, fn(&mut Receiver, $($Arg),*) -> &ReturnType> for Function
        where
            Receiver: Reflect + TypePath,
            $($Arg: FromArg,)*
            ReturnType: Reflect,
            // This clause allows us to convert `&ReturnType` into `Return`
            for<'a> &'a ReturnType: IntoReturn,
            Function: for<'a> Fn(&'a mut Receiver, $($Arg),*) -> &'a ReturnType + 'env,
            // This clause essentially asserts that `Arg::This` is the same type as `Arg`
            Function: for<'a> Fn(&'a mut Receiver, $($Arg::This<'a>),*) -> &'a ReturnType + 'env,
        {
            fn reflect_call<'a>(&self, mut args: ArgList<'a>) -> FunctionResult<'a> {
                const COUNT: usize = count_tokens!(Receiver $($Arg)*);

                if args.len() != COUNT {
                    return Err(FunctionError::ArgCountMismatch {
                        expected: ArgCount::new(COUNT).unwrap(),
                        received: args.len(),
                    });
                }

                // Extract all arguments (in order)
                let receiver = args.take_mut::<Receiver>()?;
                $(let $arg = args.take::<$Arg>()?;)*

                Ok((self)(receiver, $($arg,)*).into_return())
            }
        }
    };
}

all_tuples!(impl_reflect_fn, 0, 15, Arg, arg);
