use std::any::TypeId;

use crate::{Domain, Time};

/// Generic [`Time`] [`Domain`] representing the current time context.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct Generic {
    /// The domain promenance (i.e. which domain this generic one was created from).
    provenance: Option<TypeId>,
}

impl Generic {
    /// Constructs a new `Generic` based on the given `T` domain.
    pub fn new_from<T: Domain>() -> Self {
        Self {
            provenance: Some(TypeId::of::<T>()),
        }
    }

    /// Returns the provenance of this generic domain
    /// (i.e. which domain this generic one was created from).
    pub fn provenance(&self) -> Option<TypeId> {
        self.provenance
    }
}

impl Time<Generic> {
    /// Constructs a new `Time<Generic>` based on the given `T` domain.
    pub fn new_from<T: Domain>() -> Self {
        Self::new_with(Generic::new_from::<T>())
    }
}

impl Domain for Generic {}
