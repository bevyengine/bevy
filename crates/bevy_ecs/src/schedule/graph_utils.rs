use bevy_utils::{tracing::warn, HashMap, HashSet};
use fixedbitset::FixedBitSet;
use std::{borrow::Cow, fmt::Debug, hash::Hash};

pub enum DependencyGraphError<Labels> {
    GraphCycles(Vec<(usize, Labels)>),
}

pub trait GraphNode {
    type Label;
    fn name(&self) -> Cow<'static, str>;
    fn labels(&self) -> &[Self::Label];
    fn before(&self) -> &[Self::Label];
    fn after(&self) -> &[Self::Label];
}

/// Constructs a dependency graph of given nodes.
pub fn build_dependency_graph<Node>(
    nodes: &[Node],
) -> HashMap<usize, HashMap<usize, HashSet<Node::Label>>>
where
    Node: GraphNode,
    Node::Label: Debug + Clone + Eq + Hash,
{
    let mut labels = HashMap::<Node::Label, FixedBitSet>::default();
    for (label, index) in nodes.iter().enumerate().flat_map(|(index, container)| {
        container
            .labels()
            .iter()
            .cloned()
            .map(move |label| (label, index))
    }) {
        labels
            .entry(label)
            .or_insert_with(|| FixedBitSet::with_capacity(nodes.len()))
            .insert(index);
    }
    let mut graph = HashMap::with_capacity_and_hasher(nodes.len(), Default::default());
    for (index, node) in nodes.iter().enumerate() {
        let dependencies = graph.entry(index).or_insert_with(HashMap::default);
        for label in node.after() {
            match labels.get(label) {
                Some(new_dependencies) => {
                    for dependency in new_dependencies.ones() {
                        dependencies
                            .entry(dependency)
                            .or_insert_with(HashSet::default)
                            .insert(label.clone());
                    }
                }
                None => warn!(
                    // TODO: plumb this as proper output?
                    "{} wants to be after unknown label: {:?}",
                    nodes[index].name(),
                    label
                ),
            }
        }
        for label in node.before() {
            match labels.get(label) {
                Some(dependants) => {
                    for dependant in dependants.ones() {
                        graph
                            .entry(dependant)
                            .or_insert_with(HashMap::default)
                            .entry(index)
                            .or_insert_with(HashSet::default)
                            .insert(label.clone());
                    }
                }
                None => warn!(
                    "{} wants to be before unknown label: {:?}",
                    nodes[index].name(),
                    label
                ),
            }
        }
    }
    graph
}

/// Generates a topological order for the given graph.
pub fn topological_order<Labels: Clone>(
    graph: &HashMap<usize, HashMap<usize, Labels>>,
) -> Result<Vec<usize>, DependencyGraphError<Labels>> {
    fn check_if_cycles_and_visit<L>(
        node: &usize,
        graph: &HashMap<usize, HashMap<usize, L>>,
        sorted: &mut Vec<usize>,
        unvisited: &mut HashSet<usize>,
        current: &mut Vec<usize>,
    ) -> bool {
        if current.contains(node) {
            return true;
        } else if !unvisited.remove(node) {
            return false;
        }
        current.push(*node);
        for dependency in graph.get(node).unwrap().keys() {
            if check_if_cycles_and_visit(dependency, graph, sorted, unvisited, current) {
                return true;
            }
        }
        sorted.push(*node);
        current.pop();
        false
    }
    let mut sorted = Vec::with_capacity(graph.len());
    let mut current = Vec::with_capacity(graph.len());
    let mut unvisited = HashSet::with_capacity_and_hasher(graph.len(), Default::default());
    unvisited.extend(graph.keys().cloned());
    while let Some(node) = unvisited.iter().next().cloned() {
        if check_if_cycles_and_visit(&node, graph, &mut sorted, &mut unvisited, &mut current) {
            let mut cycle = Vec::new();
            let last_window = [*current.last().unwrap(), current[0]];
            let mut windows = current
                .windows(2)
                .chain(std::iter::once(&last_window as &[usize]));
            while let Some(&[dependant, dependency]) = windows.next() {
                cycle.push((dependant, graph[&dependant][&dependency].clone()));
            }
            return Err(DependencyGraphError::GraphCycles(cycle));
        }
    }
    Ok(sorted)
}
