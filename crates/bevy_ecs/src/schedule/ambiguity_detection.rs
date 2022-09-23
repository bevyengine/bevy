use bevy_utils::tracing::info;
use bevy_utils::HashMap;
use fixedbitset::FixedBitSet;

use crate::component::ComponentId;
use crate::schedule::{SystemContainer, SystemStage};
use crate::world::World;

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

            for (i, ambiguity) in ambiguities.iter().enumerate() {
                let SystemOrderAmbiguity {
                    segment,
                    conflicts,
                    system_names,
                    ..
                } = ambiguity;

                writeln!(string).unwrap();
                writeln!(
                    string,
                    "({i}) Ambiguous system ordering - {}",
                    segment.desc()
                )
                .unwrap();

                for name in system_names {
                    writeln!(string, " * {name}").unwrap();
                }

                writeln!(string).unwrap();
                writeln!(string, " Data access conflicts:").unwrap();
                for conflict in conflicts {
                    writeln!(string, " * {conflict}").unwrap();
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
        fn find_ambiguities_in<'a>(
            segment: SystemStageSegment,
            container: &'a [impl SystemContainer],
            world: &'a World,
        ) -> impl Iterator<Item = SystemOrderAmbiguity> + 'a {
            let info = AmbiguityInfo::delineate(find_ambiguities(container));
            info.into_iter().map(
                move |AmbiguityInfo {
                          conflicts, systems, ..
                      }| {
                    let conflicts = conflicts
                        .iter()
                        .map(|id| world.components().get_info(*id).unwrap().name().to_owned())
                        .collect();
                    let mut system_names: Vec<_> = systems
                        .iter()
                        .map(|&SystemIndex(i)| container[i].name().to_string())
                        .collect();
                    system_names.sort();
                    SystemOrderAmbiguity {
                        segment,
                        conflicts,
                        system_names,
                    }
                },
            )
        }

        find_ambiguities_in(SystemStageSegment::Parallel, &self.parallel, world)
            .chain(find_ambiguities_in(
                SystemStageSegment::ExclusiveAtStart,
                &self.exclusive_at_start,
                world,
            ))
            .chain(find_ambiguities_in(
                SystemStageSegment::ExclusiveBeforeCommands,
                &self.exclusive_before_commands,
                world,
            ))
            .chain(find_ambiguities_in(
                SystemStageSegment::ExclusiveAtEnd,
                &self.exclusive_at_end,
                world,
            ))
            .collect()
    }

    /// Returns the number of system order ambiguities between systems in this stage.
    ///
    /// The result may be incorrect if this stage has not been initialized with `world`.
    #[cfg(test)]
    fn ambiguity_count(&self, world: &World) -> usize {
        fn binomial_coefficient(n: usize, k: usize) -> usize {
            (0..k).fold(1, |x, i| x * (n - i) / (i + 1))
        }

        self.ambiguities(world)
            .iter()
            .map(|a| binomial_coefficient(a.system_names.len(), 2))
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SystemOrderAmbiguity {
    segment: SystemStageSegment,
    conflicts: Vec<String>,
    system_names: Vec<String>,
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

/// A set of systems that are all reported to be ambiguous with one another.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct AmbiguityInfo {
    // INVARIANT: `conflicts` is always sorted.
    conflicts: Vec<ComponentId>,
    systems: Vec<SystemIndex>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SystemIndex(usize);

impl AmbiguityInfo {
    fn delineate(
        pairs: impl IntoIterator<Item = (SystemIndex, SystemIndex, Vec<ComponentId>)>,
    ) -> Vec<Self> {
        // Stores the pairs of system indices associated with each set of conflicts.
        let mut pairs_by_conflicts = HashMap::new();
        for (system_a_index, system_b_index, mut conflicts) in pairs {
            conflicts.sort();
            pairs_by_conflicts
                .entry(conflicts)
                .or_insert_with(Vec::new)
                .push([system_a_index, system_b_index]);
        }

        let mut ambiguity_sets = Vec::new();
        for (conflicts, pairs) in pairs_by_conflicts {
            // Find all unique systems that have the same conflicts.
            // Note that this does *not* mean they all conflict with one another.
            let mut in_set: Vec<_> = pairs.iter().copied().flatten().collect();
            in_set.sort();
            in_set.dedup();

            // adjacency marix for the entries of `in_set`
            let mut adj: Vec<FixedBitSet> = (0..in_set.len())
                .map(|i| {
                    let mut bitset = FixedBitSet::with_capacity(in_set.len());
                    // enable the main diagonal
                    bitset.set(i, true);
                    bitset
                })
                .collect();
            // the value `pairs` mapped as indices in `in_set`.
            let mut pairs_as_indices = Vec::new();
            for &[a, b] in &pairs {
                let a_index = in_set.iter().position(|&i| i == a).unwrap();
                let b_index = in_set.iter().position(|&i| i == b).unwrap();
                pairs_as_indices.push([a_index, b_index]);
                adj[a_index].set(b_index, true);
                adj[b_index].set(a_index, true);
            }

            // Find sets of systems that are all ambiguous with one another.
            let mut subgraphs = Vec::new();
            for [a_index, b_index] in pairs_as_indices {
                let intersection: FixedBitSet = adj[a_index].intersection(&adj[b_index]).collect();
                // If this pair has been included in another set, skip it.
                if intersection.count_ones(..) <= 1 {
                    continue;
                }

                for i in intersection.ones() {
                    adj[i].difference_with(&intersection);
                    // don't unset the main diagonal
                    adj[i].set(i, true);
                }

                subgraphs.push(intersection);
            }

            for subgraph in subgraphs {
                ambiguity_sets.push(AmbiguityInfo {
                    conflicts: conflicts.clone(),
                    systems: subgraph.ones().map(|i| in_set[i]).collect(),
                });
            }
        }
        ambiguity_sets
    }
}

/// Returns vector containing all pairs of indices of systems with ambiguous execution order,
/// along with specific components that have triggered the warning.
/// Systems must be topologically sorted beforehand.
fn find_ambiguities(
    systems: &[impl SystemContainer],
) -> Vec<(SystemIndex, SystemIndex, Vec<ComponentId>)> {
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
                        ambiguities.push((SystemIndex(index_a), SystemIndex(index_b), conflicts));
                    }
                } else {
                    ambiguities.push((SystemIndex(index_a), SystemIndex(index_b), Vec::new()));
                }
            }
        }
        processed.insert(index_a);
    }
    ambiguities
}

