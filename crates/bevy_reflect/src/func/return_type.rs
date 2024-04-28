use crate::Reflect;

#[derive(Debug)]
pub enum Return<'a> {
    Unit,
    Owned(Box<dyn Reflect>),
    Ref(&'a dyn Reflect),
    Mut(&'a mut dyn Reflect),
}

impl<'a> Return<'a> {
    pub fn is_unit(&self) -> bool {
        matches!(self, Return::Unit)
    }

    pub fn unwrap_owned(self) -> Box<dyn Reflect> {
        match self {
            Return::Owned(value) => value,
            _ => panic!("expected owned value"),
        }
    }

    pub fn unwrap_ref(self) -> &'a dyn Reflect {
        match self {
            Return::Ref(value) => value,
            _ => panic!("expected reference value"),
        }
    }

    pub fn unwrap_mut(self) -> &'a mut dyn Reflect {
        match self {
            Return::Mut(value) => value,
            _ => panic!("expected mutable reference value"),
        }
    }
}

pub trait IntoReturn {
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
