//! Utilities for serializing schedule data for an [`App`](bevy_app::App).
//!
//! These are mostly around providing types implementing [`Serialize`]/[`Deserialize`] that
//! represent schedule data. In addition, there are tools for extracting this data from the
//! [`World`](bevy_ecs::world::World).

use bevy_ecs::{
    component::{ComponentId, Components},
    schedule::{ApplyDeferred, ConditionWithAccess, NodeId, Schedule, Schedules},
    system::SystemStateFlags,
};
use bevy_platform::collections::{hash_map::Entry, HashMap};
use bevy_utils::prelude::DebugName;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The data for the entire app's schedule.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppData {
    /// A list of all schedules in the app.
    pub schedules: Vec<ScheduleData>,
}

impl AppData {
    /// Creates the data from the underlying [`Schedules`].
    ///
    /// Note: we assume all schedules in `schedules` have been initialized through
    /// [`Schedule::initialize`].
    pub fn from_schedules(
        schedules: &Schedules,
        world_components: &Components,
    ) -> Result<Self, ExtractAppDataError> {
        Ok(Self {
            schedules: schedules
                .iter()
                .map(|(_, schedule)| ScheduleData::from_schedule(schedule, world_components))
                .collect::<Result<_, ExtractAppDataError>>()?,
        })
    }
}

/// Data about a particular schedule.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduleData {
    /// The name of the schedule.
    pub name: String,
    /// The systems in this schedule.
    pub systems: Vec<SystemData>,
    /// The system sets in this schedule.
    pub system_sets: Vec<SystemSetData>,
    /// A list of relationships indicating that a system/system set is contained in a system set.
    ///
    /// The order is (parent, child).
    pub hierarchy: Vec<(u32, ScheduleIndex)>,
    /// A list of ordering constraints, ensuring that one system/system set runs before another.
    ///
    /// The order is (first, second).
    pub dependency: Vec<(ScheduleIndex, ScheduleIndex)>,
    /// The components that these systems access.
    pub components: Vec<ComponentData>,
    /// A list of conflicts between systems.
    pub conflicts: Vec<SystemConflict>,
}

/// Data about a component type.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ComponentData {
    /// The name of the component.
    pub name: String,
}

/// Data about a particular system.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SystemData {
    /// The name of the system.
    pub name: String,
    /// Whether this system is a sync point (aka [`ApplyDeferred`]).
    pub apply_deferred: bool,
    /// Whether this system is exclusive.
    pub exclusive: bool,
    /// Whether this system has deferred buffers to apply.
    pub deferred: bool,
    // TODO: Store the conditions specific to this system.
}

/// Data about a particular system set.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SystemSetData {
    /// The name of the system set.
    pub name: String,
    /// The conditions applied to this system.
    pub conditions: Vec<ConditionData>,
}

/// Data about a run condition for a system.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ConditionData {
    /// The name of the condition.
    pub name: String,
}

/// An index of an element in a schedule.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ScheduleIndex {
    /// The index of a system.
    System(u32),
    /// The index of a system set.
    SystemSet(u32),
}

/// Data about an access conflict between two systems.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SystemConflict {
    /// The first system index.
    pub system_1: u32,
    /// The second system index.
    pub system_2: u32,
    /// The indices of the components that both systems are conflicting on.
    ///
    /// If empty, the systems conflict on world access.
    pub conflicting_components: Vec<u32>,
}

