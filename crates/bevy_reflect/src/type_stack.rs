use crate::Type;
use core::fmt::{Debug, Formatter};
use core::slice::Iter;

/// Helper struct for managing a stack of [`Type`] instances.
///
/// This is useful for tracking the type hierarchy when serializing and deserializing types.
#[derive(Default, Clone)]
pub(crate) struct TypeStack {
    stack: Vec<Type>,
}

impl TypeStack {
    /// Create a new empty [`TypeStack`].
    pub const fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Push a new [`Type`] onto the stack.
    pub fn push(&mut self, ty: Type) {
        self.stack.push(ty);
    }

    /// Pop the last [`Type`] off the stack.
    pub fn pop(&mut self) {
        self.stack.pop();
    }

    /// Get an iterator over the stack in the order they were pushed.
    pub fn iter(&self) -> Iter<Type> {
        self.stack.iter()
    }
}

impl Debug for TypeStack {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut iter = self.iter();

        if let Some(first) = iter.next() {
            write!(f, "`{:?}`", first)?;
        }

        for ty in iter {
            write!(f, " -> `{:?}`", ty)?;
        }

        Ok(())
    }
}
