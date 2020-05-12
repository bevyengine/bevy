use super::{NodeId, NodeState, RenderGraph, RenderGraphError};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StagerError {
    #[error("Encountered a RenderGraphError")]
    RenderGraphError(#[from] RenderGraphError),
}

#[derive(Default, Debug, Eq, PartialEq)]
pub struct Stage {
    pub jobs: Vec<OrderedJob>,
}

#[derive(Default, Debug, Eq, PartialEq)]
pub struct OrderedJob {
    pub nodes: Vec<NodeId>,
}

#[derive(Default, Debug)]
pub struct StageBorrow<'a> {
    pub jobs: Vec<OrderedJobBorrow<'a>>,
}

#[derive(Default, Debug)]
pub struct OrderedJobBorrow<'a> {
    pub node_states: Vec<&'a mut NodeState>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct NodeIndices {
    stage: usize,
    job: usize,
    node: usize,
}

#[derive(Default, Debug)]
pub struct Stages {
    stages: Vec<Stage>,
    /// a collection of node indices that are used to efficiently borrow render graph nodes
    node_indices: HashMap<NodeId, NodeIndices>,
}

impl Stages {
    pub fn new(stages: Vec<Stage>) -> Self {
        let mut node_indices = HashMap::default();
        for (stage_index, stage) in stages.iter().enumerate() {
            for (job_index, job) in stage.jobs.iter().enumerate() {
                for (node_index, node) in job.nodes.iter().enumerate() {
                    node_indices.insert(
                        *node,
                        NodeIndices {
                            stage: stage_index,
                            job: job_index,
                            node: node_index,
                        },
                    );
                }
            }
        }
        Stages {
            stages,
            node_indices,
        }
    }

    pub fn borrow<'a>(&self, render_graph: &'a mut RenderGraph) -> Vec<StageBorrow<'a>> {
        // unfortunately borrowing render graph nodes in a specific order takes a little bit of gymnastics
        let mut stage_borrows = Vec::with_capacity(self.stages.len());

        let mut node_borrows = Vec::new();
        for node in render_graph.iter_nodes_mut() {
            let indices = self.node_indices.get(&node.id).unwrap();
            node_borrows.push((node, indices));
        }

        node_borrows.sort_by_key(|(_node, indices)| indices.clone());
        let mut last_stage = usize::MAX;
        let mut last_job = usize::MAX;
        for (node, indices) in node_borrows.drain(..) {
            if last_stage != indices.stage {
                stage_borrows.push(StageBorrow::default());
                last_job = usize::MAX;
            }

            let stage = &mut stage_borrows[indices.stage];
            if last_job != indices.job {
                stage.jobs.push(OrderedJobBorrow::default());
            }

            let job = &mut stage.jobs[indices.job];
            job.node_states.push(node);

            last_stage = indices.stage;
            last_job = indices.job;
        }

        stage_borrows
    }
}

/// Produces a collection of `Stages`, which are sets of OrderedJobs that must be run before moving on to the next stage
pub trait RenderGraphStager {
    fn get_stages(&mut self, render_graph: &RenderGraph) -> Result<Stages, RenderGraphError>;
}

// TODO: remove this
/// This scheduler ignores dependencies and puts everything in one stage. It shouldn't be used for anything :)
#[derive(Default)]
pub struct LinearStager;

impl RenderGraphStager for LinearStager {
    fn get_stages(&mut self, render_graph: &RenderGraph) -> Result<Stages, RenderGraphError> {
        let mut stage = Stage::default();
        let mut job = OrderedJob::default();
        for node in render_graph.iter_nodes() {
            job.nodes.push(node.id);
        }

        stage.jobs.push(job);

        Ok(Stages::new(vec![stage]))
    }
}

#[derive(Copy, Clone)]
/// Determines the grouping strategy used when constructing graph stages
pub enum JobGrouping {
    /// Default to adding the current node to a new job in its assigned stage. This results
    /// in a "loose" pack that is easier to parallelize but has more jobs
    Loose,
    /// Default to adding the current node into the first job in its assigned stage. This results
    /// in a "tight" pack that is harder to parallelize but results in fewer jobs
    Tight,
}

/// Produces Render Graph stages and jobs in a way that ensures node dependencies are respected.
pub struct DependentNodeStager {
    job_grouping: JobGrouping,
}