#[cfg(test)]
mod tests {
    use super::*;
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
        world.spawn().insert(A);
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
        world.spawn().insert(A);
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
        world.spawn().insert(A);
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
        world.spawn().insert(A);

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
        world.spawn().insert(A);

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
        world.spawn().insert(A);
        world.init_resource::<Events<E>>();

        let mut test_stage = SystemStage::parallel();
        test_stage
            // All 3 of these conflict with each other (`.at_start()` is the default configuration)
            .add_system(write_world_system.exclusive_system())
            .add_system(write_world_system.exclusive_system().at_start())
            .add_system(res_system.exclusive_system())
            // These do not, as they're in different segments of the stage
            .add_system(write_world_system.exclusive_system().at_end())
            .add_system(write_world_system.exclusive_system().before_commands());

        test_stage.run(&mut world);

        assert_eq!(test_stage.ambiguity_count(&world), 3);
        assert_eq!(
            test_stage.ambiguities(&world),
            vec![SystemOrderAmbiguity {
                segment: SystemStageSegment::ExclusiveAtStart,
                conflicts: vec![],
                system_names: vec![
                    "bevy_ecs::schedule::ambiguity_detection::tests::res_system".to_owned(),
                    "bevy_ecs::schedule::ambiguity_detection::tests::write_world_system".to_owned(),
                    "bevy_ecs::schedule::ambiguity_detection::tests::write_world_system".to_owned(),
                ]
            }],
        );
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
}
