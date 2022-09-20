use bevy_utils::tracing::info;
use fixedbitset::FixedBitSet;

use crate::component::ComponentId;
use crate::schedule::{SystemContainer, SystemStage};
use crate::world::World;

impl SystemStage {
    /// Logs execution order ambiguities between systems. System orders must be fresh.
    pub fn report_ambiguities(&self, world: &World) {
        debug_assert!(!self.systems_modified);
        use std::fmt::Write;
        fn write_display_names_of_pairs(
            string: &mut String,
            systems: &[impl SystemContainer],
            mut ambiguities: Vec<(usize, usize, Vec<ComponentId>)>,
            world: &World,
        ) {
            for (index_a, index_b, conflicts) in ambiguities.drain(..) {
                writeln!(
                    string,
                    " -- {:?} and {:?}",
                    systems[index_a].name(),
                    systems[index_b].name()
                )
                .unwrap();
                if !conflicts.is_empty() {
                    let names = conflicts
                        .iter()
                        .map(|id| world.components().get_info(*id).unwrap().name())
                        .collect::<Vec<_>>();
                    writeln!(string, "    conflicts: {:?}", names).unwrap();
                }
            }
        }
        let parallel = find_ambiguities(&self.parallel);
        let at_start = find_ambiguities(&self.exclusive_at_start);
        let before_commands = find_ambiguities(&self.exclusive_before_commands);
        let at_end = find_ambiguities(&self.exclusive_at_end);
        if !(parallel.is_empty()
            && at_start.is_empty()
            && before_commands.is_empty()
            && at_end.is_empty())
        {
            let mut string = "Execution order ambiguities detected, you might want to \
						add an explicit dependency relation between some of these systems:\n"
                .to_owned();
            if !parallel.is_empty() {
                writeln!(string, " * Parallel systems:").unwrap();
                write_display_names_of_pairs(&mut string, &self.parallel, parallel, world);
            }
            if !at_start.is_empty() {
                writeln!(string, " * Exclusive systems at start of stage:").unwrap();
                write_display_names_of_pairs(
                    &mut string,
                    &self.exclusive_at_start,
                    at_start,
                    world,
                );
            }
            if !before_commands.is_empty() {
                writeln!(string, " * Exclusive systems before commands of stage:").unwrap();
                write_display_names_of_pairs(
                    &mut string,
                    &self.exclusive_before_commands,
                    before_commands,
                    world,
                );
            }
            if !at_end.is_empty() {
                writeln!(string, " * Exclusive systems at end of stage:").unwrap();
                write_display_names_of_pairs(&mut string, &self.exclusive_at_end, at_end, world);
            }
            info!("{}", string);
        }
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
