use crate::func::function::DynamicFunction;
use bevy_utils::all_tuples;

/// A trait for types that can be converted into a [`DynamicFunction`].
///
/// # Blanket Implementation
///
/// This trait has a blanket implementation that covers many functions, closures, and methods.
/// And though it works for many cases, it does have some limitations.
///
/// ## Arguments
///
/// Firstly, the function signature may only have up to 15 arguments
/// (or 16 if the first argument is a mutable/immutable reference).
/// This limitation is unfortunately due to the [lack of variadic generics] in Rust.
///
/// Each argument must implement [`FromArg`], [`GetOwnership`], and [`TypePath`].
///
///
/// ```compile_fail
/// # use bevy_reflect::func::IntoFunction;
/// fn too_many_args(
///   arg01: i32,
///   arg02: i32,
///   arg03: i32,
///   arg04: i32,
///   arg05: i32,
///   arg06: i32,
///   arg07: i32,
///   arg08: i32,
///   arg09: i32,
///   arg10: i32,
///   arg11: i32,
///   arg12: i32,
///   arg13: i32,
///   arg14: i32,
///   arg15: i32,
///   arg16: i32,
/// ) {
///   // ...
/// }
///
/// // This will fail to compile:
/// too_many_args.into_function();
/// ```
///
/// ## Return Type
///
/// Secondly, the allowed return type is dependent on the first argument of the function:
/// - If the first argument is an immutable reference,
/// then the return type may be either an owned type, a static reference type, or a reference type
/// bound to the lifetime of the first argument.
/// - If the first argument is a mutable reference,
/// then the return type may be either an owned type, a static reference type, or be a mutable reference type
/// bound to the lifetime of the first argument.
/// - If the first argument is an owned type,
/// then the return type may be either an owned type or a static reference type.
///
/// The return type must always implement [`GetOwnership`] and [`TypePath`].
/// If it is either an owned type or a static reference type,
/// then it must also implement [`IntoReturn`].
/// Otherwise, it must also implement [`Reflect`].
///
/// Note that both `GetOwnership`, `TypePath`, and `IntoReturn` are automatically implemented
/// when [deriving `Reflect`].
///
/// ```
/// # use bevy_reflect::func::IntoFunction;
/// fn owned_return(arg: i32) -> i32 { arg * 2 }
/// fn ref_return(arg: &i32) -> &i32 { arg }
/// fn mut_return(arg: &mut i32) -> &mut i32 { arg }
/// fn static_return(arg: i32) -> &'static i32 { &123 }
///
/// owned_return.into_function();
/// ref_return.into_function();
/// mut_return.into_function();
/// static_return.into_function();
/// ```
///
/// [lack of variadic generics]: https://poignardazur.github.io/2024/05/25/report-on-rustnl-variadics/
/// [`FromArg`]: crate::func::args::FromArg
/// [`GetOwnership`]: crate::func::args::GetOwnership
/// [`TypePath`]: crate::TypePath
/// [`IntoReturn`]: crate::func::IntoReturn
/// [`Reflect`]: crate::Reflect
/// [deriving `Reflect`]: derive@crate::Reflect
pub trait IntoFunction<'env, T> {
    /// Converts [`Self`] into a [`DynamicFunction`].
    fn into_function(self) -> DynamicFunction<'env>;
}

/// Helper macro that returns the number of tokens it receives.
///
/// This is used to get the argument count.
///
/// See [here] for details.
///
/// [here]: https://veykril.github.io/tlborm/decl-macros/building-blocks/counting.html#bit-twiddling
macro_rules! count_tts {
    () => { 0 };
    ($odd:tt $($a:tt $b:tt)*) => { (count_tts!($($a)*) << 1) | 1 };
    ($($a:tt $even:tt)*) => { count_tts!($($a)*) << 1 };
}

