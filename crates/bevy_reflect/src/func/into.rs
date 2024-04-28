use crate::func::function::Function;
use bevy_utils::all_tuples;

pub trait IntoFunction<T> {
    fn into_function(self) -> Function;
}

// https://veykril.github.io/tlborm/decl-macros/building-blocks/counting.html#bit-twiddling
macro_rules! count_tts {
    () => { 0 };
    ($odd:tt $($a:tt $b:tt)*) => { (count_tts!($($a)*) << 1) | 1 };
    ($($a:tt $even:tt)*) => { count_tts!($($a)*) << 1 };
}

macro_rules! impl_into_function {
    ($(($Arg:ident, $arg:ident)),*) => {
        // === Owned Return === //
        impl<$($Arg,)* R, F> $crate::func::IntoFunction<fn($($Arg),*) -> R> for F
        where
            $($Arg: $crate::func::args::FromArg + $crate::func::args::GetOwnership + $crate::TypePath,)*
            R: $crate::func::IntoReturn + $crate::func::args::GetOwnership + $crate::TypePath,
            F: FnMut($($Arg),*) -> R + 'static,
            F: for<'a> FnMut($($Arg::Item<'a>),*) -> R + 'static,
        {
            fn into_function(mut self) -> $crate::func::Function {
                const COUNT: usize = count_tts!($($Arg)*);

                let info = $crate::func::FunctionInfo::new({
                    #[allow(unused_mut)]
                    let mut _index = 0;
                    vec![
                        $($crate::func::args::ArgInfo::new::<$Arg>({
                            _index += 1;
                            _index - 1
                        }),)*
                    ]
                }).with_return_info($crate::func::ReturnInfo::new::<R>());

                $crate::func::Function::new(move |args, _info| {
                    if args.len() != COUNT {
                        return Err($crate::func::error::FuncError::ArgCount {
                            expected: COUNT,
                            received: args.len(),
                        });
                    }

                    let [$($arg,)*] = args.take().try_into().ok().expect("invalid number of arguments");

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

        // === Ref Return === //
        impl<Receiver, $($Arg,)* R, F> $crate::func::IntoFunction<fn(&Receiver, $($Arg),*) -> (R,)> for F
        where
            Receiver: $crate::Reflect + $crate::TypePath,
            for<'a> &'a Receiver: $crate::func::args::GetOwnership,
            R: $crate::Reflect + $crate::TypePath,
            for<'a> &'a R: $crate::func::args::GetOwnership,
            $($Arg: $crate::func::args::FromArg + $crate::func::args::GetOwnership + $crate::TypePath,)*
            F: for<'a> FnMut(&'a Receiver, $($Arg),*) -> &'a R + 'static,
            F: for<'a> FnMut(&'a Receiver, $($Arg::Item<'a>),*) -> &'a R + 'static,
        {
            fn into_function(mut self) -> $crate::func::Function {
                const COUNT: usize = count_tts!(Receiver $($Arg)*);

                let info = $crate::func::FunctionInfo::new({
                    #[allow(unused_mut)]
                    let mut _index = 1;
                    vec![
                        $crate::func::args::ArgInfo::new::<&Receiver>(0),
                        $($crate::func::args::ArgInfo::new::<$Arg>({
                            _index += 1;
                            _index - 1
                        }),)*
                    ]
                }).with_return_info($crate::func::ReturnInfo::new::<&R>());

                $crate::func::Function::new(move |args, _info| {
                    if args.len() != COUNT {
                        return Err($crate::func::error::FuncError::ArgCount {
                            expected: COUNT,
                            received: args.len(),
                        });
                    }

                    let [receiver, $($arg,)*] = args.take().try_into().ok().expect("invalid number of arguments");

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

        // === Mut Return === //
        impl<Receiver, $($Arg,)* R, F> $crate::func::IntoFunction<fn(&mut Receiver, $($Arg),*) -> (R,)> for F
        where
            Receiver: $crate::Reflect + $crate::TypePath,
            for<'a> &'a mut Receiver: $crate::func::args::GetOwnership,
            R: $crate::Reflect + $crate::TypePath,
            for<'a> &'a mut R: $crate::func::args::GetOwnership,
            $($Arg: $crate::func::args::FromArg + $crate::func::args::GetOwnership + $crate::TypePath,)*
            F: for<'a> FnMut(&'a mut Receiver, $($Arg),*) -> &'a mut R + 'static,
            F: for<'a> FnMut(&'a mut Receiver, $($Arg::Item<'a>),*) -> &'a mut R + 'static,
        {
            fn into_function(mut self) -> $crate::func::Function {
                const COUNT: usize = count_tts!(Receiver $($Arg)*);

                let info = $crate::func::FunctionInfo::new({
                    #[allow(unused_mut)]
                    let mut _index = 1;
                    vec![
                        $crate::func::args::ArgInfo::new::<&mut Receiver>(0),
                        $($crate::func::args::ArgInfo::new::<$Arg>({
                            _index += 1;
                            _index - 1
                        }),)*
                    ]
                }).with_return_info($crate::func::ReturnInfo::new::<&mut R>());

                $crate::func::Function::new(move |args, _info| {
                    if args.len() != COUNT {
                        return Err($crate::func::error::FuncError::ArgCount {
                            expected: COUNT,
                            received: args.len(),
                        });
                    }

                    let [receiver, $($arg,)*] = args.take().try_into().ok().expect("invalid number of arguments");

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
    };
}

all_tuples!(impl_into_function, 0, 15, Arg, arg);
