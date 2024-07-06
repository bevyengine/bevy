use crate::func::args::{Arg, ArgError, ArgInfo};

/// A trait for types that can be created from an [`Arg`].
///
/// This trait is used instead of a blanket [`From`] implementation due to coherence issues:
/// we can't implement `From<T>` for both `T` and `&T`/`&mut T`.
///
/// This trait is automatically implemented when using the `Reflect` [derive macro].
///
/// [derive macro]: derive@crate::Reflect
pub trait FromArg {
    /// The type of the item created from the argument.
    ///
    /// This should almost always be the same as `Self`, but with the lifetime `'a`.
    type Item<'a>;

    /// Creates an item from an argument.
    ///
    /// The argument must be of the expected type and ownership.
    fn from_arg<'a>(arg: Arg<'a>, info: &ArgInfo) -> Result<Self::Item<'a>, ArgError>;
}

/// Implements the [`FromArg`] trait for the given type.
///
/// This will implement it for `$ty`, `&$ty`, and `&mut $ty`.
///
/// See [`impl_function_traits`] for details on syntax.
///
/// [`impl_function_traits`]: crate::func::macros::impl_function_traits
macro_rules! impl_from_arg {
    (
        $ty: ty
        $(;
            <
                $($T: ident $(: $T1: tt $(+ $T2: tt)*)?),*
            >
        )?
        $(
            [
                $(const $N: ident : $size: ident),*
            ]
        )?
        $(
            where
                $($U: ty $(: $U1: tt $(+ $U2: tt)*)?),*
        )?
    ) => {
        impl <
            $($($T $(: $T1 $(+ $T2)*)?),*)?
            $(, $(const $N : $size),*)?
        > $crate::func::args::FromArg for $ty
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            type Item<'from_arg> = $ty;
            fn from_arg<'from_arg>(
                arg: $crate::func::args::Arg<'from_arg>,
                info: &$crate::func::args::ArgInfo,
            ) -> Result<Self::Item<'from_arg>, $crate::func::args::ArgError> {
                arg.take_owned(info)
            }
        }

        impl <
            $($($T $(: $T1 $(+ $T2)*)?),*)?
            $(, $(const $N : $size),*)?
        > $crate::func::args::FromArg for &'static $ty
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            type Item<'from_arg> = &'from_arg $ty;
            fn from_arg<'from_arg>(
                arg: $crate::func::args::Arg<'from_arg>,
                info: &$crate::func::args::ArgInfo,
            ) -> Result<Self::Item<'from_arg>, $crate::func::args::ArgError> {
                arg.take_ref(info)
            }
        }

        impl <
            $($($T $(: $T1 $(+ $T2)*)?),*)?
            $(, $(const $N : $size),*)?
        > $crate::func::args::FromArg for &'static mut $ty
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            type Item<'from_arg> = &'from_arg mut $ty;
            fn from_arg<'from_arg>(
                arg: $crate::func::args::Arg<'from_arg>,
                info: &$crate::func::args::ArgInfo,
            ) -> Result<Self::Item<'from_arg>, $crate::func::args::ArgError> {
                arg.take_mut(info)
            }
        }
    };
}

pub(crate) use impl_from_arg;
