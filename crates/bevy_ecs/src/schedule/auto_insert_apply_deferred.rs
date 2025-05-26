use alloc::{boxed::Box, collections::BTreeSet, vec::Vec};

use bevy_platform::collections::HashMap;

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

        fn set_has_conditions(graph: &ScheduleGraph, node: NodeId) -> bool {
            !graph.set_conditions_at(node).is_empty()
                || graph
                    .hierarchy()
                    .graph()
                    .edges_directed(node, Direction::Incoming)
                    .any(|(parent, _)| set_has_conditions(graph, parent))
        }

        fn system_has_conditions(graph: &ScheduleGraph, node: NodeId) -> bool {
            assert!(node.is_system());
            !graph.system_conditions[node.index()].is_empty()
                || graph
                    .hierarchy()
                    .graph()
                    .edges_directed(node, Direction::Incoming)
                    .any(|(parent, _)| set_has_conditions(graph, parent))
        }

        let mut system_has_conditions_cache = HashMap::<usize, bool>::default();
        let mut is_valid_explicit_sync_point = |system: NodeId| {
            let index = system.index();
            is_apply_deferred(graph.systems[index].get().unwrap())
                && !*system_has_conditions_cache
                    .entry(index)
                    .or_insert_with(|| system_has_conditions(graph, system))
        };

        // Calculate the distance for each node.
        // The "distance" is the number of sync points between a node and the beginning of the graph.
        // Also store if a preceding edge would have added a sync point but was ignored to add it at
        // a later edge that is not ignored.
        let mut distances_and_pending_sync: HashMap<usize, (u32, bool)> =
            HashMap::with_capacity_and_hasher(topo.len(), Default::default());

        // Keep track of any explicit sync nodes for a specific distance.
        let mut distance_to_explicit_sync_node: HashMap<u32, NodeId> = HashMap::default();

        // Determine the distance for every node and collect the explicit sync points.
        for node in &topo {
            let (node_distance, mut node_needs_sync) = distances_and_pending_sync
                .get(&node.index())
                .copied()
                .unwrap_or_default();

            if is_valid_explicit_sync_point(*node) {
                // The distance of this sync point does not change anymore as the iteration order
                // makes sure that this node is no unvisited target of another node.
                // Because of this, the sync point can be stored for this distance to be reused as
                // automatically added sync points later.
                distance_to_explicit_sync_node.insert(node_distance, *node);

                // This node just did a sync, so the only reason to do another sync is if one was
                // explicitly scheduled afterwards.
                node_needs_sync = false;
            } else if !node_needs_sync {
                // No previous node has postponed sync points to add so check if the system itself
                // has deferred params that require a sync point to apply them.
                node_needs_sync = graph.systems[node.index()].get().unwrap().has_deferred();
            }

            for target in dependency_flattened.neighbors_directed(*node, Direction::Outgoing) {
                let (target_distance, target_pending_sync) = distances_and_pending_sync
                    .entry(target.index())
                    .or_default();

                let mut edge_needs_sync = node_needs_sync;
                if node_needs_sync
                    && !graph.systems[target.index()].get().unwrap().is_exclusive()
                    && self.no_sync_edges.contains(&(*node, target))
                {
                    // The node has deferred params to apply, but this edge is ignoring sync points.
                    // Mark the target as 'delaying' those commands to a future edge and the current
                    // edge as not needing a sync point.
                    *target_pending_sync = true;
                    edge_needs_sync = false;
                }

                let mut weight = 0;
                if edge_needs_sync || is_valid_explicit_sync_point(target) {
                    // The target distance grows if a sync point is added between it and the node.
                    // Also raise the distance if the target is a sync point itself so it then again
                    // raises the distance of following nodes as that is what the distance is about.
                    weight = 1;
                }

                // The target cannot have fewer sync points in front of it than the preceding node.
                *target_distance = (node_distance + weight).max(*target_distance);
            }
        }

        // Find any edges which have a different number of sync points between them and make sure
        // there is a sync point between them.
        for node in &topo {
            let (node_distance, _) = distances_and_pending_sync
                .get(&node.index())
                .copied()
                .unwrap_or_default();

            for target in dependency_flattened.neighbors_directed(*node, Direction::Outgoing) {
                let (target_distance, _) = distances_and_pending_sync
                    .get(&target.index())
                    .copied()
                    .unwrap_or_default();

                if node_distance == target_distance {
                    // These nodes are the same distance, so they don't need an edge between them.
                    continue;
                }

                if is_apply_deferred(graph.systems[target.index()].get().unwrap()) {
                    // We don't need to insert a sync point since ApplyDeferred is a sync point
                    // already!
                    continue;
                }

                let sync_point = distance_to_explicit_sync_node
                    .get(&target_distance)
                    .copied()
                    .unwrap_or_else(|| self.get_sync_point(graph, target_distance));

                sync_point_graph.add_edge(*node, sync_point);
                sync_point_graph.add_edge(sync_point, target);

                // The edge without the sync point is now redundant.
                sync_point_graph.remove_edge(*node, target);
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