impl ScheduleData {
    /// Creates the data from the underlying [`Schedule`].
    ///
    /// Note: we assume `schedule` has already been initialized.
    pub fn from_schedule(
        schedule: &Schedule,
        world_components: &Components,
    ) -> Result<Self, ExtractAppDataError> {
        let graph = schedule.graph();

        let mut system_key_to_index = HashMap::new();
        let mut system_set_key_to_index = HashMap::new();

        fn debug_name_string(debug_name: &DebugName) -> String {
            format!("{}", debug_name)
        }

        fn extract_condition_data(conditions: &[ConditionWithAccess]) -> Vec<ConditionData> {
            conditions
                .iter()
                .map(|condition| ConditionData {
                    name: debug_name_string(&condition.condition.name()),
                })
                .collect()
        }

        let systems = schedule
            .systems()
            .map_err(|_| {
                ExtractAppDataError::ScheduleNotInitialized(format!("{:?}", schedule.label()))
            })?
            .enumerate()
            .map(|(index, (key, system))| {
                system_key_to_index.insert(key, index);

                let flags = system.flags();

                SystemData {
                    name: debug_name_string(&system.name()),
                    apply_deferred: system.type_id() == core::any::TypeId::of::<ApplyDeferred>(),
                    exclusive: flags.contains(SystemStateFlags::EXCLUSIVE),
                    deferred: flags.contains(SystemStateFlags::DEFERRED),
                }
            })
            .collect();

        let system_sets = graph
            .system_sets
            .iter()
            .enumerate()
            .map(|(index, (key, system_set, conditions))| {
                system_set_key_to_index.insert(key, index);

                SystemSetData {
                    name: format!("{:?}", system_set),
                    conditions: extract_condition_data(conditions),
                }
            })
            .collect();

        let node_id_to_schedule_index = |node_id: NodeId| match node_id {
            NodeId::System(key) => ScheduleIndex::System(
                *system_key_to_index
                    .get(&key)
                    .expect("the system this key refers to should have already been seen")
                    as _,
            ),
            NodeId::Set(key) => ScheduleIndex::SystemSet(
                *system_set_key_to_index
                    .get(&key)
                    .expect("the system set this key refers to should have already been seen")
                    as _,
            ),
        };

        let hierarchy = graph
            .hierarchy()
            .graph()
            .all_edges()
            .map(|(parent, child)| {
                let parent = system_set_key_to_index
                    .get(
                        &parent
                            .as_set()
                            .expect("the parent of a system/set is always a set"),
                    )
                    .expect("the system set this key refers to should have already been seen");
                let child = node_id_to_schedule_index(child);

                (*parent as _, child)
            })
            .collect();

        let dependency = graph
            .dependency()
            .graph()
            .all_edges()
            .map(|(a, b)| (node_id_to_schedule_index(a), node_id_to_schedule_index(b)))
            .collect();

        let mut component_id_to_index = HashMap::<ComponentId, usize>::new();
        let mut components = vec![];

        let conflicts = graph
            .conflicting_systems()
            .iter()
            .map(|(system_1, system_2, conflicts)| {
                let system_1 = system_key_to_index
                    .get(system_1)
                    .expect("the system this key refers to should have already been seen");
                let system_2 = system_key_to_index
                    .get(system_2)
                    .expect("the system this key refers to should have already been seen");

                SystemConflict {
                    system_1: *system_1 as _,
                    system_2: *system_2 as _,
                    conflicting_components: conflicts
                        .iter()
                        .map(|id| match component_id_to_index.entry(*id) {
                            Entry::Occupied(entry) => *entry.get() as _,
                            Entry::Vacant(entry) => {
                                let component = world_components.get_info(*id).expect(
                                    "the component has already been registered by the system",
                                );

                                components.push(ComponentData {
                                    name: debug_name_string(&component.name()),
                                });
                                *entry.insert(components.len() - 1) as _
                            }
                        })
                        .collect(),
                }
            })
            .collect();

        Ok(Self {
            name: format!("{:?}", schedule.label()),
            components,
            systems,
            system_sets,
            hierarchy,
            dependency,
            conflicts,
        })
    }
}

/// An error occurring while attempting to extract schedule data from an app.
#[derive(Error, Debug)]
pub enum ExtractAppDataError {
    /// A schedule has not been initialized through [`Schedule::initialize`].
    #[error("executable schedule has not been created for label \"{0}\"")]
    ScheduleNotInitialized(String),
}

#[cfg(test)]
mod tests {
    use bevy_app::{App, Update};
    use bevy_ecs::{
        component::Component,
        query::{With, Without},
        schedule::{IntoScheduleConfigs, Schedules, SystemSet},
        system::{Commands, Query},
    };
    use bevy_platform::collections::HashMap;

    use crate::schedule_data::serde::{
        AppData, ComponentData, ExtractAppDataError, ScheduleData, ScheduleIndex, SystemConflict,
        SystemData, SystemSetData,
    };

