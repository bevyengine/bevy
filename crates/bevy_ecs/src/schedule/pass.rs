use alloc::{boxed::Box, vec::Vec};
use core::any::{Any, TypeId};

use super::{DiGraph, NodeId, ScheduleBuildError, ScheduleGraph};
use crate::{
    schedule::{SystemKey, SystemSetKey},
    world::World,
};
use bevy_utils::TypeIdMap;
use core::fmt::Debug;

/// A pass for modular modification of the dependency graph.
pub trait ScheduleBuildPass: Send + Sync + Debug + 'static {
    /// Custom options for dependencies between sets or systems.
    type EdgeOptions: 'static;

    /// Called when a dependency between sets or systems was explicitly added to the graph.
    fn add_dependency(&mut self, from: NodeId, to: NodeId, options: Option<&Self::EdgeOptions>);

    /// Called when mapping system sets to systems. Implementations may return an iterator of additional systems to be
    /// associated with the system set.
    fn map_set_to_systems(
        &mut self,
        _set: SystemSetKey,
        _world: &mut World,
        _graph: &mut ScheduleGraph,
    ) -> impl Iterator<Item = SystemKey> {
        std::iter::empty()
    }

    /// Called while flattening the dependency graph. For each `set`, this method is called
    /// with the `systems` associated with the set as well as an immutable reference to the current graph.
    /// Instead of modifying the graph directly, this method should return an iterator of edges to add to the graph.
    fn collapse_set(
        &mut self,
        _set: SystemSetKey,
        _systems: &[SystemKey],
        _world: &mut World,
        _graph: &mut ScheduleGraph,
        _dependency_flattening: &DiGraph<NodeId>,
    ) -> impl Iterator<Item = (NodeId, NodeId)> {
        std::iter::empty()
    }

    /// The implementation will be able to modify the `ScheduleGraph` here.
    fn build(
        &mut self,
        _world: &mut World,
        _graph: &mut ScheduleGraph,
        _dependency_flattened: &mut DiGraph<SystemKey>,
    ) -> Result<(), ScheduleBuildError> {
        Ok(())
    }
}

/// Object safe version of [`ScheduleBuildPass`].
pub(super) trait ScheduleBuildPassObj: Any + Send + Sync + Debug {
    fn build(
        &mut self,
        world: &mut World,
        graph: &mut ScheduleGraph,
        dependency_flattened: &mut DiGraph<SystemKey>,
    ) -> Result<(), ScheduleBuildError>;

    fn map_set_to_systems(
        &mut self,
        set: SystemSetKey,
        systems: &mut Vec<SystemKey>,
        world: &mut World,
        graph: &mut ScheduleGraph,
    );

    fn collapse_set(
        &mut self,
        set: SystemSetKey,
        systems: &[SystemKey],
        world: &mut World,
        graph: &mut ScheduleGraph,
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
        dependency_flattened: &mut DiGraph<SystemKey>,
    ) -> Result<(), ScheduleBuildError> {
        self.build(world, graph, dependency_flattened)
    }
    fn map_set_to_systems(
        &mut self,
        set: SystemSetKey,
        systems: &mut Vec<SystemKey>,
        world: &mut World,
        graph: &mut ScheduleGraph,
    ) {
        systems.extend(self.map_set_to_systems(set, world, graph));
    }
    fn collapse_set(
        &mut self,
        set: SystemSetKey,
        systems: &[SystemKey],
        world: &mut World,
        graph: &mut ScheduleGraph,
        dependency_flattening: &DiGraph<NodeId>,
        dependencies_to_add: &mut Vec<(NodeId, NodeId)>,
    ) {
        let iter = self.collapse_set(set, systems, world, graph, dependency_flattening);
        dependencies_to_add.extend(iter);
    }
    fn add_dependency(&mut self, from: NodeId, to: NodeId, all_options: &TypeIdMap<Box<dyn Any>>) {
        let option = all_options
            .get(&TypeId::of::<T::EdgeOptions>())
            .and_then(|x| x.downcast_ref::<T::EdgeOptions>());
        self.add_dependency(from, to, option);
    }
}