/// Helper macro for implementing [`IntoFunction`] on Rust functions.
///
/// This currently implements it for the following signatures (where `argX` may be any of `T`, `&T`, or `&mut T`):
/// - `fn(arg0, arg1, ..., argN) -> R`
/// - `fn(&Receiver, arg0, arg1, ..., argN) -> &R`
/// - `fn(&mut Receiver, arg0, arg1, ..., argN) -> &mut R`
/// - `fn(&mut Receiver, arg0, arg1, ..., argN) -> &R`
macro_rules! impl_into_function {
    ($(($Arg:ident, $arg:ident)),*) => {
        // === Owned Return === //
        impl<'env, $($Arg,)* R, F> $crate::func::IntoFunction<'env, fn($($Arg),*) -> R> for F
        where
            $($Arg: $crate::func::args::FromArg + $crate::func::args::GetOwnership + $crate::TypePath,)*
            R: $crate::func::IntoReturn + $crate::func::args::GetOwnership + $crate::TypePath,
            F: FnMut($($Arg),*) -> R + 'env,
            F: for<'a> FnMut($($Arg::Item<'a>),*) -> R + 'env,
        {
            fn into_function(mut self) -> $crate::func::DynamicFunction<'env> {
                const COUNT: usize = count_tts!($($Arg)*);

                let info = $crate::func::FunctionInfo::new()
                    .with_args({
                        #[allow(unused_mut)]
                        let mut _index = 0;
                        vec![
                            $($crate::func::args::ArgInfo::new::<$Arg>({
                                _index += 1;
                                _index - 1
                            }),)*
                        ]
                    })
                    .with_return_info($crate::func::ReturnInfo::new::<R>());

                $crate::func::DynamicFunction::new(move |args, _info| {
                    if args.len() != COUNT {
                        return Err($crate::func::error::FunctionError::ArgCount {
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
                }, info)
            }
        }

        // === Ref Receiver + Ref Return === //
        impl<'env, Receiver, $($Arg,)* R, F> $crate::func::IntoFunction<'env, fn(&Receiver, $($Arg),*) -> fn(&R)> for F
        where
            Receiver: $crate::Reflect + $crate::TypePath,
            for<'a> &'a Receiver: $crate::func::args::GetOwnership,
            R: $crate::Reflect + $crate::TypePath,
            for<'a> &'a R: $crate::func::args::GetOwnership,
            $($Arg: $crate::func::args::FromArg + $crate::func::args::GetOwnership + $crate::TypePath,)*
            F: for<'a> FnMut(&'a Receiver, $($Arg),*) -> &'a R + 'env,
            F: for<'a> FnMut(&'a Receiver, $($Arg::Item<'a>),*) -> &'a R + 'env,
        {
            fn into_function(mut self) -> $crate::func::DynamicFunction<'env> {
                const COUNT: usize = count_tts!(Receiver $($Arg)*);

                let info = $crate::func::FunctionInfo::new()
                    .with_args({
                        #[allow(unused_mut)]
                        let mut _index = 1;
                        vec![
                            $crate::func::args::ArgInfo::new::<&Receiver>(0),
                            $($crate::func::args::ArgInfo::new::<$Arg>({
                                _index += 1;
                                _index - 1
                            }),)*
                        ]
                    })
                    .with_return_info($crate::func::ReturnInfo::new::<&R>());

                $crate::func::DynamicFunction::new(move |args, _info| {
                    if args.len() != COUNT {
                        return Err($crate::func::error::FunctionError::ArgCount {
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
                    Ok($crate::func::Return::Ref((self)(receiver, $($arg,)*)))
                }, info)
            }
        }

        // === Mut Receiver + Mut Return === //
        impl<'env, Receiver, $($Arg,)* R, F> $crate::func::IntoFunction<'env, fn(&mut Receiver, $($Arg),*) -> fn(&mut R)> for F
        where
            Receiver: $crate::Reflect + $crate::TypePath,
            for<'a> &'a mut Receiver: $crate::func::args::GetOwnership,
            R: $crate::Reflect + $crate::TypePath,
            for<'a> &'a mut R: $crate::func::args::GetOwnership,
            $($Arg: $crate::func::args::FromArg + $crate::func::args::GetOwnership + $crate::TypePath,)*
            F: for<'a> FnMut(&'a mut Receiver, $($Arg),*) -> &'a mut R + 'env,
            F: for<'a> FnMut(&'a mut Receiver, $($Arg::Item<'a>),*) -> &'a mut R + 'env,
        {
            fn into_function(mut self) -> $crate::func::DynamicFunction<'env> {
                const COUNT: usize = count_tts!(Receiver $($Arg)*);

                let info = $crate::func::FunctionInfo::new()
                    .with_args({
                        #[allow(unused_mut)]
                        let mut _index = 1;
                        vec![
                            $crate::func::args::ArgInfo::new::<&mut Receiver>(0),
                            $($crate::func::args::ArgInfo::new::<$Arg>({
                                _index += 1;
                                _index - 1
                            }),)*
                        ]
                    })
                    .with_return_info($crate::func::ReturnInfo::new::<&mut R>());

                $crate::func::DynamicFunction::new(move |args, _info| {
                    if args.len() != COUNT {
                        return Err($crate::func::error::FunctionError::ArgCount {
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
                    Ok($crate::func::Return::Mut((self)(receiver, $($arg,)*)))
                }, info)
            }
        }

        // === Mut Receiver + Ref Return === //
        impl<'env, Receiver, $($Arg,)* R, F> $crate::func::IntoFunction<'env, fn(&mut Receiver, $($Arg),*) -> fn(&mut R) -> &R> for F
        where
            Receiver: $crate::Reflect + $crate::TypePath,
            for<'a> &'a mut Receiver: $crate::func::args::GetOwnership,
            R: $crate::Reflect + $crate::TypePath,
            for<'a> &'a mut R: $crate::func::args::GetOwnership,
            $($Arg: $crate::func::args::FromArg + $crate::func::args::GetOwnership + $crate::TypePath,)*
            F: for<'a> FnMut(&'a mut Receiver, $($Arg),*) -> &'a R + 'env,
            F: for<'a> FnMut(&'a mut Receiver, $($Arg::Item<'a>),*) -> &'a R + 'env,
        {
            fn into_function(mut self) -> $crate::func::DynamicFunction<'env> {
                const COUNT: usize = count_tts!(Receiver $($Arg)*);

                let info = $crate::func::FunctionInfo::new()
                    .with_args({
                        #[allow(unused_mut)]
                        let mut _index = 1;
                        vec![
                            $crate::func::args::ArgInfo::new::<&mut Receiver>(0),
                            $($crate::func::args::ArgInfo::new::<$Arg>({
                                _index += 1;
                                _index - 1
                            }),)*
                        ]
                    })
                    .with_return_info($crate::func::ReturnInfo::new::<&mut R>());

                $crate::func::DynamicFunction::new(move |args, _info| {
                    if args.len() != COUNT {
                        return Err($crate::func::error::FunctionError::ArgCount {
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
                    Ok($crate::func::Return::Ref((self)(receiver, $($arg,)*)))
                }, info)
            }
        }
    };
}

all_tuples!(impl_into_function, 0, 15, Arg, arg);
