use crate::component::ComponentCounters;
use std::ops::{Deref, DerefMut};

/// Unique borrow of an entity's component
pub struct Mut<'a, T> {
    pub(crate) value: &'a mut T,
    pub(crate) component_counters: &'a mut ComponentCounters,
    pub(crate) system_counter: u32,
    pub(crate) global_system_counter: u32,
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
        self.component_counters
            .set_changed(self.global_system_counter);
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
    pub fn is_added(&self) -> bool {
        self.component_counters
            .is_added(self.system_counter, self.global_system_counter)
    }

    /// Returns true if (and only if) this component been mutated since the start of the frame.
    pub fn is_mutated(&self) -> bool {
        self.component_counters
            .is_mutated(self.system_counter, self.global_system_counter)
    }

    /// Returns true if (and only if) this component been either mutated or added since the start of the frame.
    pub fn is_changed(&self) -> bool {
        self.component_counters
            .is_changed(self.system_counter, self.global_system_counter)
    }
}
