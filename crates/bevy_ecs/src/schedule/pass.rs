use alloc::{boxed::Box, vec::Vec};
use core::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::Deref,
};

use bevy_platform::{collections::HashSet, hash::FixedHasher};
use bevy_utils::TypeIdMap;
use indexmap::IndexSet;

use super::{DiGraph, NodeId, ScheduleBuildError, ScheduleGraph};
use crate::{
    schedule::{
        graph::{Dag, DagAnalysis, DiGraphToposortError},
        SystemKey, SystemSetKey,
    },
    world::World,
};

/// A pass for modular modification of the dependency graph.
pub trait ScheduleBuildPass: Send + Sync + Debug + 'static {
    /// Custom options for dependencies between sets or systems.
    type EdgeOptions: 'static;

    /// Called when a dependency between sets or systems was explicitly added to the graph.
    fn add_dependency(&mut self, from: NodeId, to: NodeId, options: Option<&Self::EdgeOptions>);

    /// Called while flattening the dependency graph. For each `set`, this method is called
    /// with the `systems` associated with the set as well as an immutable reference to the current graph.
    /// Instead of modifying the graph directly, this method should return an iterator of edges to add to the graph.
    fn collapse_set(
        &mut self,
        set: SystemSetKey,
        systems: &IndexSet<SystemKey, FixedHasher>,
        dependency_flattening: &DiGraph<NodeId>,
    ) -> impl Iterator<Item = (NodeId, NodeId)>;

    /// The implementation will be able to modify the `ScheduleGraph` here.
    fn build(
        &mut self,
        world: &mut World,
        graph: &mut ScheduleGraph,
        dependency_flattened: FlattenedDependencies<'_>,
    ) -> Result<(), ScheduleBuildError>;
}

/// A wrapper around the directed, acyclic graph of system edges.
///
/// This allows tracking mutations to the graph for recording build pass changes.
pub struct FlattenedDependencies<'a> {
    /// The graph of dependency edges.
    pub(crate) dag: &'a mut Dag<SystemKey>,
    /// The edges that have been added by build passes.
    pub(crate) added_edges: &'a mut HashSet<(SystemKey, SystemKey)>,
}

impl Deref for FlattenedDependencies<'_> {
    type Target = Dag<SystemKey>;

    fn deref(&self) -> &Self::Target {
        self.dag
    }
}

impl FlattenedDependencies<'_> {
    /// Adds an edge to the dependencies such that `system_1` runs before `system_2`.
    pub fn add_edge(&mut self, system_1: SystemKey, system_2: SystemKey) {
        self.dag.add_edge(system_1, system_2);
        self.added_edges.insert((system_1, system_2));
    }

    /// Removes an edge going from `system_1` to `system_2` in the dependencies.
    ///
    /// This should be used with caution - removing edges this way can lead to **very** surprising
    /// behavior. However, this function can be used to remove dependencies that are made redundant
    /// by added edges.
    ///
    /// Note: these edges are **not** reported like the added edges are.
    pub fn remove_edge(&mut self, system_1: SystemKey, system_2: SystemKey) {
        self.dag.remove_edge(system_1, system_2);
        // We intentionally don't record edges (like `self.added_edges`) because it's unlikely that
        // users call this for anything other than redundant edges, and because these redundant
        // edges are actually important. It would be confusing if a visualizer omitted the removed
        // edges, since an edge you add in your plugin may not appear in the visualizer due to being
        // removed!
    }

    /// Returns a topological ordering of the graph, computing it if the graph is dirty.
    ///
    /// This function matches [`Dag::toposort`].
    pub fn toposort(&mut self) -> Result<&[SystemKey], DiGraphToposortError<SystemKey>> {
        self.dag.toposort()
    }

    /// Returns both the topological ordering and the underlying graph, computing the toposort if
    /// the graph is dirty.
    ///
    /// This function matches [`Dag::toposort_and_graph`].
    pub fn toposort_and_graph(
        &mut self,
    ) -> Result<(&[SystemKey], &DiGraph<SystemKey>), DiGraphToposortError<SystemKey>> {
        self.dag.toposort_and_graph()
    }

    /// Processes the DAG and computes various properties about it.
    ///
    /// This function matches [`Dag::analyze`].
    pub fn analyze(&mut self) -> Result<DagAnalysis<SystemKey>, DiGraphToposortError<SystemKey>> {
        self.dag.analyze()
    }
}

/// Object safe version of [`ScheduleBuildPass`].
pub(super) trait ScheduleBuildPassObj: Send + Sync + Debug {
    fn build(
        &mut self,
        world: &mut World,
        graph: &mut ScheduleGraph,
        dependency_flattened: FlattenedDependencies<'_>,
    ) -> Result<(), ScheduleBuildError>;

    fn collapse_set(
        &mut self,
        set: SystemSetKey,
        systems: &IndexSet<SystemKey, FixedHasher>,
        dependency_flattening: &DiGraph<NodeId>,
        dependencies_to_add: &mut Vec<(NodeId, NodeId)>,
    );
    fn add_dependency(&mut self, from: NodeId, to: NodeId, all_options: &TypeIdMap<Box<dyn Any>>);
}

impl<T: ScheduleBuildPass> ScheduleBuildPassObj for T {
    fn build(
        &mut self,
        world: &mut World,
        graph: &mut ScheduleGraph,
        dependency_flattened: FlattenedDependencies<'_>,
    ) -> Result<(), ScheduleBuildError> {
        self.build(world, graph, dependency_flattened)
    }
    fn collapse_set(
        &mut self,
        set: SystemSetKey,
        systems: &IndexSet<SystemKey, FixedHasher>,
        dependency_flattening: &DiGraph<NodeId>,
        dependencies_to_add: &mut Vec<(NodeId, NodeId)>,
    ) {
        let iter = self.collapse_set(set, systems, dependency_flattening);
        dependencies_to_add.extend(iter);
    }
    fn add_dependency(&mut self, from: NodeId, to: NodeId, all_options: &TypeIdMap<Box<dyn Any>>) {
        let option = all_options
            .get(&TypeId::of::<T::EdgeOptions>())
            .and_then(|x| x.downcast_ref::<T::EdgeOptions>());
        self.add_dependency(from, to, option);
    }
}
