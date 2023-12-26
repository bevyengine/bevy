use std::collections::BTreeSet;

use bevy_utils::{petgraph::Direction::Outgoing, HashMap};

use crate::system::IntoSystem;

use super::{
    apply_deferred, is_apply_deferred, NodeId, ReportCycles, ScheduleBuildPass, ScheduleGraph,
    SystemNode,
};

#[derive(Debug)]
pub struct AutoInsertApplyDeferedPass {
    no_sync_edges: BTreeSet<(NodeId, NodeId)>,
    auto_sync_node_ids: HashMap<u32, NodeId>,
}

pub struct IgnoreDeferred;

impl AutoInsertApplyDeferedPass {
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
    /// add an [`apply_deferred`] system with no config
    fn add_auto_sync(&mut self, graph: &mut ScheduleGraph) -> NodeId {
        let id = NodeId::System(graph.systems.len());

        graph
            .systems
            .push(SystemNode::new(Box::new(IntoSystem::into_system(
                apply_deferred,
            ))));
        graph.system_conditions.push(Vec::new());

        // ignore ambiguities with auto sync points
        // They aren't under user control, so no one should know or care.
        graph.ambiguous_with_all.insert(id);

        id
    }
}

impl ScheduleBuildPass for AutoInsertApplyDeferedPass {
    type EdgeOptions = IgnoreDeferred;

    fn add_dependency(&mut self, from: NodeId, to: NodeId, options: Option<&Self::EdgeOptions>) {
        if let Some(options) = options {
            self.no_sync_edges.insert((from, to));
        }
    }

    fn build(
        &mut self,
        graph: &mut ScheduleGraph,
        dependency_flattened: &mut bevy_utils::petgraph::prelude::GraphMap<
            super::NodeId,
            (),
            bevy_utils::petgraph::prelude::Directed,
        >,
    ) -> Result<
        bevy_utils::petgraph::prelude::GraphMap<
            super::NodeId,
            (),
            bevy_utils::petgraph::prelude::Directed,
        >,
        super::ScheduleBuildError,
    > {
        let mut sync_point_graph = dependency_flattened.clone();
        let topo = graph.topsort_graph(dependency_flattened, ReportCycles::Dependency)?;

        // calculate the number of sync points each sync point is from the beginning of the graph
        // use the same sync point if the distance is the same
        let mut distances: HashMap<usize, Option<u32>> = HashMap::with_capacity(topo.len());
        for node in &topo {
            let add_sync_after = graph.systems[node.index()].get().unwrap().has_deferred();

            for target in dependency_flattened.neighbors_directed(*node, Outgoing) {
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
                    sync_point_graph.add_edge(*node, sync_point, ());
                    sync_point_graph.add_edge(sync_point, target, ());

                    // edge is now redundant
                    sync_point_graph.remove_edge(*node, target);
                }
            }
        }

        Ok(sync_point_graph)
    }
}
