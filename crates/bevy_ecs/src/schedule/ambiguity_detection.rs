use bevy_utils::tracing::info;
use fixedbitset::FixedBitSet;

use crate::component::ComponentId;
use crate::schedule::{SystemContainer, SystemStage};
use crate::world::World;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SystemOrderAmbiguity {
    // Note: In order for comparisons to work correctly,
    // `system_names` and `conflicts` must be sorted at all times.
    system_names: [String; 2],
    conflicts: Vec<String>,
    pub segment: SystemStageSegment,
}

/// Which part of a [`SystemStage`] was a [`SystemOrderAmbiguity`] detected in?
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum SystemStageSegment {
    Parallel,
    ExclusiveAtStart,
    ExclusiveBeforeCommands,
    ExclusiveAtEnd,
}

impl SystemStageSegment {
    pub fn desc(&self) -> &'static str {
        match self {
            SystemStageSegment::Parallel => "Parallel systems",
            SystemStageSegment::ExclusiveAtStart => "Exclusive systems at start of stage",
            SystemStageSegment::ExclusiveBeforeCommands => {
                "Exclusive systems before commands of stage"
            }
            SystemStageSegment::ExclusiveAtEnd => "Exclusive systems at end of stage",
        }
    }
}

impl SystemOrderAmbiguity {
    fn from_raw(
        system_a_index: usize,
        system_b_index: usize,
        component_ids: Vec<ComponentId>,
        segment: SystemStageSegment,
        stage: &SystemStage,
        world: &World,
    ) -> Self {
        use crate::schedule::graph_utils::GraphNode;
        use SystemStageSegment::*;

        // TODO: blocked on https://github.com/bevyengine/bevy/pull/4166
        // We can't grab the system container generically, because .parallel_systems()
        // and the exclusive equivalent return a different type,
        // and SystemContainer is not object-safe
        let (system_a_name, system_b_name) = match segment {
            Parallel => {
                let system_container = stage.parallel_systems();
                (
                    system_container[system_a_index].name(),
                    system_container[system_b_index].name(),
                )
            }
            ExclusiveAtStart => {
                let system_container = stage.exclusive_at_start_systems();
                (
                    system_container[system_a_index].name(),
                    system_container[system_b_index].name(),
                )
            }
            ExclusiveBeforeCommands => {
                let system_container = stage.exclusive_before_commands_systems();
                (
                    system_container[system_a_index].name(),
                    system_container[system_b_index].name(),
                )
            }
            ExclusiveAtEnd => {
                let system_container = stage.exclusive_at_end_systems();
                (
                    system_container[system_a_index].name(),
                    system_container[system_b_index].name(),
                )
            }
        };

        let mut system_names = [system_a_name.to_string(), system_b_name.to_string()];
        system_names.sort();

        let mut conflicts: Vec<_> = component_ids
            .iter()
            .map(|id| world.components().get_info(*id).unwrap().name().to_owned())
            .collect();
        conflicts.sort();

        Self {
            system_names,
            conflicts,
            segment,
        }
    }
}

impl SystemStage {
    /// Logs execution order ambiguities between systems. System orders must be fresh.
    pub fn report_ambiguities(&self, world: &World) {
        debug_assert!(!self.systems_modified);
        use std::fmt::Write;
        let ambiguities = self.ambiguities(world);
        if !ambiguities.is_empty() {
            let mut string = "Execution order ambiguities detected, you might want to \
						add an explicit dependency relation between some of these systems:\n"
                .to_owned();

            let mut last_segment_kind = None;

            for SystemOrderAmbiguity {
                system_names: [system_a, system_b],
                conflicts,
                segment,
            } in &ambiguities
            {
                // If the ambiguity occurred in a different segment than the previous one, write a header for the segment.
                if last_segment_kind != Some(segment) {
                    writeln!(string, " * {}:", segment.desc()).unwrap();
                    last_segment_kind = Some(segment);
                }

                writeln!(string, " -- {:?} and {:?}", system_a, system_b).unwrap();

                if !conflicts.is_empty() {
                    writeln!(string, "    conflicts: {conflicts:?}").unwrap();
                }
            }

            info!("{}", string);
        }
    }

