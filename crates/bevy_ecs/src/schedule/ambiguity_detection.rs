use crate::component::ComponentId;
use crate::schedule::{AmbiguityDetection, SystemContainer, SystemStage};
use crate::world::World;

use fixedbitset::FixedBitSet;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AmbiguityReportLevel {
    Off,
    Minimal,
    Verbose,
}

/// Systems that access the same Component or Resource within the same stage
/// risk an ambiguous order that could result in logic bugs, unless they have an
/// explicit execution ordering constraint between them.
///
/// This occurs because, in the absence of explicit constraints, systems are executed in
/// an unstable, arbitrary order within each stage that may vary between runs and frames.
///
/// Some ambiguities reported by the ambiguity checker may be warranted (to allow two systems to run
/// without blocking each other) or spurious, as the exact combination of archetypes used may
/// prevent them from ever conflicting during actual gameplay. You can resolve the warnings produced
/// by the ambiguity checker by adding `.before` or `.after` to one of the conflicting systems
/// referencing the other system to force a specific ordering.
///
/// The checker may report a system more times than the amount of constraints it would actually need
/// to have unambiguous order with regards to a group of already-constrained systems.
///
/// By default only a warning with the number of unresolved ambiguities detected will be reported per [`SystemStage`].
/// This behavior can be changed by explicitly adding this resource using the following constructors:
/// * [`ReportExecutionOrderAmbiguities::off()`] - Disables all messages reported by the ambiguity checker.
/// * [`ReportExecutionOrderAmbiguities::minimal()`] - Displays only the number of unresolved ambiguities detected by the ambiguity checker.
/// * [`ReportExecutionOrderAmbiguities::verbose()`] - Displays a full report of ambiguities detected by the ambiguity checker.
///
/// The ambiguity checker will ignore ambiguities within official Bevy crates.
/// To ignore a custom crate, use [`ReportExecutionOrderAmbiguities::ignore`]
/// with an list of crate names as an argument.
/// This resource should be added before any bevy internal plugin.
///
/// ## Example
/// ```ignore
/// # use bevy_app::App;
/// # use bevy_ecs::schedule::ReportExecutionOrderAmbiguities;
/// App::new()
///    .insert_resource(ReportExecutionOrderAmbiguities::verbose().ignore(&["my_external_crate"]));
/// ```
pub struct ReportExecutionOrderAmbiguities {
    pub level: AmbiguityReportLevel,
    pub ignore_crates: Vec<String>,
}

