use crate::component::ComponentId;
use crate::schedule::{AmbiguityDetection, SystemContainer, SystemStage};
use crate::world::World;

use fixedbitset::FixedBitSet;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
/// By default, the value of this resource is set to `Minimal`.
///
/// ## Example
/// ```ignore
/// # use bevy_app::App;
/// # use bevy_ecs::schedule::ReportExecutionOrderAmbiguities;
/// App::new()
///    .insert_resource(ReportExecutionOrderAmbiguities::verbose().ignore(&["my_external_crate"]));
/// ```
pub enum ReportExecutionOrderAmbiguities {
    /// Disables all messages reported by the ambiguity checker
    Off,
    /// Displays only the number of unresolved ambiguities detected by the ambiguity checker
    Minimal,
    /// Displays a full report of ambiguities detected by the ambiguity checker
    Verbose,
    /// Verbosely reports all non-ignored ambiguities, including those between Bevy's systems
    ///
    /// These will not be actionable: you should only turn on this functionality when
    /// investigating to see if there's a Bevy bug or working on the engine itself.
    ReportInternal,
    /// Verbosely reports ALL ambiguities, even ignored ones
    ///
    /// This will be very noisy, but can be useful when attempting to track down subtle determinism issues,
    /// as you might need when attempting to implement lockstep networking.
    Deterministic,
}

/// A pair of systems that can run in an ambiguous order
///
/// Created by applying [`find_ambiguities`] to a [`SystemContainer`].
/// These can be reported by configuring the [`ReportExecutionOrderAmbiguities`] resource.
#[derive(Debug, Clone, PartialEq)]
struct SystemOrderAmbiguity {
    // The index of the first system in the [`SystemContainer`]
    pub system_a_index: usize,
    // The index of the second system in the [`SystemContainer`]
    pub system_b_index: usize,
    /// The components (and resources) that these systems have incompatible access to
    pub conflicts: Vec<ComponentId>,
}

/// Returns vector containing all pairs of indices of systems with ambiguous execution order,
/// along with specific components that have triggered the warning.
/// Systems must be topologically sorted beforehand.
fn find_ambiguities(
    systems: &[impl SystemContainer],
    crates_filter: &[String],
    // Should explicit attempts to ignore ambiguities be obeyed?
    report_level: ReportExecutionOrderAmbiguities,
) -> Vec<SystemOrderAmbiguity> {
    fn should_ignore_ambiguity(
        systems: &[impl SystemContainer],
        index_a: usize,
        index_b: usize,
        crates_filter: &[String],
        report_level: ReportExecutionOrderAmbiguities,
    ) -> bool {
        if report_level == ReportExecutionOrderAmbiguities::Deterministic {
            return false;
        }

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
                && !should_ignore_ambiguity(systems, index_a, index_b, crates_filter, report_level)
            {
                let a_access = systems[index_a].component_access();
                let b_access = systems[index_b].component_access();
                if let (Some(a), Some(b)) = (a_access, b_access) {
                    let conflicts = a.get_conflicts(b);
                    if !conflicts.is_empty() {
                        ambiguities.push(SystemOrderAmbiguity {
                            system_a_index: index_a,
                            system_b_index: index_b,
                            conflicts,
                        });
                    }
                } else {
                    ambiguities.push(SystemOrderAmbiguity {
                        system_a_index: index_a,
                        system_b_index: index_b,
                        conflicts: Vec::default(),
                    });
                }
            }
        }
        processed.insert(index_a);
    }
    ambiguities
}

impl Default for ReportExecutionOrderAmbiguities {
    fn default() -> Self {
        ReportExecutionOrderAmbiguities::Minimal
    }
}

