use core::fmt::{Display, Formatter};

/// A trait for getting the ownership of a type.
///
/// This trait exists so that [`TypedFunction`] can automatically generate
/// [`FunctionInfo`] containing the proper [`Ownership`] for its [argument] types.
///
/// This trait is automatically implemented when using the `Reflect` [derive macro].
///
/// [`TypedFunction`]: crate::func::TypedFunction
/// [`FunctionInfo`]: crate::func::FunctionInfo
/// [argument]: crate::func::args::Arg
/// [derive macro]: derive@crate::Reflect
pub trait GetOwnership {
    /// Returns the ownership of [`Self`].
    fn ownership() -> Ownership;
}

/// The ownership of a type.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Ownership {
    /// The type is a reference (i.e. `&T`).
    Ref,
    /// The type is a mutable reference (i.e. `&mut T`).
    Mut,
    /// The type is owned (i.e. `T`).
    Owned,
}

impl Display for Ownership {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Ref => write!(f, "reference"),
            Self::Mut => write!(f, "mutable reference"),
            Self::Owned => write!(f, "owned"),
        }
    }
}

/// Implements the [`GetOwnership`] trait for the given type.
///
/// This will implement it for `$ty`, `&$ty`, and `&mut $ty`.
///
/// See [`impl_function_traits`] for details on syntax.
///
/// [`impl_function_traits`]: crate::func::macros::impl_function_traits
macro_rules! impl_get_ownership {
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
        > $crate::func::args::GetOwnership for $ty
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            fn ownership() -> $crate::func::args::Ownership {
                $crate::func::args::Ownership::Owned
            }
        }

        impl <
            $($($T $(: $T1 $(+ $T2)*)?),*)?
            $(, $(const $N : $size),*)?
        > $crate::func::args::GetOwnership for &'_ $ty
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            fn ownership() -> $crate::func::args::Ownership {
                $crate::func::args::Ownership::Ref
            }
        }

        impl <
            $($($T $(: $T1 $(+ $T2)*)?),*)?
            $(, $(const $N : $size),*)?
        > $crate::func::args::GetOwnership for &'_ mut $ty
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            fn ownership() -> $crate::func::args::Ownership {
                $crate::func::args::Ownership::Mut
            }
        }
    };
}

pub(crate) use impl_get_ownership;
