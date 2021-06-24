use crate::schedule::{SystemDescriptor, SystemLabel, SystemSet};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

static NEXT_NODE_ID: AtomicU32 = AtomicU32::new(0);

/// A builder for creating graphs of dependent parallel execution within a `SystemStage`.
///
/// # Sequential Execution
/// The simplest graph is a sequence of systems that need to run one after another.
/// ```
/// # use bevy_ecs::prelude::*;
/// # fn sys_a() {}
/// # fn sys_b() {}
/// # fn sys_c() {}
/// let graph = SystemGraph::new();
/// graph
///   .root(sys_a.system())
///   .then(sys_b.system())
///   .then(sys_c.system());
///
/// // Convert into a SystemSet
/// let system_set: SystemSet = graph.into();
/// ```
///
/// # Fan Out
/// `SystemGraphNode::then` can be called repeatedly on the same node to create multiple fan-out
/// branches. All fanned out systems will not execute until the original has finished.
/// ```
/// # use bevy_ecs::prelude::*;
/// # fn sys_a() {}
/// # fn sys_b() {}
/// # fn sys_c() {}
/// # fn sys_d() {}
/// # fn sys_e() {}
/// let graph = SystemGraph::new();
///
/// let start_a = graph.root(sys_a.system());
///
/// start_a.then(sys_b.system());
/// start_a.then(sys_c.system());
///
/// start_a
///     .then(sys_d.system())
///     .then(sys_e.system());
///
/// // Convert into a SystemSet
/// let system_set: SystemSet = graph.into();
/// ```
///
/// # Fan In
/// A graph node can wait on multiple systems before running.
/// `SystemGraphJoinExt::join_into` is implemented on any type that iterates over
/// `SystemGraphNode`.
/// ```
/// # use bevy_ecs::prelude::*;
/// # fn sys_a() {}
/// # fn sys_b() {}
/// # fn sys_c() {}
/// # fn sys_d() {}
/// # fn sys_e() {}
/// let graph = SystemGraph::new();
///
/// let start_a = graph.root(sys_a.system());
/// let start_b = graph.root(sys_b.system());
/// let start_c = graph.root(sys_c.system());
///
/// vec![start_a, start_b, start_c]
///     .join_into(sys_d.system())
///     .then(sys_e.system());
///
/// // Convert into a SystemSet
/// let system_set: SystemSet = graph.into();
/// ```
#[derive(Clone, Default)]
pub struct SystemGraph(Arc<Mutex<HashMap<NodeId, SystemDescriptor>>>);

impl SystemGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn root(&self, system: impl Into<SystemDescriptor>) -> SystemGraphNode {
        self.create_node(system.into())
    }

    pub fn is_same_graph(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    fn create_node(&self, mut system: SystemDescriptor) -> SystemGraphNode {
        let id = NodeId(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed));
        match &mut system {
            SystemDescriptor::Parallel(descriptor) => descriptor.labels.push(id.dyn_clone()),
            SystemDescriptor::Exclusive(descriptor) => descriptor.labels.push(id.dyn_clone()),
        }
        self.0.lock().unwrap().insert(id, system);
        SystemGraphNode {
            id,
            graph: self.clone(),
        }
    }

    fn add_dependency(&self, src: NodeId, dst: NodeId) {
        if let Some(system) = self.0.lock().unwrap().get_mut(&dst) {
            match system {
                SystemDescriptor::Parallel(descriptor) => descriptor.after.push(src.dyn_clone()),
                SystemDescriptor::Exclusive(descriptor) => descriptor.after.push(src.dyn_clone()),
            }
        } else {
            panic!(
                "Attempted to add dependency for {:?}, which doesn't exist.",
                dst
            );
        }
    }
}

/// A draining conversion from [SystemGraph] to [SystemSet]. All other clones of the same graph
/// will be empty afterwards.
///
/// [SystemGraph]: crate::schedule::SystemSet
/// [SystemSet]: crate::schedule::SystemSet
impl From<SystemGraph> for SystemSet {
    fn from(graph: SystemGraph) -> Self {
        let mut system_set = SystemSet::new();
        for (_, system) in graph.0.lock().unwrap().drain() {
            system_set = system_set.with_system(system);
        }
        system_set
    }
}

#[derive(Clone)]
pub struct SystemGraphNode {
    id: NodeId,
    graph: SystemGraph,
}

impl SystemGraphNode {
    pub fn then(&self, next: impl Into<SystemDescriptor>) -> SystemGraphNode {
        let node = self.graph.create_node(next.into());
        self.add_dependent(node.id);
        node
    }

    fn add_dependent(&self, dst: NodeId) {
        self.graph.add_dependency(self.id, dst);
    }
}

pub trait SystemGraphJoinExt: Sized + IntoIterator<Item = SystemGraphNode> {
    fn join_into(self, next: impl Into<SystemDescriptor>) -> SystemGraphNode {
        let mut nodes = self.into_iter().peekable();
        let output = nodes
            .peek()
            .map(|node| {
                let output_node = node.graph.create_node(next.into());
                node.add_dependent(output_node.id);
                output_node
            })
            .expect("Attempted to join a collection of zero nodes.");

        for node in nodes {
            assert!(
                output.graph.is_same_graph(&node.graph),
                "Joined graph nodes should be from the same graph."
            );
            node.add_dependent(output.id);
        }

        output
    }
}

impl<T: IntoIterator<Item = SystemGraphNode>> SystemGraphJoinExt for T {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct NodeId(u32);

impl SystemLabel for NodeId {
    fn dyn_clone(&self) -> Box<dyn SystemLabel> {
        Box::new(<NodeId>::clone(self))
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use crate::schedule::SystemDescriptor;

    fn dummy_system() {}

    #[test]
    pub fn graph_creates_accurate_system_counts() {
        let graph = SystemGraph::new();
        let a = graph
            .root(dummy_system.system())
            .then(dummy_system.system())
            .then(dummy_system.system())
            .then(dummy_system.system());
        let b = graph
            .root(dummy_system.system())
            .then(dummy_system.system());
        let c = graph
            .root(dummy_system.system())
            .then(dummy_system.system())
            .then(dummy_system.system());
        vec![a, b, c]
            .join_into(dummy_system.system())
            .then(dummy_system.system());
        let system_set: SystemSet = graph.into();
        let (_, systems) = system_set.bake();

        assert_eq!(systems.len(), 11);
    }

    #[test]
    pub fn all_nodes_are_labeled() {
        let graph = SystemGraph::new();
        let a = graph
            .root(dummy_system.system())
            .then(dummy_system.system())
            .then(dummy_system.system())
            .then(dummy_system.system());
        let b = graph
            .root(dummy_system.system())
            .then(dummy_system.system());
        let c = graph
            .root(dummy_system.system())
            .then(dummy_system.system())
            .then(dummy_system.system());
        vec![a, b, c]
            .join_into(dummy_system.system())
            .then(dummy_system.system());
        let system_set: SystemSet = graph.into();
        let (_, systems) = system_set.bake();

        let mut root_count = 0;
        for system in systems {
            match system {
                SystemDescriptor::Parallel(desc) => {
                    assert!(!desc.labels.is_empty());
                    if desc.after.is_empty() {
                        root_count += 1;
                    }
                }
                SystemDescriptor::Exclusive(desc) => {
                    assert!(!desc.labels.is_empty());
                    if desc.after.is_empty() {
                        root_count += 1;
                    }
                }
            }
        }
        assert_eq!(root_count, 3);
    }
}
