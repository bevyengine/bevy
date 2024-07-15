use bevy_utils::all_tuples;

use crate::func::args::FromArg;
use crate::func::macros::count_tts;
use crate::func::{ArgList, FunctionError, FunctionInfo, FunctionResult, IntoReturn};
use crate::Reflect;

/// A reflection-based version of the [`FnMut`] trait.
///
/// This allows functions to be called dynamically through [reflection].
///
/// This is a supertrait of [`ReflectFn`], and is used for closures that may mutate their environment.
///
/// # Blanket Implementation
///
/// This trait has a blanket implementation that covers everything that [`ReflectFn`] does:
/// - Functions and methods defined with the `fn` keyword
/// - Closures that do not capture their environment
/// - Closures that capture immutable references to their environment
/// - Closures that take ownership of captured variables
///
/// But also allows for:
/// - Closures that capture mutable references to their environment
///
/// For each of the above cases, the function signature may only have up to 15 arguments,
/// not including an optional receiver argument (often `&self` or `&mut self`).
/// This optional receiver argument may be either a mutable or immutable reference to a type.
/// If the return type is also a reference, its lifetime will be bound to the lifetime of this receiver.
///
/// See the [module-level documentation] for more information on valid signatures.
///
/// Arguments are expected to implement [`FromArg`], and the return type is expected to implement [`IntoReturn`].
/// Both of these traits are automatically implemented when using the `Reflect` [derive macro].
///
/// # Example
///
/// ```
/// # use bevy_reflect::func::{ArgList, FunctionInfo, ReflectFnMut, TypedFunction};
/// #
/// let mut list: Vec<i32> = vec![1, 3];
///
/// // `insert` is a closure that captures a mutable reference to `list`
/// let mut insert = |index: usize, value: i32| {
///   list.insert(index, value);
/// };
///
/// let args = ArgList::new().push_owned(1_usize).push_owned(2_i32);
/// let info = insert.get_function_info();
///
/// insert.reflect_call_mut(args, &info).unwrap();
/// assert_eq!(list, vec![1, 2, 3]);
/// ```
///
/// # Trait Parameters
///
/// This trait has a `Marker` type parameter that is used to get around issues with
/// [unconstrained type parameters] when defining impls with generic arguments or return types.
/// This `Marker` can be any type, provided it doesn't conflict with other implementations.
///
/// Additionally, it has a lifetime parameter, `'env`, that is used to bound the lifetime of the function.
/// For most functions, this will end up just being `'static`,
/// however, closures that borrow from their environment will have a lifetime bound to that environment.
///
/// [reflection]: crate
/// [`ReflectFn`]: crate::func::ReflectFn
/// [module-level documentation]: crate::func
/// [derive macro]: derive@crate::Reflect
/// [unconstrained type parameters]: https://doc.rust-lang.org/error_codes/E0207.html
pub trait ReflectFnMut<'env, Marker> {
    /// Call the function with the given arguments and return the result.
    fn reflect_call_mut<'a>(
        &mut self,
        args: ArgList<'a>,
        info: &FunctionInfo,
    ) -> FunctionResult<'a>;
}

