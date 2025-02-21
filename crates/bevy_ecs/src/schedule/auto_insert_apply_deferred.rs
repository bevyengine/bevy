use alloc::{boxed::Box, collections::BTreeSet, vec::Vec};

use bevy_platform_support::collections::HashMap;

use crate::system::IntoSystem;
use crate::world::World;

use super::{
    is_apply_deferred, ApplyDeferred, DiGraph, Direction, NodeId, ReportCycles, ScheduleBuildError,
    ScheduleBuildPass, ScheduleGraph, SystemNode,
};

/// A [`ScheduleBuildPass`] that inserts [`ApplyDeferred`] systems into the schedule graph
/// when there are [`Deferred`](crate::prelude::Deferred)
/// in one system and there are ordering dependencies on that system. [`Commands`](crate::system::Commands) is one
/// such deferred buffer.
///
/// This pass is typically automatically added to the schedule. You can disable this by setting
/// [`ScheduleBuildSettings::auto_insert_apply_deferred`](crate::schedule::ScheduleBuildSettings::auto_insert_apply_deferred)
/// to `false`. You may want to disable this if you only want to sync deferred params at the end of the schedule,
/// or want to manually insert all your sync points.
#[derive(Debug, Default)]
pub struct AutoInsertApplyDeferredPass {
    /// Dependency edges that will **not** automatically insert an instance of `ApplyDeferred` on the edge.
    no_sync_edges: BTreeSet<(NodeId, NodeId)>,
    auto_sync_node_ids: HashMap<u32, NodeId>,
}

/// If added to a dependency edge, the edge will not be considered for auto sync point insertions.
pub struct IgnoreDeferred;

impl AutoInsertApplyDeferredPass {
    /// Returns the `NodeId` of the cached auto sync point. Will create
    /// a new one if needed.
    fn get_sync_point(&mut self, graph: &mut ScheduleGraph, distance: u32) -> NodeId {
        self.auto_sync_node_ids
            .get(&distance)
            .copied()
            .or_else(|| {
                let node_id = self.add_auto_sync(graph);
                self.auto_sync_node_ids.insert(distance, node_id);
                Some(node_id)
            })
            .unwrap()
    }
    /// add an [`ApplyDeferred`] system with no config
    fn add_auto_sync(&mut self, graph: &mut ScheduleGraph) -> NodeId {
        let id = NodeId::System(graph.systems.len());

        graph
            .systems
            .push(SystemNode::new(Box::new(IntoSystem::into_system(
                ApplyDeferred,
            ))));
        graph.system_conditions.push(Vec::new());

        // ignore ambiguities with auto sync points
        // They aren't under user control, so no one should know or care.
        graph.ambiguous_with_all.insert(id);

        id
    }
}

impl ScheduleBuildPass for AutoInsertApplyDeferredPass {
    type EdgeOptions = IgnoreDeferred;

    fn add_dependency(&mut self, from: NodeId, to: NodeId, options: Option<&Self::EdgeOptions>) {
        if options.is_some() {
            self.no_sync_edges.insert((from, to));
        }
    }

    fn build(
        &mut self,
        _world: &mut World,
        graph: &mut ScheduleGraph,
        dependency_flattened: &mut DiGraph,
    ) -> Result<(), ScheduleBuildError> {
        let mut sync_point_graph = dependency_flattened.clone();
        let topo = graph.topsort_graph(dependency_flattened, ReportCycles::Dependency)?;

        // calculate the number of sync points each sync point is from the beginning of the graph
        // use the same sync point if the distance is the same
        let mut distances: HashMap<usize, Option<u32>> =
            HashMap::with_capacity_and_hasher(topo.len(), Default::default());
        for node in &topo {
            let add_sync_after = graph.systems[node.index()].get().unwrap().has_deferred();

            for target in dependency_flattened.neighbors_directed(*node, Direction::Outgoing) {
                let add_sync_on_edge = add_sync_after
                    && !is_apply_deferred(graph.systems[target.index()].get().unwrap())
                    && !self.no_sync_edges.contains(&(*node, target));

                let weight = if add_sync_on_edge { 1 } else { 0 };

                let distance = distances
                    .get(&target.index())
                    .unwrap_or(&None)
                    .or(Some(0))
                    .map(|distance| {
                        distance.max(
                            distances.get(&node.index()).unwrap_or(&None).unwrap_or(0) + weight,
                        )
                    });

                distances.insert(target.index(), distance);

                if add_sync_on_edge {
                    let sync_point =
                        self.get_sync_point(graph, distances[&target.index()].unwrap());
                    sync_point_graph.add_edge(*node, sync_point);
                    sync_point_graph.add_edge(sync_point, target);

                    // edge is now redundant
                    sync_point_graph.remove_edge(*node, target);
                }
            }
        }

        *dependency_flattened = sync_point_graph;
        Ok(())
    }

    fn collapse_set(
        &mut self,
        set: NodeId,
        systems: &[NodeId],
        dependency_flattened: &DiGraph,
    ) -> impl Iterator<Item = (NodeId, NodeId)> {
        if systems.is_empty() {
            // collapse dependencies for empty sets
            for a in dependency_flattened.neighbors_directed(set, Direction::Incoming) {
                for b in dependency_flattened.neighbors_directed(set, Direction::Outgoing) {
                    if self.no_sync_edges.contains(&(a, set))
                        && self.no_sync_edges.contains(&(set, b))
                    {
                        self.no_sync_edges.insert((a, b));
                    }
                }
            }
        } else {
            for a in dependency_flattened.neighbors_directed(set, Direction::Incoming) {
                for &sys in systems {
                    if self.no_sync_edges.contains(&(a, set)) {
                        self.no_sync_edges.insert((a, sys));
                    }
                }
            }

            for b in dependency_flattened.neighbors_directed(set, Direction::Outgoing) {
                for &sys in systems {
                    if self.no_sync_edges.contains(&(set, b)) {
                        self.no_sync_edges.insert((sys, b));
                    }
                }
            }
        }
        core::iter::empty()
    }
}
