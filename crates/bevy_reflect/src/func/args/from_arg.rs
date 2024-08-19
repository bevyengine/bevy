use crate::func::args::{Arg, ArgError};

/// A trait for types that can be created from an [`Arg`].
///
/// This trait exists so that types can be automatically converted into an [`Arg`]
/// so they can be put into an [`ArgList`] and passed to a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// This trait is used instead of a blanket [`From`] implementation due to coherence issues:
/// we can't implement `From<T>` for both `T` and `&T`/`&mut T`.
///
/// This trait is automatically implemented when using the `Reflect` [derive macro].
///
/// [`ArgList`]: crate::func::args::ArgList
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
/// [derive macro]: derive@crate::Reflect
pub trait FromArg {
    /// The type to convert into.
    ///
    /// This should almost always be the same as `Self`, but with the lifetime `'a`.
    ///
    /// The reason we use a separate associated type is to allow for the lifetime
    /// to be tied to the argument, rather than the type itself.
    type This<'a>;

    /// Creates an item from an argument.
    ///
    /// The argument must be of the expected type and ownership.
    fn from_arg(arg: Arg) -> Result<Self::This<'_>, ArgError>;
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
            type This<'from_arg> = $ty;
            fn from_arg(arg: $crate::func::args::Arg) -> Result<Self::This<'_>, $crate::func::args::ArgError> {
                arg.take_owned()
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
            type This<'from_arg> = &'from_arg $ty;
            fn from_arg(arg: $crate::func::args::Arg) -> Result<Self::This<'_>, $crate::func::args::ArgError> {
                arg.take_ref()
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
            type This<'from_arg> = &'from_arg mut $ty;
            fn from_arg(arg: $crate::func::args::Arg) -> Result<Self::This<'_>, $crate::func::args::ArgError> {
                arg.take_mut()
            }
        }
    };
}

pub(crate) use impl_from_arg;
