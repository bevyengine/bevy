use core::fmt::{Display, Formatter};

pub trait GetOwnership {
    fn ownership() -> Ownership;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Ownership {
    Ref,
    Mut,
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

// TODO: Move this into the `Reflect` derive
macro_rules! impl_get_ownership {
    ($name: ty) => {
        impl $crate::func::args::GetOwnership for $name {
            fn ownership() -> $crate::func::args::Ownership {
                $crate::func::args::Ownership::Owned
            }
        }

        impl<'a> $crate::func::args::GetOwnership for &'a $name {
            fn ownership() -> $crate::func::args::Ownership {
                $crate::func::args::Ownership::Ref
            }
        }

        impl<'a> $crate::func::args::GetOwnership for &'a mut $name {
            fn ownership() -> $crate::func::args::Ownership {
                $crate::func::args::Ownership::Mut
            }
        }
    };
}

pub(crate) use impl_get_ownership;

impl_get_ownership!(());
impl_get_ownership!(bool);
impl_get_ownership!(char);
impl_get_ownership!(f32);
impl_get_ownership!(f64);
impl_get_ownership!(i8);
impl_get_ownership!(i16);
impl_get_ownership!(i32);
impl_get_ownership!(i64);
impl_get_ownership!(i128);
impl_get_ownership!(isize);
impl_get_ownership!(u8);
impl_get_ownership!(u16);
impl_get_ownership!(u32);
impl_get_ownership!(u64);
impl_get_ownership!(u128);
impl_get_ownership!(usize);
impl_get_ownership!(String);
impl_get_ownership!(&'static str);
