use crate::func::args::{ArgError, ArgInfo, Ownership};
use crate::Reflect;

pub enum Arg<'a> {
    Owned(Box<dyn Reflect>),
    Ref(&'a dyn Reflect),
    Mut(&'a mut dyn Reflect),
}

impl<'a> Arg<'a> {
    pub fn take_owned<T: Reflect>(self, info: &ArgInfo) -> Result<T, ArgError> {
        match self {
            Arg::Owned(arg) => arg.take().map_err(|arg| ArgError::UnexpectedType {
                id: info.id().clone(),
                expected: ::std::borrow::Cow::Borrowed(info.type_path()),
                received: ::std::borrow::Cow::Owned(arg.reflect_type_path().to_string()),
            }),
            Arg::Ref(_) => Err(ArgError::InvalidOwnership {
                id: info.id().clone(),
                expected: Ownership::Owned,
                received: Ownership::Ref,
            }),
            Arg::Mut(_) => Err(ArgError::InvalidOwnership {
                id: info.id().clone(),
                expected: Ownership::Owned,
                received: Ownership::Mut,
            }),
        }
    }
    pub fn take_ref<T: Reflect>(self, info: &ArgInfo) -> Result<&'a T, ArgError> {
        match self {
            Arg::Owned(_) => Err(ArgError::InvalidOwnership {
                id: info.id().clone(),
                expected: Ownership::Ref,
                received: Ownership::Owned,
            }),
            Arg::Ref(arg) => Ok(arg.downcast_ref().ok_or_else(|| ArgError::UnexpectedType {
                id: info.id().clone(),
                expected: ::std::borrow::Cow::Borrowed(info.type_path()),
                received: ::std::borrow::Cow::Owned(arg.reflect_type_path().to_string()),
            })?),
            Arg::Mut(_) => Err(ArgError::InvalidOwnership {
                id: info.id().clone(),
                expected: Ownership::Ref,
                received: Ownership::Mut,
            }),
        }
    }

    pub fn take_mut<T: Reflect>(self, info: &ArgInfo) -> Result<&'a mut T, ArgError> {
        match self {
            Arg::Owned(_) => Err(ArgError::InvalidOwnership {
                id: info.id().clone(),
                expected: Ownership::Mut,
                received: Ownership::Owned,
            }),
            Arg::Ref(_) => Err(ArgError::InvalidOwnership {
                id: info.id().clone(),
                expected: Ownership::Mut,
                received: Ownership::Ref,
            }),
            Arg::Mut(arg) => {
                let received = ::std::borrow::Cow::Owned(arg.reflect_type_path().to_string());
                Ok(arg.downcast_mut().ok_or_else(|| ArgError::UnexpectedType {
                    id: info.id().clone(),
                    expected: ::std::borrow::Cow::Borrowed(info.type_path()),
                    received,
                })?)
            }
        }
    }
}
