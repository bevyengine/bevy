use crate::PartialReflect;
use alloc::boxed::Box;

/// The return type of a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
#[derive(Debug)]
pub enum Return<'a> {
    /// The function returns an owned value.
    ///
    /// This includes functions that return nothing (i.e. they return `()`).
    Owned(Box<dyn PartialReflect>),
    /// The function returns a reference to a value.
    Ref(&'a dyn PartialReflect),
    /// The function returns a mutable reference to a value.
    Mut(&'a mut dyn PartialReflect),
}

impl<'a> Return<'a> {
    /// Creates an [`Owned`](Self::Owned) unit (`()`) type.
    pub fn unit() -> Self {
        Self::Owned(Box::new(()))
    }

    /// Returns `true` if the return value is an [`Owned`](Self::Owned) unit (`()`) type.
    pub fn is_unit(&self) -> bool {
        match self {
            Return::Owned(val) => val.represents::<()>(),
            _ => false,
        }
    }

    /// Unwraps the return value as an owned value.
    ///
    /// # Panics
    ///
    /// Panics if the return value is not [`Self::Owned`].
    pub fn unwrap_owned(self) -> Box<dyn PartialReflect> {
        match self {
            Return::Owned(value) => value,
            _ => panic!("expected owned value"),
        }
    }

    /// Unwraps the return value as a reference to a value.
    ///
    /// # Panics
    ///
    /// Panics if the return value is not [`Self::Ref`].
    pub fn unwrap_ref(self) -> &'a dyn PartialReflect {
        match self {
            Return::Ref(value) => value,
            _ => panic!("expected reference value"),
        }
    }

    /// Unwraps the return value as a mutable reference to a value.
    ///
    /// # Panics
    ///
    /// Panics if the return value is not [`Self::Mut`].
    pub fn unwrap_mut(self) -> &'a mut dyn PartialReflect {
        match self {
            Return::Mut(value) => value,
            _ => panic!("expected mutable reference value"),
        }
    }
}

/// A trait for types that can be converted into a [`Return`] value.
///
/// This trait exists so that types can be automatically converted into a [`Return`]
/// by [`ReflectFn`] and [`ReflectFnMut`].
///
/// This trait is used instead of a blanket [`Into`] implementation due to coherence issues:
/// we can't implement `Into<Return>` for both `T` and `&T`/`&mut T`.
///
/// This trait is automatically implemented when using the `Reflect` [derive macro].
///
/// [`ReflectFn`]: crate::func::ReflectFn
/// [`ReflectFnMut`]: crate::func::ReflectFnMut
/// [derive macro]: derive@crate::Reflect
pub trait IntoReturn {
    /// Converts [`Self`] into a [`Return`] value.
    fn into_return<'a>(self) -> Return<'a>
    where
        Self: 'a;
}

impl IntoReturn for () {
    fn into_return<'a>(self) -> Return<'a> {
        Return::unit()
    }
}

/// Implements the [`IntoReturn`] trait for the given type.
///
/// This will implement it for `ty`, `&ty`, and `&mut ty`.
///
/// See [`impl_function_traits`] for details on syntax.
///
/// [`impl_function_traits`]: crate::func::macros::impl_function_traits
macro_rules! impl_into_return {
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
        > $crate::func::IntoReturn for $ty
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            fn into_return<'into_return>(self) -> $crate::func::Return<'into_return> where Self: 'into_return {
                $crate::func::Return::Owned(Box::new(self))
            }
        }

        impl <
            $($($T $(: $T1 $(+ $T2)*)?),*)?
            $(, $(const $N : $size),*)?
        > $crate::func::IntoReturn for &'static $ty
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            fn into_return<'into_return>(self) -> $crate::func::Return<'into_return> where Self: 'into_return {
                $crate::func::Return::Ref(self)
            }
        }

        impl <
            $($($T $(: $T1 $(+ $T2)*)?),*)?
            $(, $(const $N : $size),*)?
        > $crate::func::IntoReturn for &'static mut $ty
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            fn into_return<'into_return>(self) -> $crate::func::Return<'into_return> where Self: 'into_return {
                $crate::func::Return::Mut(self)
            }
        }
    };
}

pub(crate) use impl_into_return;
