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
        impl<$($Arg,)* R, F> $crate::func::IntoFunction<fn($($Arg),*) -> R> for F
        where
            $($Arg: $crate::func::args::FromArg + $crate::TypePath,)*
            R: $crate::Reflect,
            F: FnMut($($Arg),*) -> R + 'static,
            F: for<'a> FnMut($($Arg::Item<'a>),*) -> R + 'static,
        {
            fn into_function(mut self) -> $crate::func::Function {
                const COUNT: usize = count_tts!($($Arg)*);

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
                    $crate::func::utils::to_function_result((self)($($arg,)*))
                }, {
                    #[allow(unused_mut)]
                    let mut _index = 0;
                    vec![
                        $($crate::func::args::ArgInfo::new::<$Arg>($crate::func::args::ArgId::Index({
                            _index += 1;
                            _index - 1
                        })),)*
                    ]
                })
            }
        }
    }
}

all_tuples!(impl_into_function, 0, 15, Arg, arg);