impl DependentNodeStager {
    pub fn loose_grouping() -> Self {
        DependentNodeStager {
            job_grouping: JobGrouping::Loose,
        }
    }

    pub fn tight_grouping() -> Self {
        DependentNodeStager {
            job_grouping: JobGrouping::Tight,
        }
    }
}

impl RenderGraphStager for DependentNodeStager {
    fn get_stages<'a>(&mut self, render_graph: &RenderGraph) -> Result<Stages, RenderGraphError> {
        // get all nodes without input. this intentionally includes nodes with no outputs
        let output_only_nodes = render_graph
            .iter_nodes()
            .filter(|node| node.input_slots.len() == 0);
        let mut stages = vec![Stage::default()];
        let mut node_stages = HashMap::new();
        for output_only_node in output_only_nodes {
            // each "output only" node should start a new job on the first stage
            stage_node(
                render_graph,
                &mut stages,
                &mut node_stages,
                output_only_node,
                self.job_grouping,
            );
        }

        Ok(Stages::new(stages))
    }
}

fn stage_node(
    graph: &RenderGraph,
    stages: &mut Vec<Stage>,
    node_stages_and_jobs: &mut HashMap<NodeId, (usize, usize)>,
    node: &NodeState,
    job_grouping: JobGrouping,
) {
    // don't re-visit nodes or visit them before all of their parents have been visited
    if node_stages_and_jobs.contains_key(&node.id)
        || node
            .edges
            .input_edges
            .iter()
            .find(|e| !node_stages_and_jobs.contains_key(&e.get_output_node()))
            .is_some()
    {
        return;
    }

    // by default assume we are creating a new job on a new stage
    let mut stage_index = 0;
    let mut job_index = match job_grouping {
        JobGrouping::Tight => Some(0),
        JobGrouping::Loose => None,
    };

    // check to see if the current node has a parent. if so, grab the parent with the highest stage
    if let Some((max_parent_stage, max_parent_job)) = node
        .edges
        .input_edges
        .iter()
        .map(|e| {
            node_stages_and_jobs
                .get(&e.get_output_node())
                .expect("already checked that parents were visited")
        })
        .max()
    {
        // count the number of parents that are in the highest stage
        let max_stage_parent_count = node
            .edges
            .input_edges
            .iter()
            .filter(|e| {
                let (max_stage, _) = node_stages_and_jobs
                    .get(&e.get_output_node())
                    .expect("already checked that parents were visited");
                max_stage == max_parent_stage
            })
            .count();

        // if the current node has more than one parent on the highest stage (aka requires synchronization), then move it to the next
        // stage and start a new job on that stage
        if max_stage_parent_count > 1 {
            stage_index = max_parent_stage + 1;
        } else {
            stage_index = *max_parent_stage;
            job_index = Some(*max_parent_job);
        }
    }

    if stage_index == stages.len() {
        stages.push(Stage::default());
    }

    let stage = &mut stages[stage_index];

    let job_index = job_index.unwrap_or_else(|| stage.jobs.len());
    if job_index == stage.jobs.len() {
        stage.jobs.push(OrderedJob::default());
    }

    let job = &mut stage.jobs[job_index];
    job.nodes.push(node.id);

    node_stages_and_jobs.insert(node.id, (stage_index, job_index));

    for (_edge, node) in graph.iter_node_outputs(node.id).unwrap() {
        stage_node(graph, stages, node_stages_and_jobs, node, job_grouping);
    }
}

#[cfg(test)]
mod tests {
    use super::{DependentNodeStager, OrderedJob, RenderGraphStager, Stage};
    use crate::{
        render_graph::{Node, NodeId, RenderGraph, ResourceSlotInfo, ResourceSlots},
        renderer::RenderContext, shader::FieldBindType,
    };
    use legion::prelude::{Resources, World};

    struct TestNode {
        inputs: Vec<ResourceSlotInfo>,
        outputs: Vec<ResourceSlotInfo>,
    }

    impl TestNode {
        pub fn new(inputs: usize, outputs: usize) -> Self {
            TestNode {
                inputs: (0..inputs)
                    .map(|i| ResourceSlotInfo {
                        name: format!("in_{}", i).into(),
                        resource_type: FieldBindType::Texture,
                    })
                    .collect(),
                outputs: (0..outputs)
                    .map(|i| ResourceSlotInfo {
                        name: format!("out_{}", i).into(),
                        resource_type: FieldBindType::Texture,
                    })
                    .collect(),
            }
        }
    }

