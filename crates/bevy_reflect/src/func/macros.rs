/// Helper macro to implement the necessary traits for function reflection.
///
/// This macro calls the following macros:
/// - [`impl_get_ownership`](crate::func::args::impl_get_ownership)
/// - [`impl_from_arg`](crate::func::args::impl_from_arg)
/// - [`impl_into_return`](crate::func::impl_into_return)
///
/// # Syntax
///
/// For non-generic types, the macro simply expects the type:
///
/// ```ignore
/// impl_function_traits!(foo::bar::Baz);
/// ```
///
/// For generic types, however, the generic type parameters must also be given in angle brackets (`<` and `>`):
///
/// ```ignore
/// impl_function_traits!(foo::bar::Baz<T, U>; <T: Clone, U>);
/// ```
///
/// For generic const parameters, they must be given in square brackets (`[` and `]`):
///
/// ```ignore
/// impl_function_traits!(foo::bar::Baz<T, N>; <T> [const N: usize]);
/// ```
macro_rules! impl_function_traits {
    (
        $ty: ty
        $(;
            < $($T: ident $(: $T1: tt $(+ $T2: tt)*)?),* >
        )?
        $(
            [ $(const $N: ident : $size: ident),* ]
        )?
        $(
            where $($U: ty $(: $U1: tt $(+ $U2: tt)*)?),*
        )?
    ) => {
        $crate::func::args::impl_get_ownership!(
            $ty
            $(;
                < $($T $(: $T1 $(+ $T2)*)?),* >
            )?
            $(
                [ $(const $N : $size),* ]
            )?
            $(
                where $($U $(: $U1 $(+ $U2)*)?),*
            )?
        );
        $crate::func::args::impl_from_arg!(
            $ty
            $(;
                < $($T $(: $T1 $(+ $T2)*)?),* >
            )?
            $(
                [ $(const $N : $size),* ]
            )?
            $(
                where $($U $(: $U1 $(+ $U2)*)?),*
            )?
        );
        $crate::func::impl_into_return!(
            $ty
            $(;
                < $($T $(: $T1 $(+ $T2)*)?),* >
            )?
            $(
                [ $(const $N : $size),* ]
            )?
            $(
                where $($U $(: $U1 $(+ $U2)*)?),*
            )?
        );
    };
}

pub(crate) use impl_function_traits;

/// Helper macro that returns the number of tokens it receives.
///
/// See [here] for details.
///
/// [here]: https://veykril.github.io/tlborm/decl-macros/building-blocks/counting.html#bit-twiddling
macro_rules! count_tokens {
    () => { 0 };
    ($odd:tt $($a:tt $b:tt)*) => { ($crate::func::macros::count_tokens!($($a)*) << 1) | 1 };
    ($($a:tt $even:tt)*) => { $crate::func::macros::count_tokens!($($a)*) << 1 };
}

pub(crate) use count_tokens;
