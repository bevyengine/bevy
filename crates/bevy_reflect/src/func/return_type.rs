use crate::Reflect;

/// The return type of a [`DynamicFunction`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
#[derive(Debug)]
pub enum Return<'a> {
    /// The function returns nothing (i.e. it returns `()`).
    Unit,
    /// The function returns an owned value.
    Owned(Box<dyn Reflect>),
    /// The function returns a reference to a value.
    Ref(&'a dyn Reflect),
    /// The function returns a mutable reference to a value.
    Mut(&'a mut dyn Reflect),
}

impl<'a> Return<'a> {
    /// Returns `true` if the return value is [`Self::Unit`].
    pub fn is_unit(&self) -> bool {
        matches!(self, Return::Unit)
    }

    /// Unwraps the return value as an owned value.
    ///
    /// # Panics
    ///
    /// Panics if the return value is not [`Self::Owned`].
    pub fn unwrap_owned(self) -> Box<dyn Reflect> {
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
    pub fn unwrap_ref(self) -> &'a dyn Reflect {
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
    pub fn unwrap_mut(self) -> &'a mut dyn Reflect {
        match self {
            Return::Mut(value) => value,
            _ => panic!("expected mutable reference value"),
        }
    }
}

/// A trait for types that can be converted into a [`Return`] value.
pub trait IntoReturn {
    /// Converts [`Self`] into a [`Return`] value.
    fn into_return<'a>(self) -> Return<'a>;
}

impl IntoReturn for () {
    fn into_return<'a>(self) -> Return<'a> {
        Return::Unit
    }
}

macro_rules! impl_into_return {
    (
        $name: ty
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
        > $crate::func::IntoReturn for $name
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            fn into_return<'into_return>(self) -> $crate::func::Return<'into_return> {
                $crate::func::Return::Owned(Box::new(self))
            }
        }

        impl <
            $($($T $(: $T1 $(+ $T2)*)?),*)?
            $(, $(const $N : $size),*)?
        > $crate::func::IntoReturn for &'static $name
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            fn into_return<'into_return>(self) -> $crate::func::Return<'into_return> {
                $crate::func::Return::Ref(self)
            }
        }

        impl <
            $($($T $(: $T1 $(+ $T2)*)?),*)?
            $(, $(const $N : $size),*)?
        > $crate::func::IntoReturn for &'static mut $name
        $(
            where
                $($U $(: $U1 $(+ $U2)*)?),*
        )?
        {
            fn into_return<'into_return>(self) -> $crate::func::Return<'into_return> {
                $crate::func::Return::Mut(self)
            }
        }
    };
}

pub(crate) use impl_into_return;
