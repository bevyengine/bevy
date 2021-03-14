use crate::component::ComponentFlags;
use std::ops::{Deref, DerefMut};

/// Unique borrow of an entity's component
pub struct Mut<'a, T> {
    pub(crate) value: &'a mut T,
    pub(crate) flags: &'a mut ComponentFlags,
}

impl<'a, T> Deref for Mut<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T> DerefMut for Mut<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.flags.insert(ComponentFlags::MUTATED);
        self.value
    }
}

impl<'a, T: core::fmt::Debug> core::fmt::Debug for Mut<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.value.fmt(f)
    }
}

impl<'w, T> Mut<'w, T> {
    /// Returns true if (and only if) this component been added since the start of the frame.
    pub fn added(&self) -> bool {
        self.flags.contains(ComponentFlags::ADDED)
    }

    /// Returns true if (and only if) this component been mutated since the start of the frame.
    pub fn mutated(&self) -> bool {
        self.flags.contains(ComponentFlags::MUTATED)
    }

    /// Returns true if (and only if) this component been either mutated or added since the start of
    /// the frame.
    pub fn changed(&self) -> bool {
        self.flags
            .intersects(ComponentFlags::ADDED | ComponentFlags::MUTATED)
    }
}