    fn app_data_from_app(app: &mut App) -> Result<AppData, ExtractAppDataError> {
        let schedules = app.world_mut().resource::<Schedules>();
        // TODO: This is a pain. It would be nice to be able to just hokey-pokey the whole
        // `Schedules` resource, but initializing a schedule writes to `Schedules`. Also we need to
        // use interned labels since `Box<dyn ScheduleLabel>` doesn't impl `ScheduleLabel`!
        let interned_labels = schedules
            .iter()
            .map(|(_, schedule)| schedule.label())
            .collect::<Vec<_>>();

        for label in interned_labels {
            let mut schedule = app
                .world_mut()
                .resource_mut::<Schedules>()
                .remove(label)
                .expect("we just copied the label from this schedule");

            schedule.initialize(app.world_mut()).unwrap();

            app.world_mut().resource_mut::<Schedules>().insert(schedule);
        }

        let mut app_data = AppData::from_schedules(
            app.world().resource::<Schedules>(),
            app.world().components(),
        )?;

        // We don't want the names of items to include module paths for this test (otherwise moving
        // the test would change the output).
        for schedule in app_data.schedules.iter_mut() {
            for system in schedule.systems.iter_mut() {
                system.name = system.name.rsplit_once(":").unwrap().1.to_string();
            }
            for set in schedule.system_sets.iter_mut() {
                let name_modless = set
                    .name
                    .rsplit_once(":")
                    .map(|(_, suffix)| suffix)
                    .unwrap_or(set.name.as_str())
                    .to_string();
                if set.name.starts_with("SystemTypeSet") {
                    // This is a set corresponding to a system. Make sure to keep the
                    // `SystemTypeSet` prefix.
                    set.name = format!("SystemTypeSet:{name_modless}");
                } else {
                    set.name = name_modless;
                }
            }
            for component in schedule.components.iter_mut() {
                component.name = component.name.rsplit_once(":").unwrap().1.to_string();
            }
        }

        sort_app_data(&mut app_data);
        Ok(app_data)
    }

    /// Sorts the [`AppData`] so we have a deterministic order when asserting.
    // Note: we could do this when extracting unconditionally (even in prod), but there's not much
    // point since schedule order is not guaranteed to be deterministic anyway. So relying on the
    // same order seems weird.
    fn sort_app_data(app_data: &mut AppData) {
        // Sort schedules by name.
        app_data
            .schedules
            .sort_by_key(|schedule| schedule.name.clone());
        // Sort each schedule.
        app_data.schedules.iter_mut().for_each(sort_schedule);

        /// Sorts a schedule so that systems, system sets, conditions, and components are in name
        /// order, and other structures are in index order.
        fn sort_schedule(schedule: &mut ScheduleData) {
            /// Sorts the slice using `key_fn` and returns a mapping, which maps the original index
            /// to the new index.
            fn reorder_slice<T, K: Ord>(
                slice: &mut [T],
                key_fn: impl Fn(&T) -> K,
            ) -> HashMap<usize, usize> {
                let mut mapping = (0..slice.len()).collect::<Vec<_>>();
                // We assume the two sorts produce the same thing which should be true since we are
                // using a stable sort.
                mapping.sort_by_key(|index| key_fn(&slice[*index]));
                slice.sort_by_key(key_fn);

                mapping
                    .into_iter()
                    // Enumerating produces the new indices.
                    .enumerate()
                    // Flip the order of indices so that we go from old to new.
                    .map(|(new, old)| (old, new))
                    .collect()
            }

            let system_old_index_to_new_index =
                reorder_slice(&mut schedule.systems, |system| system.name.clone());
            let system_set_old_index_to_new_index =
                reorder_slice(&mut schedule.system_sets, |set| set.name.clone());
            let component_old_index_to_new_index =
                reorder_slice(&mut schedule.components, |component| component.name.clone());

            let reindex_system = |index: &mut u32| {
                *index = *system_old_index_to_new_index
                    .get(&(*index as usize))
                    .unwrap() as u32;
            };
            let reindex_system_set = |index: &mut u32| {
                *index = *system_set_old_index_to_new_index
                    .get(&(*index as usize))
                    .unwrap() as u32;
            };
            let reindex_schedule_index = |index: &mut ScheduleIndex| match index {
                ScheduleIndex::System(system) => reindex_system(system),
                ScheduleIndex::SystemSet(set) => reindex_system_set(set),
            };

            let reindex_component = |index: &mut u32| {
                *index = *component_old_index_to_new_index
                    .get(&(*index as usize))
                    .unwrap() as u32;
            };

            // Sort the conditions in a system set.
            for set in schedule.system_sets.iter_mut() {
                set.conditions
                    .sort_by_key(|condition| condition.name.clone());
            }

            // Reindex the hierarchy, and sort it.
            for (parent, child) in schedule.hierarchy.iter_mut() {
                reindex_system_set(parent);
                reindex_schedule_index(child);
            }
            schedule.hierarchy.sort();

            // Reindex the dependencies, and sort it.
            for (parent, child) in schedule.dependency.iter_mut() {
                reindex_schedule_index(parent);
                reindex_schedule_index(child);
            }
            schedule.dependency.sort();

            // Reindex the conflicts.
            for conflict in schedule.conflicts.iter_mut() {
                reindex_system(&mut conflict.system_1);
                reindex_system(&mut conflict.system_2);

                // The order of the indices don't matter, so pick the ordering such that `system_1 <
                // system_2`.
                if conflict.system_1 > conflict.system_2 {
                    core::mem::swap(&mut conflict.system_1, &mut conflict.system_2);
                }

                conflict
                    .conflicting_components
                    .iter_mut()
                    .for_each(reindex_component);
                conflict.conflicting_components.sort();
            }
            schedule
                .conflicts
                .sort_by_key(|conflict| (conflict.system_1, conflict.system_2));
        }
    }

