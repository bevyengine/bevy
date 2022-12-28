use bevy_utils::tracing::info;
use fixedbitset::FixedBitSet;

use crate::component::ComponentId;
use crate::schedule::{AmbiguityDetection, GraphNode, SystemContainer, SystemStage};
use crate::world::World;

use super::SystemLabelId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SystemOrderAmbiguity {
    segment: SystemStageSegment,
    // Note: In order for comparisons to work correctly,
    // `system_names` and `conflicts` must be sorted at all times.
    system_names: [String; 2],
    conflicts: Vec<String>,
}

/// Which part of a [`SystemStage`] was a [`SystemOrderAmbiguity`] detected in?
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
enum SystemStageSegment {
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
        use SystemStageSegment::*;

        let systems = match segment {
            Parallel => stage.parallel_systems(),
            ExclusiveAtStart => stage.exclusive_at_start_systems(),
            ExclusiveBeforeCommands => stage.exclusive_before_commands_systems(),
            ExclusiveAtEnd => stage.exclusive_at_end_systems(),
        };
        let mut system_names = [
            systems[system_a_index].name().to_string(),
            systems[system_b_index].name().to_string(),
        ];
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
    /// Logs execution order ambiguities between systems.
    ///
    /// The output may be incorrect if this stage has not been initialized with `world`.
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

                writeln!(string, " -- {system_a:?} and {system_b:?}").unwrap();

                if !conflicts.is_empty() {
                    writeln!(string, "    conflicts: {conflicts:?}").unwrap();
                }
            }

            info!("{}", string);
        }
    }

    /// Returns all execution order ambiguities between systems.
    ///
    /// Returns 4 vectors of ambiguities for each stage, in the following order:
    /// - parallel
    /// - exclusive at start,
    /// - exclusive before commands
    /// - exclusive at end
    ///
    /// The result may be incorrect if this stage has not been initialized with `world`.
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

        let mut ambiguities: Vec<_> = at_start
            .chain(parallel)
            .chain(before_commands)
            .chain(at_end)
            .collect();
        ambiguities.sort();
        ambiguities
    }

    /// Returns the number of system order ambiguities between systems in this stage.
    ///
    /// The result may be incorrect if this stage has not been initialized with `world`.
    #[cfg(test)]
    fn ambiguity_count(&self, world: &World) -> usize {
        self.ambiguities(world).len()
    }
}