impl SystemStage {
    /// Returns all execution order ambiguities between systems
    ///
    /// Returns 4 vectors of ambiguities for each stage, in the following order:
    /// - parallel
    /// - exclusive at start,
    /// - exclusive before commands
    /// - exclusive at end
    ///
    /// # Panics
    ///
    /// You must call [`SystemStage::initialize`] first or this method will panic.
    fn ambiguities(
        &self,
        report_level: ReportExecutionOrderAmbiguities,
    ) -> [Vec<SystemOrderAmbiguity>; 4] {
        if report_level == ReportExecutionOrderAmbiguities::Off {
            return [
                Vec::default(),
                Vec::default(),
                Vec::default(),
                Vec::default(),
            ];
        }

        // System order must be fresh
        debug_assert!(!self.systems_modified);

        // TODO: remove all internal ambiguities and remove this logic
        let ignored_crates = if report_level != ReportExecutionOrderAmbiguities::ReportInternal {
            vec![
                // Rendering
                "bevy_render".to_string(),
                "bevy_sprite".to_string(),
                "bevy_render".to_string(),
                "bevy_pbr".to_string(),
                "bevy_text".to_string(),
                "bevy_core_pipeline".to_string(),
                "bevy_ui".to_string(),
                "bevy_hierarchy".to_string(),
                // Misc
                "bevy_winit".to_string(),
                "bevy_audio".to_string(),
            ]
        } else {
            Vec::default()
        };

        let parallel = find_ambiguities(&self.parallel, &ignored_crates, report_level);
        let at_start = find_ambiguities(&self.exclusive_at_start, &ignored_crates, report_level);
        let before_commands = find_ambiguities(
            &self.exclusive_before_commands,
            &ignored_crates,
            report_level,
        );
        let at_end = find_ambiguities(&self.exclusive_at_end, &ignored_crates, report_level);

        [parallel, at_start, before_commands, at_end]
    }

    /// Returns the number of system order ambiguities between systems in this stage
    ///
    /// # Panics
    ///
    /// You must call [`SystemStage::initialize`] first or this method will panic.
    pub fn n_ambiguities(&self, report_level: ReportExecutionOrderAmbiguities) -> usize {
        let ambiguities = self.ambiguities(report_level);
        ambiguities.map(|vec| vec.len()).iter().sum()
    }

    /// Reports all execution order ambiguities between systems
    ///
    /// # Panics
    ///
    /// You must call [`SystemStage::initialize`] first or this method will panic.
    pub fn report_ambiguities(&self, world: &World, report_level: ReportExecutionOrderAmbiguities) {
        let [parallel, at_start, before_commands, at_end] = self.ambiguities(report_level);

        let mut unresolved_count = parallel.len();
        unresolved_count += at_start.len();
        unresolved_count += before_commands.len();
        unresolved_count += at_end.len();

        if unresolved_count > 0 {
            // Grammar
            if unresolved_count == 1 {
                println!("\n One of your stages contains 1 pair of systems with unknown order and conflicting data access. \
				You may want to add `.before()` or `.after()` constraints between some of these systems to prevent bugs.\n");
            } else {
                println!("\n One of your stages contains {unresolved_count} pairs of systems with unknown order and conflicting data access. \
				You may want to add `.before()` or `.after()` constraints between some of these systems to prevent bugs.\n");
            }

            if report_level == ReportExecutionOrderAmbiguities::Minimal {
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

fn write_display_names_of_pairs(
    offset: usize,
    systems: &[impl SystemContainer],
    ambiguities: Vec<SystemOrderAmbiguity>,
    world: &World,
) -> usize {
    for (i, system_order_ambiguity) in ambiguities.iter().enumerate() {
        let _system_a_name = systems[system_order_ambiguity.system_a_index].name();
        let _system_b_name = systems[system_order_ambiguity.system_b_index].name();

        let _conflicting_components = system_order_ambiguity
            .conflicts
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

// Systems and TestResource are used in tests
#[allow(dead_code)]
#[cfg(test)]
mod tests {
    // Required to make the derive macro behave
    use crate as bevy_ecs;
    use crate::event::Events;
    use crate::prelude::*;

    struct R;

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    // An event type
    struct E;

    fn empty_system() {}
    fn res_system(_res: Res<R>) {}
    fn resmut_system(_res: ResMut<R>) {}
    fn nonsend_system(_ns: NonSend<R>) {}
    fn nonsendmut_system(_ns: NonSendMut<R>) {}
    fn read_component_system(_query: Query<&A>) {}
    fn write_component_system(_query: Query<&mut A>) {}
    fn with_filtered_component_system(_query: Query<&A, With<B>>) {}
    fn without_filtered_component_system(_query: Query<&A, Without<B>>) {}
    fn event_reader_system(_reader: EventReader<E>) {}
    fn event_writer_system(_writer: EventWriter<E>) {}
    fn event_resource_system(_events: ResMut<Events<E>>) {}

    // Tests for conflict detection
    #[test]
    fn one_of_everything() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();

        test_stage
            .add_system(empty_system)
            // nonsendmut system deliberately conflicts with resmut system
            .add_system(resmut_system)
            .add_system(write_component_system)
            .add_system(event_writer_system);

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Deterministic),
            0
        );
    }

    #[test]
    fn read_only() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();

        test_stage
            .add_system(empty_system)
            .add_system(empty_system)
            .add_system(res_system)
            .add_system(res_system)
            .add_system(nonsend_system)
            .add_system(nonsend_system)
            .add_system(read_component_system)
            .add_system(read_component_system)
            .add_system(event_reader_system)
            .add_system(event_reader_system);

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Deterministic),
            0
        );
    }