/// Returns vector containing all pairs of indices of systems with ambiguous execution order,
/// along with specific components that have triggered the warning.
/// Systems must be topologically sorted beforehand.
pub(super) fn find_ambiguities(
    systems: &[impl SystemContainer],
    crates_filter: &[String],
) -> Vec<(usize, usize, Vec<ComponentId>)> {
    fn should_ignore_ambiguity(
        systems: &[impl SystemContainer],
        index_a: usize,
        index_b: usize,
        crates_filter: &[String],
    ) -> bool {
        let system_a = &systems[index_a];
        let system_b = &systems[index_b];

        (match system_a.ambiguity_detection() {
            AmbiguityDetection::Ignore => true,
            AmbiguityDetection::Check => false,
            AmbiguityDetection::IgnoreWithLabel(labels) => {
                labels.iter().any(|l| system_b.labels().contains(l))
            }
        }) || (match system_b.ambiguity_detection() {
            AmbiguityDetection::Ignore => true,
            AmbiguityDetection::Check => false,
            AmbiguityDetection::IgnoreWithLabel(labels) => {
                labels.iter().any(|l| system_a.labels().contains(l))
            }
        }) || (crates_filter.iter().any(|s| system_a.name().starts_with(s))
            && crates_filter.iter().any(|s| system_b.name().starts_with(s)))
    }

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
            if !processed.contains(index_b)
                && !should_ignore_ambiguity(systems, index_a, index_b, crates_filter)
            {
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

impl ReportExecutionOrderAmbiguities {
    /// Disables all messages reported by the ambiguity checker.
    pub fn off() -> Self {
        Self {
            level: AmbiguityReportLevel::Off,
            ..Default::default()
        }
    }

    /// Displays only the number of unresolved ambiguities detected by the ambiguity checker. This is the default behavior.
    pub fn minimal() -> Self {
        Self {
            level: AmbiguityReportLevel::Minimal,
            ..Default::default()
        }
    }

    /// Displays a full report of ambiguities detected by the ambiguity checker.
    pub fn verbose() -> Self {
        Self {
            level: AmbiguityReportLevel::Verbose,
            ..Default::default()
        }
    }

    /// Adds the given crate to be ignored by ambiguity checker. Check [`ReportExecutionOrderAmbiguities`] for more details.
    pub fn ignore(mut self, crate_prefix: &str) -> Self {
        self.ignore_crates.push(crate_prefix.to_string());
        self
    }

    /// Adds all the given crates to be ignored by ambiguity checker. Check [`ReportExecutionOrderAmbiguities`] for more details.
    pub fn ignore_all(mut self, crate_prefixes: &[&str]) -> Self {
        for s in crate_prefixes {
            self.ignore_crates.push(s.to_string());
        }
        self
    }
}

impl Default for ReportExecutionOrderAmbiguities {
    fn default() -> Self {
        Self {
            level: AmbiguityReportLevel::Minimal,
            ignore_crates: vec![],
        }
    }
}

impl SystemStage {
    /// Logs execution order ambiguities between systems. System orders must be fresh.
    pub fn report_ambiguities(&self, world: &mut World) {
        let ambiguity_report =
            world.get_resource_or_insert_with(ReportExecutionOrderAmbiguities::default);

        if ambiguity_report.level == AmbiguityReportLevel::Off {
            return;
        }

        debug_assert!(!self.systems_modified);

        fn write_display_names_of_pairs(
            offset: usize,
            systems: &[impl SystemContainer],
            ambiguities: Vec<(usize, usize, Vec<ComponentId>)>,
            world: &World,
        ) -> usize {
            for (i, (system_a_index, system_b_index, conflicting_indexes)) in
                ambiguities.iter().enumerate()
            {
                let _system_a_name = systems[*system_a_index].name();
                let _system_b_name = systems[*system_b_index].name();

                let _conflicting_components = conflicting_indexes
                    .iter()
                    .map(|id| world.components().get_info(*id).unwrap().name())
                    .collect::<Vec<_>>();

                let _ambiguity_number = i + offset;

                println!(
						"{_ambiguity_number}. {_system_a_name} conflicts with {_system_b_name} on {_conflicting_components:?}"
					);
            }

            // We can't merge the SystemContainer arrays, so instead we manually keep track of how high we've counted :upside_down:
            ambiguities.len()
        }

        let parallel = find_ambiguities(&self.parallel, &ambiguity_report.ignore_crates);
        let at_start = find_ambiguities(&self.exclusive_at_start, &ambiguity_report.ignore_crates);
        let before_commands = find_ambiguities(
            &self.exclusive_before_commands,
            &ambiguity_report.ignore_crates,
        );
        let at_end = find_ambiguities(&self.exclusive_at_end, &ambiguity_report.ignore_crates);

        let mut unresolved_count = parallel.len();
        unresolved_count += at_start.len();
        unresolved_count += before_commands.len();
        unresolved_count += at_end.len();

        if unresolved_count > 0 {
            println!("One of your stages contains {unresolved_count} pairs of systems with unknown order and conflicting data access. \
				You may want to add `.before()` or `.after()` constraints between some of these systems to prevent bugs.\n");

            if ambiguity_report.level != AmbiguityReportLevel::Verbose {
                println!("Set the level of the `ReportExecutionOrderAmbiguities` resource to `AmbiguityReportLevel::Verbose` for more details.");
            } else {
                // TODO: clean up this logic once exclusive systems are more compatible with parallel systems
                // allowing us to merge these collections
                let mut offset = 1;
                offset = write_display_names_of_pairs(offset, &self.parallel, parallel, world);
                offset =
                    write_display_names_of_pairs(offset, &self.exclusive_at_start, at_start, world);
                offset = write_display_names_of_pairs(
                    offset,
                    &self.exclusive_before_commands,
                    before_commands,
                    world,
                );
                write_display_names_of_pairs(offset, &self.exclusive_at_end, at_end, world);
            }
        }
    }
}
