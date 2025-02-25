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

        let mut system_has_conditions_cache = HashMap::default();

        fn is_valid_explicit_sync_point(
            graph: &ScheduleGraph,
            system: NodeId,
            system_has_conditions_cache: &mut HashMap<usize, bool>,
        ) -> bool {
            let index = system.index();
            is_apply_deferred(graph.systems[index].get().unwrap())
                && !*system_has_conditions_cache
                    .entry(index)
                    .or_insert_with(|| system_has_conditions(graph, system))
        }

        // calculate the number of sync points each sync point is from the beginning of the graph
        let mut distances: HashMap<usize, u32> =
            HashMap::with_capacity_and_hasher(topo.len(), Default::default());
        // Keep track of any explicit sync nodes for a specific distance.
        let mut distance_to_explicit_sync_node: HashMap<u32, NodeId> = HashMap::default();
        for node in &topo {
            let node_system = graph.systems[node.index()].get().unwrap();

            let node_needs_sync =
                if is_valid_explicit_sync_point(graph, *node, &mut system_has_conditions_cache) {
                    distance_to_explicit_sync_node.insert(
                        distances.get(&node.index()).copied().unwrap_or_default(),
                        *node,
                    );

                    // This node just did a sync, so the only reason to do another sync is if one was
                    // explicitly scheduled afterwards.
                    false
                } else {
                    node_system.has_deferred()
                };

            for target in dependency_flattened.neighbors_directed(*node, Direction::Outgoing) {
                let edge_needs_sync = node_needs_sync
                    && !self.no_sync_edges.contains(&(*node, target))
                    || is_valid_explicit_sync_point(
                        graph,
                        target,
                        &mut system_has_conditions_cache,
                    );

                let weight = if edge_needs_sync { 1 } else { 0 };

                // Use whichever distance is larger, either the current distance, or the distance to
                // the parent plus the weight.
                let distance = distances
                    .get(&target.index())
                    .copied()
                    .unwrap_or_default()
                    .max(distances.get(&node.index()).copied().unwrap_or_default() + weight);

                distances.insert(target.index(), distance);
            }
        }

        // Find any edges which have a different number of sync points between them and make sure
        // there is a sync point between them.
        for node in &topo {
            let node_distance = distances.get(&node.index()).copied().unwrap_or_default();
            for target in dependency_flattened.neighbors_directed(*node, Direction::Outgoing) {
                let target_distance = distances.get(&target.index()).copied().unwrap_or_default();
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
