use crate::Reflect;

/// The return type of a [`Function`].
///
/// [`Function`]: crate::func::Function
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

// TODO: Move this into the `Reflect` derive
macro_rules! impl_into_return {
    ($name: ty) => {
        impl IntoReturn for $name {
            fn into_return<'a>(self) -> Return<'a> {
                Return::Owned(Box::new(self))
            }
        }

        impl IntoReturn for &'static $name {
            fn into_return<'a>(self) -> Return<'a> {
                Return::Ref(self)
            }
        }

        impl IntoReturn for &'static mut $name {
            fn into_return<'a>(self) -> Return<'a> {
                Return::Mut(self)
            }
        }
    };
}

pub(crate) use impl_into_return;

impl_into_return!(bool);
impl_into_return!(char);
impl_into_return!(f32);
impl_into_return!(f64);
impl_into_return!(i8);
impl_into_return!(i16);
impl_into_return!(i32);
impl_into_return!(i64);
impl_into_return!(i128);
impl_into_return!(isize);
impl_into_return!(u8);
impl_into_return!(u16);
impl_into_return!(u32);
impl_into_return!(u64);
impl_into_return!(u128);
impl_into_return!(usize);
impl_into_return!(String);
impl_into_return!(&'static str);