    #[test]
    fn resources() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();

        test_stage.add_system(resmut_system).add_system(res_system);

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Deterministic),
            1
        );
    }

    #[test]
    fn nonsend() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();

        test_stage
            .add_system(nonsendmut_system)
            .add_system(nonsend_system);

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Deterministic),
            1
        );
    }

    #[test]
    fn components() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();

        test_stage
            .add_system(read_component_system)
            .add_system(write_component_system);

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Deterministic),
            1
        );
    }

    #[test]
    fn filtered_components() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();

        test_stage
            .add_system(with_filtered_component_system)
            .add_system(without_filtered_component_system);

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Deterministic),
            0
        );
    }

    #[test]
    fn events() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();

        test_stage
            .add_system(event_reader_system)
            .add_system(event_writer_system)
            .add_system(event_resource_system);

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Deterministic),
            // All of these systems clash
            3
        );
    }

    // Tests for silencing and resolving ambiguities
    #[test]
    fn before_and_after() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();

        test_stage
            .add_system(event_reader_system.before(event_writer_system))
            .add_system(event_writer_system)
            .add_system(event_resource_system.after(event_writer_system));

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Deterministic),
            // All of these systems clash
            0
        );
    }

    #[test]
    fn ignore_all_ambiguities() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(resmut_system.ignore_all_ambiguities())
            .add_system(res_system);

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Minimal),
            0
        );
    }

    #[test]
    fn ambiguous_with_label() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(resmut_system.ambiguous_with("IGNORE_ME"))
            .add_system(res_system.label("IGNORE_ME"));

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Minimal),
            0
        );
    }

    // Tests for reporting levels

    fn system_a(_res: ResMut<R>) {}
    fn system_b(_res: ResMut<R>) {}
    fn system_c(_res: ResMut<R>) {}
    fn system_d(_res: ResMut<R>) {}

    fn make_test_stage() -> SystemStage {
        let mut test_stage = SystemStage::parallel();
        let mut world = World::new();
        world.insert_resource(R);

        test_stage
            // Ambiguous with B and D
            .add_system(system_a)
            // Ambiguous with A
            .add_system(system_b.label("b"))
            .add_system(system_c.ignore_all_ambiguities())
            // Ambiguous with A
            .add_system(system_d.ambiguous_with("b"));

        // We need to ensure that the schedule has been properly initialized
        test_stage.initialize(&mut world);
        test_stage
    }

    #[test]
    fn ambiguous_with_system() {
        let mut world = World::new();
        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(system_a.ambiguous_with(system_b))
            .add_system(system_b);

        test_stage.initialize(&mut world);
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Minimal),
            0
        );
    }

    #[test]
    fn off() {
        let test_stage = make_test_stage();
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Off),
            0
        );
    }

    #[test]
    fn minimal() {
        let test_stage = make_test_stage();
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Minimal),
            2
        );
    }

    #[test]
    fn verbose() {
        let test_stage = make_test_stage();
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Verbose),
            2
        );
    }

    #[test]
    fn deterministic() {
        let test_stage = make_test_stage();
        assert_eq!(
            test_stage.n_ambiguities(ReportExecutionOrderAmbiguities::Deterministic),
            6
        );
    }

    /*
    // Tests that the correct ambiguities were reported
    #[test]
    fn correct_ambiguities() {
        use crate::component::ComponentId;
        use crate::schedule::SystemOrderAmbiguity;

        let test_stage = make_test_stage();
        let ambiguities = test_stage.ambiguities(ReportExecutionOrderAmbiguities::Verbose);
        assert_eq!(
            ambiguities,
            [
                // All ambiguities are in parallel systems
                // FIXME: this test is flaky due to the fact that the topological order is built nondeterministically
                vec![
                    SystemOrderAmbiguity {
                        system_a_index: 0,
                        system_b_index: 1,
                        conflicts: vec![ComponentId(0)]
                    },
                    SystemOrderAmbiguity {
                        system_a_index: 0,
                        system_b_index: 3,
                        conflicts: vec![ComponentId(0)]
                    }
                ],
                // Nothing in exclusive-at-start
                Vec::default(),
                // Nothing in exclusive-before-commands
                Vec::default(),
                // Nothing in exclusive-at-end
                Vec::default()
            ]
        );
    }
    */
}