    impl Node for TestNode {
        fn input(&self) -> &[ResourceSlotInfo] {
            &self.inputs
        }

        fn output(&self) -> &[ResourceSlotInfo] {
            &self.outputs
        }
        fn update(
            &mut self,
            _: &World,
            _: &Resources,
            _: &mut dyn RenderContext,
            _: &ResourceSlots,
            _: &mut ResourceSlots,
        ) {
        }
    }

    #[test]
    fn test_render_graph_dependency_stager_loose() {
        let mut graph = RenderGraph::default();

        // Setup graph to look like this:
        //
        // A -> B -> C -> D
        //    /     /
        //  E      F -> G
        //
        // H -> I -> J

        let a_id = graph.add_node_named("A", TestNode::new(0, 1));
        let b_id = graph.add_node_named("B", TestNode::new(2, 1));
        let c_id = graph.add_node_named("C", TestNode::new(2, 1));
        let d_id = graph.add_node_named("D", TestNode::new(1, 0));
        let e_id = graph.add_node_named("E", TestNode::new(0, 1));
        let f_id = graph.add_node_named("F", TestNode::new(0, 2));
        let g_id = graph.add_node_named("G", TestNode::new(1, 0));
        let h_id = graph.add_node_named("H", TestNode::new(0, 1));
        let i_id = graph.add_node_named("I", TestNode::new(1, 1));
        let j_id = graph.add_node_named("J", TestNode::new(1, 0));

        graph.add_node_edge("A", "B").unwrap();
        graph.add_node_edge("B", "C").unwrap();
        graph.add_node_edge("C", "D").unwrap();
        graph.add_node_edge("E", "B").unwrap();
        graph.add_node_edge("F", "C").unwrap();
        graph.add_node_edge("F", "G").unwrap();
        graph.add_node_edge("H", "I").unwrap();
        graph.add_node_edge("I", "J").unwrap();

        let mut stager = DependentNodeStager::loose_grouping();
        let mut stages = stager.get_stages(&graph).unwrap();

        // Expected Stages:
        // (X indicates nodes that are not part of that stage)

        // Stage 1
        // A -> X -> X -> X
        //    /     /
        //  E      F -> G
        //
        // H -> I -> J

        // Stage 2
        // X -> B -> C -> D
        //    /     /
        //  X      X -> X
        //
        // X -> X -> X

        let mut expected_stages = vec![
            Stage {
                jobs: vec![
                    OrderedJob {
                        nodes: vec![f_id, g_id],
                    },
                    OrderedJob { nodes: vec![a_id] },
                    OrderedJob { nodes: vec![e_id] },
                    OrderedJob {
                        nodes: vec![h_id, i_id, j_id],
                    },
                ],
            },
            Stage {
                jobs: vec![OrderedJob {
                    nodes: vec![b_id, c_id, d_id],
                }],
            },
        ];

        // ensure job order lines up within stages (this can vary due to hash maps)
        // jobs within a stage are unordered conceptually so this is ok
        expected_stages
            .iter_mut()
            .for_each(|stage| stage.jobs.sort_by_key(|job| job.nodes[0]));

        stages
            .stages
            .iter_mut()
            .for_each(|stage| stage.jobs.sort_by_key(|job| job.nodes[0]));

        assert_eq!(
            stages.stages, expected_stages,
            "stages should be loosely grouped"
        );

        let mut borrowed = stages.borrow(&mut graph);
        // ensure job order lines up within stages (this can vary due to hash maps)
        // jobs within a stage are unordered conceptually so this is ok
        borrowed
            .iter_mut()
            .for_each(|stage| stage.jobs.sort_by_key(|job| job.node_states[0].id));

        assert_eq!(
            borrowed.len(),
            expected_stages.len(),
            "same number of stages"
        );
        for (stage_index, borrowed_stage) in borrowed.iter().enumerate() {
            assert_eq!(
                borrowed_stage.jobs.len(),
                stages.stages[stage_index].jobs.len(),
                "job length matches"
            );
            for (job_index, borrowed_job) in borrowed_stage.jobs.iter().enumerate() {
                assert_eq!(
                    borrowed_job.node_states.len(),
                    stages.stages[stage_index].jobs[job_index].nodes.len(),
                    "node length matches"
                );
                for (node_index, borrowed_node) in borrowed_job.node_states.iter().enumerate() {
                    assert_eq!(
                        borrowed_node.id,
                        stages.stages[stage_index].jobs[job_index].nodes[node_index]
                    );
                }
            }
        }
    }