    fn ambiguities(&self, world: &World) -> Vec<SystemOrderAmbiguity> {
        let parallel = find_ambiguities(&self.parallel).into_iter().map(
            |(system_a_index, system_b_index, component_ids)| {
                SystemOrderAmbiguity::from_raw(
                    system_a_index,
                    system_b_index,
                    component_ids.to_vec(),
                    SystemStageSegment::Parallel,
                    self,
                    world,
                )
            },
        );

        let at_start = find_ambiguities(&self.exclusive_at_start).into_iter().map(
            |(system_a_index, system_b_index, component_ids)| {
                SystemOrderAmbiguity::from_raw(
                    system_a_index,
                    system_b_index,
                    component_ids,
                    SystemStageSegment::ExclusiveAtStart,
                    self,
                    world,
                )
            },
        );

        let before_commands = find_ambiguities(&self.exclusive_before_commands)
            .into_iter()
            .map(|(system_a_index, system_b_index, component_ids)| {
                SystemOrderAmbiguity::from_raw(
                    system_a_index,
                    system_b_index,
                    component_ids,
                    SystemStageSegment::ExclusiveBeforeCommands,
                    self,
                    world,
                )
            });

        let at_end = find_ambiguities(&self.exclusive_at_end).into_iter().map(
            |(system_a_index, system_b_index, component_ids)| {
                SystemOrderAmbiguity::from_raw(
                    system_a_index,
                    system_b_index,
                    component_ids,
                    SystemStageSegment::ExclusiveAtEnd,
                    self,
                    world,
                )
            },
        );

        at_start
            .chain(parallel)
            .chain(before_commands)
            .chain(at_end)
            .collect()
    }
}

/// Returns vector containing all pairs of indices of systems with ambiguous execution order,
/// along with specific components that have triggered the warning.
/// Systems must be topologically sorted beforehand.
fn find_ambiguities(systems: &[impl SystemContainer]) -> Vec<(usize, usize, Vec<ComponentId>)> {
    let mut all_dependencies = Vec::<FixedBitSet>::with_capacity(systems.len());
    let mut all_dependants = Vec::<FixedBitSet>::with_capacity(systems.len());
    for (index, container) in systems.iter().enumerate() {
        let mut dependencies = FixedBitSet::with_capacity(systems.len());
        for &dependency in container.dependencies() {
            dependencies.union_with(&all_dependencies[dependency]);
            dependencies.insert(dependency);
            all_dependants[dependency].insert(index);
        }

        all_dependants.push(FixedBitSet::with_capacity(systems.len()));
        all_dependencies.push(dependencies);
    }
    for index in (0..systems.len()).rev() {
        let mut dependants = FixedBitSet::with_capacity(systems.len());
        for dependant in all_dependants[index].ones() {
            dependants.union_with(&all_dependants[dependant]);
            dependants.insert(dependant);
        }
        all_dependants[index] = dependants;
    }
    let mut all_relations = all_dependencies
        .drain(..)
        .zip(all_dependants.drain(..))
        .enumerate()
        .map(|(index, (dependencies, dependants))| {
            let mut relations = FixedBitSet::with_capacity(systems.len());
            relations.union_with(&dependencies);
            relations.union_with(&dependants);
            relations.insert(index);
            relations
        })
        .collect::<Vec<FixedBitSet>>();
    let mut ambiguities = Vec::new();
    let full_bitset: FixedBitSet = (0..systems.len()).collect();
    let mut processed = FixedBitSet::with_capacity(systems.len());
    for (index_a, relations) in all_relations.drain(..).enumerate() {
        // TODO: prove that `.take(index_a)` would be correct here, and uncomment it if so.
        for index_b in full_bitset.difference(&relations)
        // .take(index_a)
        {
            if !processed.contains(index_b) {
                let a_access = systems[index_a].component_access();
                let b_access = systems[index_b].component_access();
                if let (Some(a), Some(b)) = (a_access, b_access) {
                    let conflicts = a.get_conflicts(b);
                    if !conflicts.is_empty() {
                        ambiguities.push((index_a, index_b, conflicts));
                    }
                } else {
                    ambiguities.push((index_a, index_b, Vec::new()));
                }
            }
        }
        processed.insert(index_a);
    }
    ambiguities
}
