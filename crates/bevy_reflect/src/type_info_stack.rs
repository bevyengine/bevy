use crate::TypeInfo;
use alloc::vec::Vec;
use core::{
    fmt::{Debug, Formatter},
    slice::Iter,
};

/// Helper struct for managing a stack of [`TypeInfo`] instances.
///
/// This is useful for tracking the type hierarchy when serializing and deserializing types.
#[derive(Default, Clone)]
pub(crate) struct TypeInfoStack {
    stack: Vec<&'static TypeInfo>,
}

impl TypeInfoStack {
    /// Create a new empty [`TypeInfoStack`].
    pub const fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Push a new [`TypeInfo`] onto the stack.
    pub fn push(&mut self, type_info: &'static TypeInfo) {
        self.stack.push(type_info);
    }

    /// Pop the last [`TypeInfo`] off the stack.
    pub fn pop(&mut self) {
        self.stack.pop();
    }

    /// Get an iterator over the stack in the order they were pushed.
    pub fn iter(&self) -> Iter<'_, &'static TypeInfo> {
        self.stack.iter()
    }
}

impl Debug for TypeInfoStack {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut iter = self.iter();

        if let Some(first) = iter.next() {
            write!(f, "`{}`", first.type_path())?;
        }

        for info in iter {
            write!(f, " -> `{}`", info.type_path())?;
        }

        Ok(())
    }
}
