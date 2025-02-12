use alloc::collections::BTreeMap;
use core::hash::{BuildHasher, Hash};

use bevy_platform_support::collections::HashMap;

use crate::{component::Immutable, prelude::Component};

/// A storage backend for indexing the [`Component`] `C`.
///
/// Typically, you would use a [`HashMap`] for this purpose, but you may be able to further optimize
/// the indexing performance of a particular component with a custom implementation of this backend.
///
/// For example, the component `C` may be uniquely identifiable with a subset of its data, or implement
/// traits like [`Hash`] or [`Ord`] which would allow for specialized storage options.
pub trait IndexStorage<C: Component<Mutability = Immutable>>: 'static + Send + Sync {
    /// Get the identifier of the provided value for `C`.
    ///
    /// Returns [`None`] if no identifier for the given value has been provided.
    fn get(&self, value: &C) -> Option<usize>;
    /// Sets the identifier of the provided value for `C`.
    fn insert(&mut self, value: &C, index: usize);
    /// Removes the identifier of the provided value for `C`.
    fn remove(&mut self, value: &C);
}

impl<C, S> IndexStorage<C> for HashMap<C, usize, S>
where
    C: Component<Mutability = Immutable> + Eq + Hash + Clone,
    S: BuildHasher + Send + Sync + 'static,
{
    fn get(&self, value: &C) -> Option<usize> {
        self.get(value).copied()
    }

    fn insert(&mut self, value: &C, index: usize) {
        self.insert(C::clone(value), index);
    }

    fn remove(&mut self, value: &C) {
        self.remove(value);
    }
}

impl<C> IndexStorage<C> for BTreeMap<C, usize>
where
    C: Component<Mutability = Immutable> + Ord + Clone,
{
    fn get(&self, value: &C) -> Option<usize> {
        self.get(value).copied()
    }

    fn insert(&mut self, value: &C, index: usize) {
        self.insert(C::clone(value), index);
    }

    fn remove(&mut self, value: &C) {
        self.remove(value);
    }
}
