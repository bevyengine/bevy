use crate::func::args::{Arg, ArgError, ArgInfo};

pub trait FromArg {
    type Item<'a>;
    fn from_arg<'a>(arg: Arg<'a>, info: &ArgInfo) -> Result<Self::Item<'a>, ArgError>;
}

// TODO: Move this into the `Reflect` derive
macro_rules! impl_from_arg {
    ($name: ty) => {
        impl $crate::func::args::FromArg for $name {
            type Item<'a> = $name;
            fn from_arg<'a>(
                arg: $crate::func::args::Arg<'a>,
                info: &$crate::func::args::ArgInfo,
            ) -> Result<Self::Item<'a>, $crate::func::args::ArgError> {
                match arg {
                    $crate::func::args::Arg::Owned(arg) => {
                        arg.take()
                            .map_err(|arg| $crate::func::args::ArgError::UnexpectedType {
                                id: info.id().clone(),
                                expected: ::std::borrow::Cow::Borrowed(info.type_path()),
                                received: ::std::borrow::Cow::Owned(arg.reflect_type_path().to_string()),
                            })
                    }
                    $crate::func::args::Arg::Ref(_) => {
                        Err($crate::func::args::ArgError::InvalidOwnership {
                            id: info.id().clone(),
                            expected: $crate::func::args::Ownership::Owned,
                            received: $crate::func::args::Ownership::Ref,
                        })
                    }
                    $crate::func::args::Arg::Mut(_) => {
                        Err($crate::func::args::ArgError::InvalidOwnership {
                            id: info.id().clone(),
                            expected: $crate::func::args::Ownership::Owned,
                            received: $crate::func::args::Ownership::Mut,
                        })
                    }
                }
            }
        }

        impl $crate::func::args::FromArg for &'static $name {
            type Item<'a> = &'a $name;
            fn from_arg<'a>(
                arg: $crate::func::args::Arg<'a>,
                info: &$crate::func::args::ArgInfo,
            ) -> Result<Self::Item<'a>, $crate::func::args::ArgError> {
                match arg {
                    $crate::func::args::Arg::Owned(_) => {
                        Err($crate::func::args::ArgError::InvalidOwnership {
                            id: info.id().clone(),
                            expected: $crate::func::args::Ownership::Ref,
                            received: $crate::func::args::Ownership::Owned,
                        })
                    }
                    $crate::func::args::Arg::Ref(arg) => {
                        Ok(arg.downcast_ref().ok_or_else(|| {
                            $crate::func::args::ArgError::UnexpectedType {
                                id: info.id().clone(),
                                expected: ::std::borrow::Cow::Borrowed(info.type_path()),
                                received: ::std::borrow::Cow::Owned(arg.reflect_type_path().to_string()),
                            }
                        })?)
                    }
                    $crate::func::args::Arg::Mut(_) => {
                        Err($crate::func::args::ArgError::InvalidOwnership {
                            id: info.id().clone(),
                            expected: $crate::func::args::Ownership::Ref,
                            received: $crate::func::args::Ownership::Mut,
                        })
                    }
                }
            }
        }

        impl $crate::func::args::FromArg for &'static mut $name {
            type Item<'a> = &'a mut $name;
            fn from_arg<'a>(
                arg: $crate::func::args::Arg<'a>,
                info: &$crate::func::args::ArgInfo,
            ) -> Result<Self::Item<'a>, $crate::func::args::ArgError> {
                match arg {
                    $crate::func::args::Arg::Owned(_) => {
                        Err($crate::func::args::ArgError::InvalidOwnership {
                            id: info.id().clone(),
                            expected: $crate::func::args::Ownership::Mut,
                            received: $crate::func::args::Ownership::Owned,
                        })
                    }
                    $crate::func::args::Arg::Ref(_) => {
                        Err($crate::func::args::ArgError::InvalidOwnership {
                            id: info.id().clone(),
                            expected: $crate::func::args::Ownership::Mut,
                            received: $crate::func::args::Ownership::Ref,
                        })
                    }
                    $crate::func::args::Arg::Mut(arg) => {
                        let received = ::std::borrow::Cow::Owned(arg.reflect_type_path().to_string());
                        Ok(arg.downcast_mut().ok_or_else(|| {
                            $crate::func::args::ArgError::UnexpectedType {
                                id: info.id().clone(),
                                expected: ::std::borrow::Cow::Borrowed(info.type_path()),
                                received,
                            }
                        })?)
                    }
                }
            }
        }
    };
}

pub(crate) use impl_from_arg;

impl_from_arg!(bool);
impl_from_arg!(char);
impl_from_arg!(f32);
impl_from_arg!(f64);
impl_from_arg!(i8);
impl_from_arg!(i16);
impl_from_arg!(i32);
impl_from_arg!(i64);
impl_from_arg!(i128);
impl_from_arg!(isize);
impl_from_arg!(u8);
impl_from_arg!(u16);
impl_from_arg!(u32);
impl_from_arg!(u64);
impl_from_arg!(u128);
impl_from_arg!(usize);
impl_from_arg!(String);
impl_from_arg!(&'static str);
