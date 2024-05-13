use std::hash::Hash;
use std::{borrow::Borrow, ptr};

//essentially a version of Cow that doesn't require Clone and uses reference equality for the
//borrowed case
pub enum RefEq<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T> RefEq<'a, T> {
    pub fn reborrow(&'a self) -> Self {
        match self {
            Self::Borrowed(r) => Self::Borrowed(r),
            Self::Owned(t) => Self::Borrowed(t),
        }
    }
}

impl<'a, T: PartialEq> PartialEq for RefEq<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Borrowed(l), Self::Borrowed(r)) => ptr::eq(*l, *r),
            (Self::Owned(l), Self::Owned(r)) => l == r,
            _ => false,
        }
    }
}

impl<'a, T: Eq> Eq for RefEq<'a, T> {}

impl<'a, T: Hash> Hash for RefEq<'a, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            RefEq::Borrowed(r) => (*r as *const T).hash(state),
            RefEq::Owned(t) => t.hash(state),
        }
    }
}

impl<T> Borrow<T> for RefEq<'_, T> {
    fn borrow(&self) -> &T {
        match self {
            RefEq::Borrowed(r) => r,
            RefEq::Owned(t) => t,
        }
    }
}

impl<T: Clone> Clone for RefEq<'_, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(r) => Self::Borrowed(*r),
            Self::Owned(t) => Self::Owned(t.clone()),
        }
    }
}

impl<'a, T> From<T> for RefEq<'a, T> {
    fn from(value: T) -> Self {
        Self::Owned(value)
    }
}

impl<'a, T> From<&'a T> for RefEq<'a, T> {
    fn from(value: &'a T) -> Self {
        Self::Borrowed(value)
    }
}
