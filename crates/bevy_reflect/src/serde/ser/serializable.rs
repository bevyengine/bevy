use alloc::boxed::Box;
use core::ops::Deref;

/// A type-erased serializable value.
pub enum Serializable<'a> {
    /// TODO
    Owned(Box<dyn erased_serde::Serialize + 'a>),
    /// TODO
    Borrowed(&'a dyn erased_serde::Serialize),
}

impl<'a> Deref for Serializable<'a> {
    type Target = dyn erased_serde::Serialize + 'a;

    fn deref(&self) -> &Self::Target {
        match self {
            Serializable::Borrowed(serialize) => serialize,
            Serializable::Owned(serialize) => serialize,
        }
    }
}