/// Helper macro for implementing [`ReflectFnMut`] on Rust closures.
///
/// This currently implements it for the following signatures (where `argX` may be any of `T`, `&T`, or `&mut T`):
/// - `fn(arg0, arg1, ..., argN) -> R`
/// - `fn(&Receiver, arg0, arg1, ..., argN) -> &R`
/// - `fn(&mut Receiver, arg0, arg1, ..., argN) -> &mut R`
/// - `fn(&mut Receiver, arg0, arg1, ..., argN) -> &R`
macro_rules! impl_reflect_fn_mut {
    ($(($Arg:ident, $arg:ident)),*) => {
        // === (...) -> ReturnType === //
        impl<'env, $($Arg,)* ReturnType, Function> ReflectFnMut<'env, fn($($Arg),*) -> [ReturnType]> for Function
        where
            $($Arg: FromArg,)*
            // This clause allows us to convert `ReturnType` into `Return`
            ReturnType: IntoReturn + Reflect,
            Function: FnMut($($Arg),*) -> ReturnType + 'env,
            // This clause essentially asserts that `Arg::Item` is the same type as `Arg`
            Function: for<'a> FnMut($($Arg::Item<'a>),*) -> ReturnType + 'env,
        {
            fn reflect_call_mut<'a>(&mut self, args: ArgList<'a>, _info: &FunctionInfo) -> FunctionResult<'a> {
                const COUNT: usize = count_tts!($($Arg)*);

                if args.len() != COUNT {
                    return Err(FunctionError::InvalidArgCount {
                        expected: COUNT,
                        received: args.len(),
                    });
                }

                let [$($arg,)*] = args.take().try_into().expect("invalid number of arguments");

                #[allow(unused_mut)]
                let mut _index = 0;
                let ($($arg,)*) = ($($Arg::from_arg($arg, {
                    _index += 1;
                    _info.args().get(_index - 1).expect("argument index out of bounds")
                })?,)*);

                Ok((self)($($arg,)*).into_return())
            }
        }

        // === (&self, ...) -> &ReturnType === //
        impl<'env, Receiver, $($Arg,)* ReturnType, Function> ReflectFnMut<'env, fn(&Receiver, $($Arg),*) -> &ReturnType> for Function
        where
            Receiver: Reflect,
            $($Arg: FromArg,)*
            ReturnType: Reflect,
            // This clause allows us to convert `&ReturnType` into `Return`
            for<'a> &'a ReturnType: IntoReturn,
            Function: for<'a> FnMut(&'a Receiver, $($Arg),*) -> &'a ReturnType + 'env,
            // This clause essentially asserts that `Arg::Item` is the same type as `Arg`
            Function: for<'a> FnMut(&'a Receiver, $($Arg::Item<'a>),*) -> &'a ReturnType + 'env,
        {
            fn reflect_call_mut<'a>(&mut self, args: ArgList<'a>, _info: &FunctionInfo) -> FunctionResult<'a> {
                const COUNT: usize = count_tts!(Receiver $($Arg)*);

                if args.len() != COUNT {
                    return Err(FunctionError::InvalidArgCount {
                        expected: COUNT,
                        received: args.len(),
                    });
                }

                let [receiver, $($arg,)*] = args.take().try_into().expect("invalid number of arguments");

                let receiver = receiver.take_ref::<Receiver>(_info.args().get(0).expect("argument index out of bounds"))?;

                #[allow(unused_mut)]
                let mut _index = 1;
                let ($($arg,)*) = ($($Arg::from_arg($arg, {
                    _index += 1;
                    _info.args().get(_index - 1).expect("argument index out of bounds")
                })?,)*);

                Ok((self)(receiver, $($arg,)*).into_return())
            }
        }

        // === (&mut self, ...) -> &mut ReturnType === //
        impl<'env, Receiver, $($Arg,)* ReturnType, Function> ReflectFnMut<'env, fn(&mut Receiver, $($Arg),*) -> &mut ReturnType> for Function
        where
            Receiver: Reflect,
            $($Arg: FromArg,)*
            ReturnType: Reflect,
            // This clause allows us to convert `&mut ReturnType` into `Return`
            for<'a> &'a mut ReturnType: IntoReturn,
            Function: for<'a> FnMut(&'a mut Receiver, $($Arg),*) -> &'a mut ReturnType + 'env,
            // This clause essentially asserts that `Arg::Item` is the same type as `Arg`
            Function: for<'a> FnMut(&'a mut Receiver, $($Arg::Item<'a>),*) -> &'a mut ReturnType + 'env,
        {
            fn reflect_call_mut<'a>(&mut self, args: ArgList<'a>, _info: &FunctionInfo) -> FunctionResult<'a> {
                const COUNT: usize = count_tts!(Receiver $($Arg)*);

                if args.len() != COUNT {
                    return Err(FunctionError::InvalidArgCount {
                        expected: COUNT,
                        received: args.len(),
                    });
                }

                let [receiver, $($arg,)*] = args.take().try_into().expect("invalid number of arguments");

                let receiver = receiver.take_mut::<Receiver>(_info.args().get(0).expect("argument index out of bounds"))?;

                #[allow(unused_mut)]
                let mut _index = 1;
                let ($($arg,)*) = ($($Arg::from_arg($arg, {
                    _index += 1;
                    _info.args().get(_index - 1).expect("argument index out of bounds")
                })?,)*);

                Ok((self)(receiver, $($arg,)*).into_return())
            }
        }

        // === (&mut self, ...) -> &ReturnType === //
        impl<'env, Receiver, $($Arg,)* ReturnType, Function> ReflectFnMut<'env, fn(&mut Receiver, $($Arg),*) -> &ReturnType> for Function
        where
            Receiver: Reflect,
            $($Arg: FromArg,)*
            ReturnType: Reflect,
            // This clause allows us to convert `&ReturnType` into `Return`
            for<'a> &'a ReturnType: IntoReturn,
            Function: for<'a> FnMut(&'a mut Receiver, $($Arg),*) -> &'a ReturnType + 'env,
            // This clause essentially asserts that `Arg::Item` is the same type as `Arg`
            Function: for<'a> FnMut(&'a mut Receiver, $($Arg::Item<'a>),*) -> &'a ReturnType + 'env,
        {
            fn reflect_call_mut<'a>(&mut self, args: ArgList<'a>, _info: &FunctionInfo) -> FunctionResult<'a> {
                const COUNT: usize = count_tts!(Receiver $($Arg)*);

                if args.len() != COUNT {
                    return Err(FunctionError::InvalidArgCount {
                        expected: COUNT,
                        received: args.len(),
                    });
                }

                let [receiver, $($arg,)*] = args.take().try_into().expect("invalid number of arguments");

                let receiver = receiver.take_mut::<Receiver>(_info.args().get(0).expect("argument index out of bounds"))?;

                #[allow(unused_mut)]
                let mut _index = 1;
                let ($($arg,)*) = ($($Arg::from_arg($arg, {
                    _index += 1;
                    _info.args().get(_index - 1).expect("argument index out of bounds")
                })?,)*);

                Ok((self)(receiver, $($arg,)*).into_return())
            }
        }
    };
}

all_tuples!(impl_reflect_fn_mut, 0, 15, Arg, arg);