/// Returns vector containing all pairs of indices of systems with ambiguous execution order,
/// along with specific components that have triggered the warning.
/// Systems must be topologically sorted beforehand.
fn find_ambiguities(systems: &[SystemContainer]) -> Vec<(usize, usize, Vec<ComponentId>)> {
    // Check if we should ignore ambiguities between `system_a` and `system_b`.
    fn should_ignore(system_a: &SystemContainer, system_b: &SystemContainer) -> bool {
        fn should_ignore_inner(
            system_a_detection: &AmbiguityDetection,
            system_b_labels: &[SystemLabelId],
        ) -> bool {
            match system_a_detection {
                AmbiguityDetection::Check => false,
                AmbiguityDetection::IgnoreAll => true,
                AmbiguityDetection::IgnoreWithLabel(labels) => {
                    labels.iter().any(|l| system_b_labels.contains(l))
                }
            }
        }
        should_ignore_inner(&system_a.ambiguity_detection, system_b.labels())
            || should_ignore_inner(&system_b.ambiguity_detection, system_a.labels())
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
    let all_relations = all_dependencies
        .into_iter()
        .zip(all_dependants.into_iter())
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
    for (index_a, relations) in all_relations.into_iter().enumerate() {
        // TODO: prove that `.take(index_a)` would be correct here, and uncomment it if so.
        for index_b in full_bitset.difference(&relations)
        // .take(index_a)
        {
            if !processed.contains(index_b) && !should_ignore(&systems[index_a], &systems[index_b])
            {
                let system_a = &systems[index_a];
                let system_b = &systems[index_b];
                if system_a.is_exclusive() || system_b.is_exclusive() {
                    ambiguities.push((index_a, index_b, Vec::new()));
                } else {
                    let a_access = systems[index_a].component_access();
                    let b_access = systems[index_b].component_access();
                    let conflicts = a_access.get_conflicts(b_access);
                    if !conflicts.is_empty() {
                        ambiguities.push((index_a, index_b, conflicts));
                    }
                }
            }
        }
        processed.insert(index_a);
    }
    ambiguities
}

#[cfg(test)]
mod tests {
    // Required to make the derive macro behave
    use crate as bevy_ecs;
    use crate::event::Events;
    use crate::prelude::*;

    #[derive(Resource)]
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
    fn with_filtered_component_system(_query: Query<&mut A, With<B>>) {}
    fn without_filtered_component_system(_query: Query<&mut A, Without<B>>) {}
    fn event_reader_system(_reader: EventReader<E>) {}
    fn event_writer_system(_writer: EventWriter<E>) {}
    fn event_resource_system(_events: ResMut<Events<E>>) {}
    fn read_world_system(_world: &World) {}
    fn write_world_system(_world: &mut World) {}

    // Tests for conflict detection

    #[test]
    fn one_of_everything() {
        let mut world = World::new();
        world.insert_resource(R);
        world.spawn(A);
        world.init_resource::<Events<E>>();

        let mut test_stage = SystemStage::parallel();
        test_stage
            // nonsendmut system deliberately conflicts with resmut system
            .add_system(resmut_system)
            .add_system(write_component_system)
            .add_system(event_writer_system);

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 0);
    }

    #[test]
    fn read_only() {
        let mut world = World::new();
        world.insert_resource(R);
        world.spawn(A);
        world.init_resource::<Events<E>>();

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
            .add_system(event_reader_system)
            .add_system(read_world_system)
            .add_system(read_world_system);

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 0);
    }

    #[test]
    fn read_world() {
        let mut world = World::new();
        world.insert_resource(R);
        world.spawn(A);
        world.init_resource::<Events<E>>();

        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(resmut_system)
            .add_system(write_component_system)
            .add_system(event_writer_system)
            .add_system(read_world_system);

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 3);
    }

    #[test]
    fn resources() {
        let mut world = World::new();
        world.insert_resource(R);

        let mut test_stage = SystemStage::parallel();
        test_stage.add_system(resmut_system).add_system(res_system);

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 1);
    }

    #[test]
    fn nonsend() {
        let mut world = World::new();
        world.insert_resource(R);

        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(nonsendmut_system)
            .add_system(nonsend_system);

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 1);
    }

    #[test]
    fn components() {
        let mut world = World::new();
        world.spawn(A);

        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(read_component_system)
            .add_system(write_component_system);

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 1);
    }

    #[test]
    #[ignore = "Known failing but fix is non-trivial: https://github.com/bevyengine/bevy/issues/4381"]
    fn filtered_components() {
        let mut world = World::new();
        world.spawn(A);

        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(with_filtered_component_system)
            .add_system(without_filtered_component_system);

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 0);
    }

    #[test]
    fn events() {
        let mut world = World::new();
        world.init_resource::<Events<E>>();

        let mut test_stage = SystemStage::parallel();
        test_stage
            // All of these systems clash
            .add_system(event_reader_system)
            .add_system(event_writer_system)
            .add_system(event_resource_system);

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 3);
    }

    #[test]
    fn exclusive() {
        let mut world = World::new();
        world.insert_resource(R);
        world.spawn(A);
        world.init_resource::<Events<E>>();

        let mut test_stage = SystemStage::parallel();
        test_stage
            // All 3 of these conflict with each other
            .add_system(write_world_system)
            .add_system(write_world_system.at_end())
            .add_system(res_system.at_start())
            // These do not, as they're in different segments of the stage
            .add_system(write_world_system.at_start())
            .add_system(write_world_system.before_commands());

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 3);
    }

    // Tests for silencing and resolving ambiguities

    #[test]
    fn before_and_after() {
        let mut world = World::new();
        world.init_resource::<Events<E>>();

        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(event_reader_system.before(event_writer_system))
            .add_system(event_writer_system)
            .add_system(event_resource_system.after(event_writer_system));

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 0);
    }

    #[test]
    fn ignore_all_ambiguities() {
        let mut world = World::new();
        world.insert_resource(R);

        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(resmut_system.ignore_all_ambiguities())
            .add_system(res_system)
            .add_system(nonsend_system);

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 0);
    }

    #[test]
    fn ambiguous_with_label() {
        let mut world = World::new();
        world.insert_resource(R);

        #[derive(SystemLabel)]
        struct IgnoreMe;

        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(resmut_system.ambiguous_with(IgnoreMe))
            .add_system(res_system.label(IgnoreMe))
            .add_system(nonsend_system.label(IgnoreMe));

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 0);
    }

    #[test]
    fn ambiguous_with_system() {
        let mut world = World::new();

        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(write_component_system.ambiguous_with(read_component_system))
            .add_system(read_component_system);

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 0);
    }

    fn system_a(_res: ResMut<R>) {}
    fn system_b(_res: ResMut<R>) {}
    fn system_c(_res: ResMut<R>) {}
    fn system_d(_res: ResMut<R>) {}
    fn system_e(_res: ResMut<R>) {}

    // Tests that the correct ambiguities were reported in the correct order.
    #[test]
    fn correct_ambiguities() {
        use super::*;

        let mut world = World::new();
        world.insert_resource(R);

        let mut test_stage = SystemStage::parallel();
        test_stage
            .add_system(system_a)
            .add_system(system_b)
            .add_system(system_c.ignore_all_ambiguities())
            .add_system(system_d.ambiguous_with(system_b))
            .add_system(system_e.after(system_a));

        test_stage.run(&mut world);

        let ambiguities = test_stage.ambiguities(&world);
        assert_eq!(
            ambiguities,
            vec![
                SystemOrderAmbiguity {
                    system_names: [
                        "bevy_ecs::schedule::ambiguity_detection::tests::system_a".to_string(),
                        "bevy_ecs::schedule::ambiguity_detection::tests::system_b".to_string()
                    ],
                    conflicts: vec!["bevy_ecs::schedule::ambiguity_detection::tests::R".to_string()],
                    segment: SystemStageSegment::Parallel,
                },
                SystemOrderAmbiguity {
                    system_names: [
                        "bevy_ecs::schedule::ambiguity_detection::tests::system_a".to_string(),
                        "bevy_ecs::schedule::ambiguity_detection::tests::system_d".to_string()
                    ],
                    conflicts: vec!["bevy_ecs::schedule::ambiguity_detection::tests::R".to_string()],
                    segment: SystemStageSegment::Parallel,
                },
                SystemOrderAmbiguity {
                    system_names: [
                        "bevy_ecs::schedule::ambiguity_detection::tests::system_b".to_string(),
                        "bevy_ecs::schedule::ambiguity_detection::tests::system_e".to_string()
                    ],
                    conflicts: vec!["bevy_ecs::schedule::ambiguity_detection::tests::R".to_string()],
                    segment: SystemStageSegment::Parallel,
                },
                SystemOrderAmbiguity {
                    system_names: [
                        "bevy_ecs::schedule::ambiguity_detection::tests::system_d".to_string(),
                        "bevy_ecs::schedule::ambiguity_detection::tests::system_e".to_string()
                    ],
                    conflicts: vec!["bevy_ecs::schedule::ambiguity_detection::tests::R".to_string()],
                    segment: SystemStageSegment::Parallel,
                },
            ]
        );
    }
}
