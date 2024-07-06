use crate::func::args::{ArgError, Ownership};
use crate::{Reflect, TypePath};
use std::ops::Deref;

/// Represents an argument that can be passed to a [`DynamicFunction`], [`DynamicClosure`],
/// or [`DynamicClosureMut`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicClosure`]: crate::func::DynamicClosure
/// [`DynamicClosureMut`]: crate::func::DynamicClosureMut
#[derive(Debug)]
pub struct Arg<'a> {
    index: usize,
    value: ArgValue<'a>,
}

impl<'a> Arg<'a> {
    pub fn new(index: usize, value: ArgValue<'a>) -> Self {
        Self { index, value }
    }

    pub fn new_owned(index: usize, arg: impl Reflect) -> Self {
        Self {
            index,
            value: ArgValue::Owned(Box::new(arg)),
        }
    }

    pub fn new_ref(index: usize, arg: &'a dyn Reflect) -> Self {
        Self {
            index,
            value: ArgValue::Ref(arg),
        }
    }

    pub fn new_mut(index: usize, arg: &'a mut dyn Reflect) -> Self {
        Self {
            index,
            value: ArgValue::Mut(arg),
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn value(&self) -> &ArgValue<'a> {
        &self.value
    }

    pub fn take(self) -> ArgValue<'a> {
        self.value
    }

    /// Returns `Ok(T)` if the argument is [`ArgValue::Owned`].
    pub fn take_owned<T: Reflect + TypePath>(self) -> Result<T, ArgError> {
        match self.value {
            ArgValue::Owned(arg) => arg.take().map_err(|arg| ArgError::UnexpectedType {
                index: self.index,
                expected: std::borrow::Cow::Borrowed(T::type_path()),
                received: std::borrow::Cow::Owned(arg.reflect_type_path().to_string()),
            }),
            ArgValue::Ref(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Owned,
                received: Ownership::Ref,
            }),
            ArgValue::Mut(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Owned,
                received: Ownership::Mut,
            }),
        }
    }

    /// Returns `Ok(&T)` if the argument is [`ArgValue::Ref`].
    pub fn take_ref<T: Reflect + TypePath>(self) -> Result<&'a T, ArgError> {
        match self.value {
            ArgValue::Owned(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Ref,
                received: Ownership::Owned,
            }),
            ArgValue::Ref(arg) => {
                Ok(arg.downcast_ref().ok_or_else(|| ArgError::UnexpectedType {
                    index: self.index,
                    expected: std::borrow::Cow::Borrowed(T::type_path()),
                    received: std::borrow::Cow::Owned(arg.reflect_type_path().to_string()),
                })?)
            }
            ArgValue::Mut(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Ref,
                received: Ownership::Mut,
            }),
        }
    }

    /// Returns `Ok(&mut T)` if the argument is [`ArgValue::Mut`].
    pub fn take_mut<T: Reflect + TypePath>(self) -> Result<&'a mut T, ArgError> {
        match self.value {
            ArgValue::Owned(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Mut,
                received: Ownership::Owned,
            }),
            ArgValue::Ref(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Mut,
                received: Ownership::Ref,
            }),
            ArgValue::Mut(arg) => {
                let received = std::borrow::Cow::Owned(arg.reflect_type_path().to_string());
                Ok(arg.downcast_mut().ok_or_else(|| ArgError::UnexpectedType {
                    index: self.index,
                    expected: std::borrow::Cow::Borrowed(T::type_path()),
                    received,
                })?)
            }
        }
    }
}

/// Represents an argument that can be passed to a [`DynamicFunction`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
#[derive(Debug)]
pub enum ArgValue<'a> {
    Owned(Box<dyn Reflect>),
    Ref(&'a dyn Reflect),
    Mut(&'a mut dyn Reflect),
}

impl<'a> Deref for ArgValue<'a> {
    type Target = dyn Reflect;

    fn deref(&self) -> &Self::Target {
        match self {
            ArgValue::Owned(arg) => arg.as_ref(),
            ArgValue::Ref(arg) => *arg,
            ArgValue::Mut(arg) => *arg,
        }
    }
}
