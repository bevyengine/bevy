use alloc::{format, string::String, vec::Vec};
use core::fmt::Write as _;

use thiserror::Error;

use crate::{
    component::{ComponentId, Components},
    schedule::{graph::GraphNodeId, NodeId, ScheduleGraph, SystemKey, SystemSetKey},
    world::World,
};

/// Category of errors encountered during [`Schedule::initialize`](crate::schedule::Schedule::initialize).
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum ScheduleBuildError {
    /// A system set contains itself.
    #[error("System set `{0:?}` contains itself.")]
    HierarchyLoop(NodeId),
    /// The hierarchy of system sets contains a cycle.
    #[error("The hierarchy of system sets contains a cycle: {0:?}")]
    HierarchyCycle(Vec<Vec<NodeId>>),
    /// A system (set) has been told to run before itself.
    #[error("`{0:?}` has been told to run before itself.")]
    DependencyLoop(NodeId),
    /// The dependency graph contains a cycle.
    #[error("The dependency graph contains a cycle: {0:?}")]
    DependencyCycle(Vec<Vec<NodeId>>),
    /// Tried to order a system (set) relative to a system set it belongs to.
    #[error("`{0:?}` and `{1:?}` have both `in_set` and `before`-`after` relationships (these might be transitive). This combination is unsolvable as a system cannot run before or after a set it belongs to.")]
    CrossDependency(NodeId, NodeId),
    /// Tried to order system sets that share systems.
    #[error("`{0:?}` and `{1:?}` have a `before`-`after` relationship (which may be transitive) but share systems.")]
    SetsHaveOrderButIntersect(SystemSetKey, SystemSetKey),
    /// Tried to order a system (set) relative to all instances of some system function.
    #[error("Tried to order against `{0:?}` in a schedule that has more than one `{0:?}` instance. `{0:?}` is a `SystemTypeSet` and cannot be used for ordering if ambiguous. Use a different set without this restriction.")]
    SystemTypeSetAmbiguity(SystemSetKey),
    /// Tried to run a schedule before all of its systems have been initialized.
    #[error("Tried to run a schedule before all of its systems have been initialized.")]
    Uninitialized,
    /// A warning that was elevated to an error.
    #[error(transparent)]
    Elevated(#[from] ScheduleBuildWarning),
}

/// Category of warnings encountered during [`Schedule::initialize`](crate::schedule::Schedule::initialize).
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum ScheduleBuildWarning {
    /// The hierarchy of system sets contains redundant edges.
    ///
    /// This warning is **enabled** by default, but can be disabled by setting
    /// [`ScheduleBuildSettings::hierarchy_detection`] to [`LogLevel::Ignore`]
    /// or upgraded to a [`ScheduleBuildError`] by setting it to [`LogLevel::Error`].
    ///
    /// [`ScheduleBuildSettings::hierarchy_detection`]: crate::schedule::ScheduleBuildSettings::hierarchy_detection
    /// [`LogLevel::Ignore`]: crate::schedule::LogLevel::Ignore
    /// [`LogLevel::Error`]: crate::schedule::LogLevel::Error
    #[error("The hierarchy of system sets contains redundant edges: {0:?}")]
    HierarchyRedundancy(Vec<(NodeId, NodeId)>),
    /// Systems with conflicting access have indeterminate run order.
    ///
    /// This warning is **disabled** by default, but can be enabled by setting
    /// [`ScheduleBuildSettings::ambiguity_detection`] to [`LogLevel::Warn`]
    /// or upgraded to a [`ScheduleBuildError`] by setting it to [`LogLevel::Error`].
    ///
    /// [`ScheduleBuildSettings::ambiguity_detection`]: crate::schedule::ScheduleBuildSettings::ambiguity_detection
    /// [`LogLevel::Warn`]: crate::schedule::LogLevel::Warn
    /// [`LogLevel::Error`]: crate::schedule::LogLevel::Error
    #[error("Systems with conflicting access have indeterminate run order: {0:?}")]
    Ambiguity(Vec<(SystemKey, SystemKey, Vec<ComponentId>)>),
}

impl ScheduleBuildError {
    /// Renders the error as a human-readable string with node identifiers
    /// replaced with their names.
    ///
    /// The given `graph` and `world` are used to resolve the names of the nodes
    /// and components involved in the error. The same `graph` and `world`
    /// should be used as those used to [`initialize`] the [`Schedule`]. Failure
    /// to do so will result in incorrect or incomplete error messages.
    ///
    /// [`initialize`]: crate::schedule::Schedule::initialize
    /// [`Schedule`]: crate::schedule::Schedule
    pub fn to_string(&self, graph: &ScheduleGraph, world: &World) -> String {
        match self {
            ScheduleBuildError::HierarchyLoop(node_id) => {
                Self::hierarchy_loop_to_string(node_id, graph)
            }
            ScheduleBuildError::HierarchyCycle(cycles) => {
                Self::hierarchy_cycle_to_string(cycles, graph)
            }
            ScheduleBuildError::DependencyLoop(node_id) => {
                Self::dependency_loop_to_string(node_id, graph)
            }
            ScheduleBuildError::DependencyCycle(cycles) => {
                Self::dependency_cycle_to_string(cycles, graph)
            }
            ScheduleBuildError::CrossDependency(a, b) => {
                Self::cross_dependency_to_string(a, b, graph)
            }
            ScheduleBuildError::SetsHaveOrderButIntersect(a, b) => {
                Self::sets_have_order_but_intersect_to_string(a, b, graph)
            }
            ScheduleBuildError::SystemTypeSetAmbiguity(set) => {
                Self::system_type_set_ambiguity_to_string(set, graph)
            }
            ScheduleBuildError::Uninitialized => Self::uninitialized_to_string(),
            ScheduleBuildError::Elevated(e) => e.to_string(graph, world),
        }
    }

    fn hierarchy_loop_to_string(node_id: &NodeId, graph: &ScheduleGraph) -> String {
        format!(
            "{} `{}` contains itself",
            node_id.kind(),
            graph.get_node_name(node_id)
        )
    }

    fn hierarchy_cycle_to_string(cycles: &[Vec<NodeId>], graph: &ScheduleGraph) -> String {
        let mut message = format!("schedule has {} in_set cycle(s):\n", cycles.len());
        for (i, cycle) in cycles.iter().enumerate() {
            let mut names = cycle.iter().map(|id| (id.kind(), graph.get_node_name(id)));
            let (first_kind, first_name) = names.next().unwrap();
            writeln!(
                message,
                "cycle {}: {first_kind} `{first_name}` contains itself",
                i + 1,
            )
            .unwrap();
            writeln!(message, "{first_kind} `{first_name}`").unwrap();
            for (kind, name) in names.chain(core::iter::once((first_kind, first_name))) {
                writeln!(message, " ... which contains {kind} `{name}`").unwrap();
            }
            writeln!(message).unwrap();
        }
        message
    }

    fn hierarchy_redundancy_to_string(
        transitive_edges: &[(NodeId, NodeId)],
        graph: &ScheduleGraph,
    ) -> String {
        let mut message = String::from("hierarchy contains redundant edge(s)");
        for (parent, child) in transitive_edges {
            writeln!(
                message,
                " -- {} `{}` cannot be child of {} `{}`, longer path exists",
                child.kind(),
                graph.get_node_name(child),
                parent.kind(),
                graph.get_node_name(parent),
            )
            .unwrap();
        }
        message
    }

    fn dependency_loop_to_string(node_id: &NodeId, graph: &ScheduleGraph) -> String {
        format!(
            "{} `{}` has been told to run before itself",
            node_id.kind(),
            graph.get_node_name(node_id)
        )
    }

    fn dependency_cycle_to_string(cycles: &[Vec<NodeId>], graph: &ScheduleGraph) -> String {
        let mut message = format!("schedule has {} before/after cycle(s):\n", cycles.len());
        for (i, cycle) in cycles.iter().enumerate() {
            let mut names = cycle.iter().map(|id| (id.kind(), graph.get_node_name(id)));
            let (first_kind, first_name) = names.next().unwrap();
            writeln!(
                message,
                "cycle {}: {first_kind} `{first_name}` must run before itself",
                i + 1,
            )
            .unwrap();
            writeln!(message, "{first_kind} `{first_name}`").unwrap();
            for (kind, name) in names.chain(core::iter::once((first_kind, first_name))) {
                writeln!(message, " ... which must run before {kind} `{name}`").unwrap();
            }
            writeln!(message).unwrap();
        }
        message
    }

    fn cross_dependency_to_string(a: &NodeId, b: &NodeId, graph: &ScheduleGraph) -> String {
        format!(
            "{} `{}` and {} `{}` have both `in_set` and `before`-`after` relationships (these might be transitive). \
            This combination is unsolvable as a system cannot run before or after a set it belongs to.",
            a.kind(),
            graph.get_node_name(a),
            b.kind(),
            graph.get_node_name(b)
        )
    }

    fn sets_have_order_but_intersect_to_string(
        a: &SystemSetKey,
        b: &SystemSetKey,
        graph: &ScheduleGraph,
    ) -> String {
        format!(
            "`{}` and `{}` have a `before`-`after` relationship (which may be transitive) but share systems.",
            graph.get_node_name(&NodeId::Set(*a)),
            graph.get_node_name(&NodeId::Set(*b)),
        )
    }

    fn system_type_set_ambiguity_to_string(set: &SystemSetKey, graph: &ScheduleGraph) -> String {
        let name = graph.get_node_name(&NodeId::Set(*set));
        format!(
            "Tried to order against `{name}` in a schedule that has more than one `{name}` instance. `{name}` is a \
            `SystemTypeSet` and cannot be used for ordering if ambiguous. Use a different set without this restriction."
        )
    }

    pub(crate) fn ambiguity_to_string(
        ambiguities: &[(SystemKey, SystemKey, Vec<ComponentId>)],
        graph: &ScheduleGraph,
        components: &Components,
    ) -> String {
        let n_ambiguities = ambiguities.len();
        let mut message = format!(
            "{n_ambiguities} pairs of systems with conflicting data access have indeterminate execution order. \
            Consider adding `before`, `after`, or `ambiguous_with` relationships between these:\n",
        );
        let ambiguities = graph.conflicts_to_string(ambiguities, components);
        for (name_a, name_b, conflicts) in ambiguities {
            writeln!(message, " -- {name_a} and {name_b}").unwrap();

            if !conflicts.is_empty() {
                writeln!(message, "    conflict on: {conflicts:?}").unwrap();
            } else {
                // one or both systems must be exclusive
                let world = core::any::type_name::<World>();
                writeln!(message, "    conflict on: {world}").unwrap();
            }
        }
        message
    }

    fn uninitialized_to_string() -> String {
        String::from("tried to run a schedule before all of its systems have been initialized")
    }
}

impl ScheduleBuildWarning {
    /// Renders the warning as a human-readable string with node identifiers
    /// replaced with their names.
    pub fn to_string(&self, graph: &ScheduleGraph, world: &World) -> String {
        match self {
            ScheduleBuildWarning::HierarchyRedundancy(transitive_edges) => {
                ScheduleBuildError::hierarchy_redundancy_to_string(transitive_edges, graph)
            }
            ScheduleBuildWarning::Ambiguity(ambiguities) => {
                ScheduleBuildError::ambiguity_to_string(ambiguities, graph, world.components())
            }
        }
    }
}
