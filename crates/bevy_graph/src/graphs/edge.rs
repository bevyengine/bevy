use std::ops::{Deref, DerefMut};

use super::keys::NodeIdx;

/// An edge between nodes that store data of type `E`.
#[derive(Clone)]
pub struct Edge<E>(pub NodeIdx, pub NodeIdx, pub E);

impl<E> Edge<E> {
    /// Returns a [`EdgeRef`] of this edge
    #[inline]
    pub const fn as_ref_edge(&self) -> EdgeRef<E> {
        EdgeRef(self.0, self.1, &self.2)
    }

    /// Returns a [`EdgeMut`] of this edge
    #[inline]
    pub fn as_mut_edge(&mut self) -> EdgeMut<E> {
        EdgeMut(self.0, self.1, &mut self.2)
    }
}

/// An util container which holds Edge<E> data with an immutable reference to the edge value
pub struct EdgeRef<'v, E>(pub NodeIdx, pub NodeIdx, pub &'v E);

impl<'v, E> Deref for EdgeRef<'v, E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.2
    }
}

/// An util container which holds Edge<E> data with a mutable reference to the edge value
pub struct EdgeMut<'v, E>(pub NodeIdx, pub NodeIdx, pub &'v mut E);

impl<'v, E> Deref for EdgeMut<'v, E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.2
    }
}

impl<'v, E> DerefMut for EdgeMut<'v, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.2
    }
}