    #[test]
    fn test_render_graph_dependency_stager_tight() {
        let mut graph = RenderGraph::default();

        // Setup graph to look like this:
        //
        // A -> B -> C -> D
        //    /     /
        //  E      F -> G
        //
        // H -> I -> J

        let _a_id = graph.add_node_named("A", TestNode::new(0, 1));
        let b_id = graph.add_node_named("B", TestNode::new(2, 1));
        let c_id = graph.add_node_named("C", TestNode::new(2, 1));
        let d_id = graph.add_node_named("D", TestNode::new(1, 0));
        let _e_id = graph.add_node_named("E", TestNode::new(0, 1));
        let f_id = graph.add_node_named("F", TestNode::new(0, 2));
        let g_id = graph.add_node_named("G", TestNode::new(1, 0));
        let h_id = graph.add_node_named("H", TestNode::new(0, 1));
        let i_id = graph.add_node_named("I", TestNode::new(1, 1));
        let j_id = graph.add_node_named("J", TestNode::new(1, 0));

        graph.add_node_edge("A", "B").unwrap();
        graph.add_node_edge("B", "C").unwrap();
        graph.add_node_edge("C", "D").unwrap();
        graph.add_node_edge("E", "B").unwrap();
        graph.add_node_edge("F", "C").unwrap();
        graph.add_node_edge("F", "G").unwrap();
        graph.add_node_edge("H", "I").unwrap();
        graph.add_node_edge("I", "J").unwrap();

        let mut stager = DependentNodeStager::tight_grouping();
        let mut stages = stager.get_stages(&graph).unwrap();

        // Expected Stages:
        // (X indicates nodes that are not part of that stage)

        // Stage 1
        // A -> X -> X -> X
        //    /     /
        //  E      F -> G
        //
        // H -> I -> J

        // Stage 2
        // X -> B -> C -> D
        //    /     /
        //  X      X -> X
        //
        // X -> X -> X

        assert_eq!(stages.stages[0].jobs.len(), 1, "expect exactly 1 job");

        let job = &stages.stages[0].jobs[0];

        assert_eq!(job.nodes.len(), 7, "expect exactly 7 nodes in the job");

        // its hard to test the exact order of this job's nodes because of hashing, so instead we'll test the constraints that must hold true
        let index =
            |node_id: NodeId| -> usize { job.nodes.iter().position(|id| *id == node_id).unwrap() };

        assert!(index(f_id) < index(g_id));
        assert!(index(h_id) < index(i_id));
        assert!(index(i_id) < index(j_id));

        let expected_stage_1 = Stage {
            jobs: vec![OrderedJob {
                nodes: vec![b_id, c_id, d_id],
            }],
        };

        assert_eq!(stages.stages[1], expected_stage_1,);

        let mut borrowed = stages.borrow(&mut graph);
        // ensure job order lines up within stages (this can vary due to hash maps)
        // jobs within a stage are unordered conceptually so this is ok
        stages
            .stages
            .iter_mut()
            .for_each(|stage| stage.jobs.sort_by_key(|job| job.nodes[0]));
        borrowed
            .iter_mut()
            .for_each(|stage| stage.jobs.sort_by_key(|job| job.node_states[0].id));

        assert_eq!(borrowed.len(), 2, "same number of stages");
        for (stage_index, borrowed_stage) in borrowed.iter().enumerate() {
            assert_eq!(
                borrowed_stage.jobs.len(),
                stages.stages[stage_index].jobs.len(),
                "job length matches"
            );
            for (job_index, borrowed_job) in borrowed_stage.jobs.iter().enumerate() {
                assert_eq!(
                    borrowed_job.node_states.len(),
                    stages.stages[stage_index].jobs[job_index].nodes.len(),
                    "node length matches"
                );
                for (node_index, borrowed_node) in borrowed_job.node_states.iter().enumerate() {
                    assert_eq!(
                        borrowed_node.id,
                        stages.stages[stage_index].jobs[job_index].nodes[node_index]
                    );
                }
            }
        }
    }
}
