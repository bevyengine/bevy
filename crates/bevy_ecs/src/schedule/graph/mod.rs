use alloc::vec::Vec;
use core::fmt::Debug;

use crate::schedule::InternedSystemSet;

mod dag;
mod graph_map;
mod tarjan_scc;

pub use dag::*;
pub use graph_map::{DiGraph, DiGraphToposortError, Direction, GraphNodeId, UnGraph};

/// Specifies what kind of edge should be added to the dependency graph.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub(crate) enum DependencyKind {
    /// A node that should be preceded.
    Before,
    /// A node that should be succeeded.
    After,
}

/// An edge to be added to the dependency graph.
pub(crate) struct Dependency {
    pub(crate) kind: DependencyKind,
    pub(crate) set: InternedSystemSet,
    pub(crate) is_weak: bool,
    pub(crate) ignore_deferred: bool,
}

impl Dependency {
    pub fn new(kind: DependencyKind, set: InternedSystemSet) -> Self {
        Self {
            kind,
            set,
            is_weak: false,
            ignore_deferred: false,
        }
    }

    /// Marks the dependency as weak.
    /// A weak dependency allows systems to run in parallel if they do not conflict.
    pub fn set_weak(mut self) -> Self {
        self.is_weak = true;
        self
    }

    /// Marks the dependency to ignore deferred commands between systems.
    /// This tells the [`AutoInsertApplyDeferredPass`] to ignore this dependency when considering sync points.
    ///
    /// [`AutoInsertApplyDeferredPass`]: crate::schedule::passes::AutoInsertApplyDeferredPass
    pub fn ignore_deferred(mut self) -> Self {
        self.ignore_deferred = true;
        self
    }
}

/// Configures ambiguity detection for a single system.
#[derive(Clone, Debug, Default)]
pub(crate) enum Ambiguity {
    #[default]
    Check,
    /// Ignore warnings with systems in any of these system sets. May contain duplicates.
    IgnoreWithSet(Vec<InternedSystemSet>),
    /// Ignore all warnings.
    IgnoreAll,
}

/// Metadata about how the node fits in the schedule graph
#[derive(Default)]
pub struct GraphInfo {
    /// the sets that the node belongs to (hierarchy)
    pub(crate) hierarchy: Vec<InternedSystemSet>,
    /// the sets that the node depends on (must run before or after)
    pub(crate) dependencies: Vec<Dependency>,
    pub(crate) ambiguous_with: Ambiguity,
}

/// Converts 2D row-major pair of indices into a 1D array index.
pub(crate) fn index(row: usize, col: usize, num_cols: usize) -> usize {
    debug_assert!(col < num_cols);
    (row * num_cols) + col
}

/// Converts a 1D array index into a 2D row-major pair of indices.
pub(crate) fn row_col(index: usize, num_cols: usize) -> (usize, usize) {
    (index / num_cols, index % num_cols)
}