    /// Convenience to create a [`SystemData`] for the common case of no flags set.
    fn simple_system(name: &str) -> SystemData {
        SystemData {
            name: name.into(),
            apply_deferred: false,
            exclusive: false,
            deferred: false,
        }
    }

    /// Convenience to create a [`SystemSetData`] for the common case of being empty.
    fn simple_system_set(name: &str) -> SystemSetData {
        SystemSetData {
            name: name.into(),
            conditions: vec![],
        }
    }

    /// Convenience to create a [`ComponentData`] to make test cases shorter.
    fn simple_component(name: &str) -> ComponentData {
        ComponentData { name: name.into() }
    }

    /// Convenience to create a [`SystemConflict`] to make test cases shorter.
    fn conflict(system_1: u32, system_2: u32, components: Vec<u32>) -> SystemConflict {
        SystemConflict {
            system_1,
            system_2,
            conflicting_components: components,
        }
    }

    /// A convenience system set that is generic allowing us to make many of these quickly.
    #[derive(SystemSet, Hash, PartialEq, Eq, Clone)]
    struct MySet<const NUM: u32>;

    impl<const NUM: u32> core::fmt::Debug for MySet<NUM> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "MySet<{NUM}>")
        }
    }

    /// A convenience component that is generic allowing us to make many of these quickly.
    #[derive(Component)]
    struct MyComponent<const NUM: u32>;

    #[test]
    fn linear() {
        let mut app = App::empty();

        fn a() {}
        fn b() {}
        fn c() {}

        app.add_systems(Update, (a, b, c).chain());

        let data = app_data_from_app(&mut app).unwrap();
        assert_eq!(data.schedules.len(), 1);
        let schedule = &data.schedules[0];
        assert_eq!(schedule.name, "Update");
        assert_eq!(
            schedule.systems,
            [simple_system("a"), simple_system("b"), simple_system("c"),]
        );
        // Each system is also a system set.
        assert_eq!(
            schedule.system_sets,
            [
                simple_system_set("SystemTypeSet:a"),
                simple_system_set("SystemTypeSet:b"),
                simple_system_set("SystemTypeSet:c"),
            ]
        );
        // Every system is in its own system set.
        assert_eq!(
            schedule.hierarchy,
            [
                (0, ScheduleIndex::System(0)),
                (1, ScheduleIndex::System(1)),
                (2, ScheduleIndex::System(2)),
            ]
        );
        // There are 2 dependency edges to connect a-b and b-c.
        assert_eq!(
            schedule.dependency,
            [
                (ScheduleIndex::System(0), ScheduleIndex::System(1)),
                (ScheduleIndex::System(1), ScheduleIndex::System(2)),
            ]
        );
        assert_eq!(schedule.components.len(), 0);
        assert_eq!(schedule.conflicts.len(), 0);
    }

    #[test]
    fn linear_with_system_sets() {
        let mut app = App::empty();

        app.configure_sets(Update, (MySet::<0>, MySet::<1>, MySet::<2>).chain());

        let data = app_data_from_app(&mut app).unwrap();
        assert_eq!(data.schedules.len(), 1);
        let schedule = &data.schedules[0];
        assert_eq!(schedule.name, "Update");
        assert_eq!(schedule.systems, []);
        assert_eq!(
            schedule.system_sets,
            [
                simple_system_set("MySet<0>"),
                simple_system_set("MySet<1>"),
                simple_system_set("MySet<2>"),
            ]
        );
        assert_eq!(schedule.hierarchy, []);
        // There are 2 dependency edges to connect 0-1 and 1-2.
        assert_eq!(
            schedule.dependency,
            [
                (ScheduleIndex::SystemSet(0), ScheduleIndex::SystemSet(1)),
                (ScheduleIndex::SystemSet(1), ScheduleIndex::SystemSet(2)),
            ]
        );
        assert_eq!(schedule.components.len(), 0);
        assert_eq!(schedule.conflicts.len(), 0);
    }

    #[test]
    fn stack_of_system_sets() {
        let mut app = App::empty();

        fn a() {}

        app.add_systems(Update, a.in_set(MySet::<0>))
            .configure_sets(Update, MySet::<0>.in_set(MySet::<1>))
            .configure_sets(Update, MySet::<1>.in_set(MySet::<2>));

        let data = app_data_from_app(&mut app).unwrap();
        assert_eq!(data.schedules.len(), 1);
        let schedule = &data.schedules[0];
        assert_eq!(schedule.name, "Update");
        assert_eq!(schedule.systems, [simple_system("a")]);
        assert_eq!(
            schedule.system_sets,
            [
                simple_system_set("MySet<0>"),
                simple_system_set("MySet<1>"),
                simple_system_set("MySet<2>"),
                simple_system_set("SystemTypeSet:a"),
            ]
        );
        assert_eq!(
            schedule.hierarchy,
            [
                (0, ScheduleIndex::System(0)),
                (1, ScheduleIndex::SystemSet(0)),
                (2, ScheduleIndex::SystemSet(1)),
                (3, ScheduleIndex::System(0)),
            ]
        );
        assert_eq!(schedule.dependency, []);
        assert_eq!(schedule.components.len(), 0);
        assert_eq!(schedule.conflicts.len(), 0);
    }

    #[test]
    fn records_system_kind_flags() {
        let mut app = App::empty();

        fn a0(_commands: Commands) {}
        fn a1(_commands: Commands) {}
        fn b0() {}
        fn b1() {}

        fn c0() {}
        fn c1() {}

        app.add_systems(
            Update,
            (
                (
                    (a0, a1), //
                    (b0, b1), //
                )
                    .chain(),
                (c0, c1).chain(),
            ),
        );

        let data = app_data_from_app(&mut app).unwrap();
        assert_eq!(data.schedules.len(), 1);
        let schedule = &data.schedules[0];
        assert_eq!(schedule.name, "Update");
        assert_eq!(
            schedule.systems,
            [
                SystemData {
                    name: "a0".into(),
                    apply_deferred: false,
                    exclusive: false,
                    deferred: true,
                },
                SystemData {
                    name: "a1".into(),
                    apply_deferred: false,
                    exclusive: false,
                    deferred: true,
                },
                SystemData {
                    name: "apply_deferred".into(),
                    apply_deferred: true,
                    exclusive: true,
                    deferred: false,
                },
                simple_system("b0"),
                simple_system("b1"),
                simple_system("c0"),
                simple_system("c1"),
            ]
        );
        assert_eq!(
            schedule.system_sets,
            [
                simple_system_set("SystemTypeSet:a0"),
                simple_system_set("SystemTypeSet:a1"),
                simple_system_set("SystemTypeSet:b0"),
                simple_system_set("SystemTypeSet:b1"),
                simple_system_set("SystemTypeSet:c0"),
                simple_system_set("SystemTypeSet:c1"),
            ]
        );
        assert_eq!(
            schedule.hierarchy,
            [
                (0, ScheduleIndex::System(0)),
                (1, ScheduleIndex::System(1)),
                (2, ScheduleIndex::System(3)),
                (3, ScheduleIndex::System(4)),
                (4, ScheduleIndex::System(5)),
                (5, ScheduleIndex::System(6)),
            ]
        );
        assert_eq!(
            schedule.dependency,
            [
                // TODO: These dependencies are incomplete - after the schedule is built, the
                // execution schedule contains edges a->sync->b. However these edges only get
                // attached to the execution scheudule: not the original graph, where we get edges
                // from.
                // a->b
                (ScheduleIndex::System(0), ScheduleIndex::System(3)),
                (ScheduleIndex::System(0), ScheduleIndex::System(4)),
                (ScheduleIndex::System(1), ScheduleIndex::System(3)),
                (ScheduleIndex::System(1), ScheduleIndex::System(4)),
                // c0->c1
                (ScheduleIndex::System(5), ScheduleIndex::System(6)),
            ]
        );
        assert_eq!(schedule.components.len(), 0);
        assert_eq!(schedule.conflicts.len(), 0);
    }

    #[test]
    fn records_conflicts() {
        let mut app = App::empty();

        // These two systems don't conflict.
        fn a0(_: Query<&MyComponent<0>>) {}
        fn a1(_: Query<&MyComponent<0>>) {}

        // These two systems conflict on one component.
        fn b0(_: Query<&MyComponent<1>>) {}
        fn b1(_: Query<&mut MyComponent<1>>) {}

        // These two systems conflict on two components.
        fn c0(
            _: Query<(
                &MyComponent<2>,
                &mut MyComponent<3>,
                &MyComponent<4>,
                &MyComponent<5>,
            )>,
        ) {
        }
        fn c1(
            _: Query<(
                &mut MyComponent<2>,
                &MyComponent<3>,
                &MyComponent<4>,
                &MyComponent<6>,
            )>,
        ) {
        }

        // These two systems use With/Without to avoid a conflict.
        fn d0(_: Query<&mut MyComponent<7>, With<MyComponent<8>>>) {}
        fn d1(_: Query<&mut MyComponent<7>, Without<MyComponent<8>>>) {}

        // These two systems use an ordering to avoid a conflict.
        fn e0(_: Query<&mut MyComponent<9>>) {}
        fn e1(_: Query<&mut MyComponent<9>>) {}

        app.add_systems(Update, (a0, a1, b0, b1, c0, c1, d0, d1, (e0, e1).chain()));

        let data = app_data_from_app(&mut app).unwrap();
        assert_eq!(data.schedules.len(), 1);
        let schedule = &data.schedules[0];
        assert_eq!(schedule.name, "Update");
        assert_eq!(
            schedule.systems,
            [
                simple_system("a0"),
                simple_system("a1"),
                simple_system("b0"),
                simple_system("b1"),
                simple_system("c0"),
                simple_system("c1"),
                simple_system("d0"),
                simple_system("d1"),
                simple_system("e0"),
                simple_system("e1"),
            ]
        );
        assert_eq!(
            schedule.system_sets,
            [
                simple_system_set("SystemTypeSet:a0"),
                simple_system_set("SystemTypeSet:a1"),
                simple_system_set("SystemTypeSet:b0"),
                simple_system_set("SystemTypeSet:b1"),
                simple_system_set("SystemTypeSet:c0"),
                simple_system_set("SystemTypeSet:c1"),
                simple_system_set("SystemTypeSet:d0"),
                simple_system_set("SystemTypeSet:d1"),
                simple_system_set("SystemTypeSet:e0"),
                simple_system_set("SystemTypeSet:e1"),
            ]
        );
        assert_eq!(
            schedule.hierarchy,
            [
                (0, ScheduleIndex::System(0)),
                (1, ScheduleIndex::System(1)),
                (2, ScheduleIndex::System(2)),
                (3, ScheduleIndex::System(3)),
                (4, ScheduleIndex::System(4)),
                (5, ScheduleIndex::System(5)),
                (6, ScheduleIndex::System(6)),
                (7, ScheduleIndex::System(7)),
                (8, ScheduleIndex::System(8)),
                (9, ScheduleIndex::System(9)),
            ]
        );
        assert_eq!(
            schedule.dependency,
            [
                // e0 -> e1
                (ScheduleIndex::System(8), ScheduleIndex::System(9)),
            ]
        );
        assert_eq!(
            schedule.components,
            [
                simple_component("MyComponent<1>"),
                simple_component("MyComponent<2>"),
                simple_component("MyComponent<3>"),
            ]
        );
        assert_eq!(
            schedule.conflicts,
            [conflict(2, 3, vec![0]), conflict(4, 5, vec![1, 2])]
        );
    }
}
