//! Utilities for serializing schedule data for an [`App`](bevy_app::App).
//!
//! These are mostly around providing types implementing [`Serialize`]/[`Deserialize`] that
//! represent schedule data. In addition, there are tools for extracting this data from the
//! [`World`](bevy_ecs::world::World).

use bevy_ecs::{
    component::{ComponentId, Components},
    schedule::{
        ApplyDeferred, ConditionWithAccess, InternedScheduleLabel, NodeId, Schedule,
        ScheduleBuildMetadata, Schedules,
    },
    system::SystemStateFlags,
};
use bevy_platform::collections::HashMap;
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
        label_to_build_metadata: &HashMap<InternedScheduleLabel, ScheduleBuildMetadata>,
    ) -> Result<Self, ExtractAppDataError> {
        Ok(Self {
            schedules: schedules
                .iter()
                .map(|(_, schedule)| {
                    ScheduleData::from_schedule(
                        schedule,
                        world_components,
                        label_to_build_metadata.get(&schedule.label()),
                    )
                })
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
    pub hierarchy: Vec<(SystemSetIndex, ScheduleIndex)>,
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
    /// Direct required component indexes
    pub required: Vec<usize>,
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
    /// Filtered accesses for the system, generally 1x per system query
    pub filtered_accesses: Vec<FilteredAccessData>,
    // TODO: Store run conditions specific to this system.
}

/// A serializable version of [`bevy_ecs::query::Access`].
/// Tracks read and write access to specific components.
///
/// All indexes are into [`ScheduleData::components`]
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct AccessData {
    /// All accessed components, or forbidden components if
    /// `Self::reads_inverted` is set.
    pub reads: Vec<usize>,
    /// All exclusively-accessed components, or components that may not be
    /// exclusively accessed if `Self::writes_inverted` is set.
    pub writes: Vec<usize>,
    /// Is `true` if this component can read all components *except* those
    /// present in `Self::reads`.
    pub reads_inverted: bool,
    /// Is `true` if this component can write to all components *except* those
    /// present in `Self::writes`.
    pub writes_inverted: bool,
    /// Components that are not accessed, but whose presence in an archetype affect query results.
    pub archetypal: Vec<usize>,
}

impl AccessData {
    fn new(value: &bevy_ecs::query::Access, trace: &mut ComponentTrace) -> Self {
        // NOTE: `try_reads` returns error if `reads_inverted=true`,
        // thus `AccessData` always has `reads_inverted=false`
        // Similarly for `try_writes` and `writes_inverted=false`
        // We return empty vectors when inverted=true, however this should not be used by consumers.

        let reads = value.try_reads();
        let writes = value.try_writes();

        let (reads_inverted, reads) = match reads {
            Ok(reads) => (false, trace.get_indexes(reads.iter())),
            Err(_) => (true, vec![]),
        };
        let (writes_inverted, writes) = match writes {
            Ok(writes) => (false, trace.get_indexes(writes.iter())),
            Err(_) => (true, vec![]),
        };

        Self {
            reads,
            writes,
            reads_inverted,
            writes_inverted,
            archetypal: trace.get_indexes(value.archetypal().iter()),
        }
    }
}

/// A serializable version of [`bevy_ecs::query::AccessFilters`].
/// A clause in disjunctive normal form that filters entities by their components.
/// An [`AccessFiltersData`] matches entities that have *all* the components in the
/// `with` filters and *none* of the components in the `without` filters.
///
/// All indexes are into [`ScheduleData::components`]
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct AccessFiltersData {
    /// The set of components that must all be present for this [`AccessFiltersData`] to match.
    pub with: Vec<usize>,
    /// The set of components that must all be absent for this [`AccessFiltersData`] to match.
    pub without: Vec<usize>,
}

impl AccessFiltersData {
    fn new(value: &bevy_ecs::query::AccessFilters, trace: &mut ComponentTrace) -> Self {
        Self {
            with: trace.get_indexes(value.with().iter()),
            without: trace.get_indexes(value.without().iter()),
        }
    }
}

/// A serializable version of [`bevy_ecs::query::FilteredAccess`] (docs copied from there).
/// Corresponds to a query in the system signature.
///
/// All indexes are into [`ScheduleData::components`]
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct FilteredAccessData {
    /// The access of the Query (components that have read/writes)
    pub access: AccessData,
    /// An array of filter sets to express `With` or `Without` clauses.
    /// Each filter set is `Or`-d together
    pub filter_sets: Vec<AccessFiltersData>,
}

impl FilteredAccessData {
    fn new(value: &bevy_ecs::query::FilteredAccess, trace: &mut ComponentTrace) -> Self {
        let access = AccessData::new(value.access(), trace);
        let filter_sets = value
            .filter_sets()
            .iter()
            .map(|f| AccessFiltersData::new(f, trace))
            .collect();

        Self {
            access,
            filter_sets,
        }
    }
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
    System(usize),
    /// The index of a system set.
    SystemSet(usize),
}

/// Data about an access conflict between two systems.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SystemConflict {
    /// The first system index.
    pub system_1: usize,
    /// The second system index.
    pub system_2: usize,
    /// The kind of conflict between these systems.
    pub conflicting_access: AccessConflict,
}

/// Data for describing the kind of access conflict.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum AccessConflict {
    /// There is a conflict on the **whole world**, since one of the systems requires world access
    /// and the other needs mutable access to (some of) the world.
    World,
    /// There is incompatible accesses to the listed components.
    Components(Vec<usize>),
}

/// A newtype for the index of a system set.
///
/// This is the same kind of index as [`ScheduleIndex::SystemSet`], but for cases where we know we
/// can't have a system.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord)]
pub struct SystemSetIndex(pub usize);

/// Helper to build the components used by systems in this schedule
struct ComponentTrace<'a> {
    world_components: &'a Components,
    component_id_to_index: HashMap<ComponentId, usize>,
    components: Vec<ComponentData>,
}

impl<'a> ComponentTrace<'a> {
    fn with_world_components(world_components: &'a Components) -> Self {
        Self {
            world_components,
            component_id_to_index: HashMap::new(),
            components: vec![],
        }
    }

    fn get_index(&mut self, id: ComponentId) -> usize {
        let result1 = self.component_id_to_index.get(&id);
        if let Some(r) = result1 {
            return *r;
        }

        let component = self
            .world_components
            .get_info(id)
            .expect("the component has already been registered by the system");

        let required = component
            .required_components()
            .iter_direct_ids()
            .map(|rid| self.get_index(rid))
            .collect();

        let res = ComponentData {
            name: format!("{}", component.name()),
            required,
        };

        self.components.push(res);
        let result = self.components.len() - 1;

        self.component_id_to_index.insert(id, result);
        result
    }

    fn get_indexes(&mut self, ids: impl Iterator<Item = ComponentId>) -> Vec<usize> {
        ids.map(|id| self.get_index(id)).collect()
    }
}

impl ScheduleData {
    /// Creates the data from the underlying [`Schedule`].
    ///
    /// Note: we assume `schedule` has already been initialized.
    pub fn from_schedule(
        schedule: &Schedule,
        world_components: &Components,
        build_metadata: Option<&ScheduleBuildMetadata>,
    ) -> Result<Self, ExtractAppDataError> {
        let mut component_trace = ComponentTrace::with_world_components(world_components);

        let graph = schedule.graph();

        let mut system_key_to_index = HashMap::new();
        let mut system_set_key_to_index = HashMap::new();

        fn extract_condition_data(conditions: &[ConditionWithAccess]) -> Vec<ConditionData> {
            conditions
                .iter()
                .map(|condition| ConditionData {
                    name: format!("{}", condition.condition.name()),
                })
                .collect()
        }

        let systems = schedule
            .systems_with_access()
            .map_err(|_| {
                ExtractAppDataError::ScheduleNotInitialized(format!("{:?}", schedule.label()))
            })?
            .enumerate()
            .map(|(index, (key, system_with_access))| {
                system_key_to_index.insert(key, index);

                let system = system_with_access.system();
                let access = system_with_access.access();
                let filtered_accesses = access.filtered_accesses();

                let flags = system.flags();

                SystemData {
                    name: format!("{}", system.name()),
                    apply_deferred: system.system_type()
                        == core::any::TypeId::of::<ApplyDeferred>(),
                    exclusive: flags.contains(SystemStateFlags::EXCLUSIVE),
                    deferred: flags.contains(SystemStateFlags::DEFERRED),
                    filtered_accesses: filtered_accesses
                        .iter()
                        .map(|fa| FilteredAccessData::new(fa, &mut component_trace))
                        .collect(),
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

                (SystemSetIndex(*parent as _), child)
            })
            .collect();

        let mut dependency = graph
            .dependency()
            .graph()
            .all_edges()
            .map(|(a, b)| (node_id_to_schedule_index(a), node_id_to_schedule_index(b)))
            .collect::<Vec<_>>();

        if let Some(build_metadata) = build_metadata {
            // Add in all the edges that were created by build passes.
            dependency.extend(
                build_metadata
                    .edges_added_by_build_passes
                    .iter()
                    .map(|(a, b)| {
                        (
                            node_id_to_schedule_index(NodeId::System(*a)),
                            node_id_to_schedule_index(NodeId::System(*b)),
                        )
                    }),
            );
        }

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
                    conflicting_access: if conflicts.is_empty() {
                        // The systems conflict on the world if there's no particular component IDs.
                        AccessConflict::World
                    } else {
                        AccessConflict::Components(
                            component_trace.get_indexes(conflicts.iter().copied()),
                        )
                    },
                }
            })
            .collect();

        let components = component_trace.components;

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
/// Tests for extracted schedule data.
///
/// This is public to allow other test modules in this crate to use its utilities.
pub mod tests {
    use bevy_app::{App, Update};
    use bevy_ecs::{
        component::Component,
        query::{With, Without},
        schedule::{IntoScheduleConfigs, Schedules, SystemSet},
        system::{Commands, Query},
    };
    use bevy_platform::collections::HashMap;

    use crate::schedule_data::serde::{
        AccessConflict, AccessData, AccessFiltersData, AppData, ComponentData, ExtractAppDataError,
        FilteredAccessData, ScheduleData, ScheduleIndex, SystemConflict, SystemData, SystemSetData,
        SystemSetIndex,
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

        let mut label_to_build_metadata = HashMap::new();

        for label in interned_labels {
            let build_metadata = app
                .world_mut()
                .schedule_scope(label, |world, schedule| schedule.initialize(world))
                .unwrap()
                .unwrap();
            label_to_build_metadata.insert(label, build_metadata);
        }

        let mut app_data = AppData::from_schedules(
            app.world().resource::<Schedules>(),
            app.world().components(),
            &label_to_build_metadata,
        )?;

        remove_module_paths(&mut app_data);
        sort_app_data(&mut app_data);
        Ok(app_data)
    }

    /// Removes the module paths from all items in the [`AppData`], so that moving tests around
    /// doesn't change the output.
    pub fn remove_module_paths(app_data: &mut AppData) {
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
    }

    /// Sorts the [`AppData`] so we have a deterministic order when asserting.
    // Note: we could do this when extracting unconditionally (even in prod), but there's not much
    // point since schedule order is not guaranteed to be deterministic anyway. So relying on the
    // same order seems weird.
    pub fn sort_app_data(app_data: &mut AppData) {
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

            let reindex_system = |index: &mut usize| {
                *index = system_old_index_to_new_index[index];
            };
            let reindex_system_set = |index: &mut usize| {
                *index = system_set_old_index_to_new_index[index];
            };
            let reindex_schedule_index = |index: &mut ScheduleIndex| match index {
                ScheduleIndex::System(system) => reindex_system(system),
                ScheduleIndex::SystemSet(set) => reindex_system_set(set),
            };

            let reindex_component = |index: &mut usize| {
                *index = component_old_index_to_new_index[index];
            };

            let reindex_component_vec = |components: &mut Vec<usize>| {
                for component in components.iter_mut() {
                    reindex_component(component);
                }
                components.sort();
            };

            for component in schedule.components.iter_mut() {
                reindex_component_vec(&mut component.required);
            }

            let reindex_access = |access: &mut AccessData| {
                reindex_component_vec(&mut access.archetypal);
                reindex_component_vec(&mut access.reads);
                reindex_component_vec(&mut access.writes);
            };

            // Sort the conditions in a system set.
            for set in schedule.system_sets.iter_mut() {
                set.conditions
                    .sort_by_key(|condition| condition.name.clone());
            }

            // Reindex the hierarchy, and sort it.
            for (parent, child) in schedule.hierarchy.iter_mut() {
                reindex_system_set(&mut parent.0);
                reindex_schedule_index(child);
            }
            schedule.hierarchy.sort();

            // Reindex the dependencies, and sort it.
            for (parent, child) in schedule.dependency.iter_mut() {
                reindex_schedule_index(parent);
                reindex_schedule_index(child);
            }
            schedule.dependency.sort();

            // Reindex access in systems
            for system in schedule.systems.iter_mut() {
                for filtered_access in system.filtered_accesses.iter_mut() {
                    reindex_access(&mut filtered_access.access);

                    for filter_set in filtered_access.filter_sets.iter_mut() {
                        reindex_component_vec(&mut filter_set.with);
                        reindex_component_vec(&mut filter_set.without);
                    }
                }
            }

            // Reindex the conflicts.
            for conflict in schedule.conflicts.iter_mut() {
                reindex_system(&mut conflict.system_1);
                reindex_system(&mut conflict.system_2);

                // The order of the indices don't matter, so pick the ordering such that `system_1 <
                // system_2`.
                if conflict.system_1 > conflict.system_2 {
                    core::mem::swap(&mut conflict.system_1, &mut conflict.system_2);
                }

                match &mut conflict.conflicting_access {
                    AccessConflict::World => {}
                    AccessConflict::Components(components) => {
                        components.iter_mut().for_each(reindex_component);
                        components.sort();
                    }
                };
            }
            schedule
                .conflicts
                .sort_by_key(|conflict| (conflict.system_1, conflict.system_2));
        }
    }

    /// Convenience to create a [`SystemData`] for the common case of no flags set.
    pub fn simple_system(name: &str) -> SystemData {
        SystemData {
            name: name.into(),
            apply_deferred: false,
            exclusive: false,
            deferred: false,
            filtered_accesses: vec![],
        }
    }

    /// Convenience to create a [`SystemData`] for more detailed case.
    pub fn full_system(
        name: &str,
        reads: Vec<usize>,
        writes: Vec<usize>,
        with: Option<Vec<usize>>,
        without: Option<Vec<usize>>,
    ) -> SystemData {
        // +1 on inputted components as 0 = Disabled

        let offset_reads: Vec<usize> = reads.iter().map(|n| n + 1).collect();
        let offset_writes: Vec<usize> = writes.iter().map(|n| n + 1).collect();

        let offset_with: Vec<usize> = with
            .map(|f| f.iter().map(|n| n + 1).collect())
            .unwrap_or(offset_reads.clone());

        let mut offset_without: Vec<usize> = without
            .map(|f| f.iter().map(|n| n + 1).collect())
            .unwrap_or(vec![]);
        offset_without.insert(0, 0);

        SystemData {
            name: name.into(),
            apply_deferred: false,
            exclusive: false,
            deferred: false,
            filtered_accesses: vec![FilteredAccessData {
                access: AccessData {
                    reads: offset_reads.clone(),
                    writes: offset_writes.clone(),
                    reads_inverted: false,
                    writes_inverted: false,
                    archetypal: vec![],
                },
                filter_sets: vec![AccessFiltersData {
                    with: offset_with.clone(),
                    without: offset_without.clone(),
                }],
            }],
        }
    }

    /// Convenience to create a [`SystemSetData`] for the common case of being empty.
    pub fn simple_system_set(name: &str) -> SystemSetData {
        SystemSetData {
            name: name.into(),
            conditions: vec![],
        }
    }

    /// Convenience to create a [`ComponentData`] to make test cases shorter.
    pub fn simple_component(name: &str) -> ComponentData {
        ComponentData {
            name: name.into(),
            required: vec![],
        }
    }

    /// Convenience to create a [`SystemConflict`] to make test cases shorter.
    pub fn conflict(
        system_1: usize,
        system_2: usize,
        conflicting_access: AccessConflict,
    ) -> SystemConflict {
        SystemConflict {
            system_1,
            system_2,
            conflicting_access,
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
                (SystemSetIndex(0), ScheduleIndex::System(0)),
                (SystemSetIndex(1), ScheduleIndex::System(1)),
                (SystemSetIndex(2), ScheduleIndex::System(2)),
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
                (SystemSetIndex(0), ScheduleIndex::System(0)),
                (SystemSetIndex(1), ScheduleIndex::SystemSet(0)),
                (SystemSetIndex(2), ScheduleIndex::SystemSet(1)),
                (SystemSetIndex(3), ScheduleIndex::System(0)),
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

        app.add_systems(Update, (((a0, a1), (b0, b1)).chain(), (c0, c1).chain()));

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
                    filtered_accesses: vec![],
                },
                SystemData {
                    name: "a1".into(),
                    apply_deferred: false,
                    exclusive: false,
                    deferred: true,
                    filtered_accesses: vec![],
                },
                SystemData {
                    name: "apply_deferred".into(),
                    apply_deferred: true,
                    exclusive: true,
                    deferred: false,
                    filtered_accesses: vec![],
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
                (SystemSetIndex(0), ScheduleIndex::System(0)),
                (SystemSetIndex(1), ScheduleIndex::System(1)),
                (SystemSetIndex(2), ScheduleIndex::System(3)),
                (SystemSetIndex(3), ScheduleIndex::System(4)),
                (SystemSetIndex(4), ScheduleIndex::System(5)),
                (SystemSetIndex(5), ScheduleIndex::System(6)),
            ]
        );
        assert_eq!(
            schedule.dependency,
            [
                // a->sync and a->b
                (ScheduleIndex::System(0), ScheduleIndex::System(2)),
                (ScheduleIndex::System(0), ScheduleIndex::System(3)),
                (ScheduleIndex::System(0), ScheduleIndex::System(4)),
                (ScheduleIndex::System(1), ScheduleIndex::System(2)),
                (ScheduleIndex::System(1), ScheduleIndex::System(3)),
                (ScheduleIndex::System(1), ScheduleIndex::System(4)),
                // sync->b
                (ScheduleIndex::System(2), ScheduleIndex::System(3)),
                (ScheduleIndex::System(2), ScheduleIndex::System(4)),
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
            schedule.components,
            [
                simple_component("Disabled"),
                simple_component("MyComponent<0>"),
                simple_component("MyComponent<1>"),
                simple_component("MyComponent<2>"),
                simple_component("MyComponent<3>"),
                simple_component("MyComponent<4>"),
                simple_component("MyComponent<5>"),
                simple_component("MyComponent<6>"),
                simple_component("MyComponent<7>"),
                simple_component("MyComponent<8>"),
                simple_component("MyComponent<9>"),
            ]
        );

        assert_eq!(
            schedule.systems,
            [
                full_system("a0", vec![0], vec![], None, None),
                full_system("a1", vec![0], vec![], None, None),
                full_system("b0", vec![1], vec![], None, None),
                full_system("b1", vec![1], vec![1], None, None),
                full_system("c0", vec![2, 3, 4, 5], vec![3], None, None),
                full_system("c1", vec![2, 3, 4, 6], vec![2], None, None),
                full_system("d0", vec![7], vec![7], Some(vec![7, 8]), None),
                full_system("d1", vec![7], vec![7], None, Some(vec![8])),
                full_system("e0", vec![9], vec![9], None, None),
                full_system("e1", vec![9], vec![9], None, None),
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
                (SystemSetIndex(0), ScheduleIndex::System(0)),
                (SystemSetIndex(1), ScheduleIndex::System(1)),
                (SystemSetIndex(2), ScheduleIndex::System(2)),
                (SystemSetIndex(3), ScheduleIndex::System(3)),
                (SystemSetIndex(4), ScheduleIndex::System(4)),
                (SystemSetIndex(5), ScheduleIndex::System(5)),
                (SystemSetIndex(6), ScheduleIndex::System(6)),
                (SystemSetIndex(7), ScheduleIndex::System(7)),
                (SystemSetIndex(8), ScheduleIndex::System(8)),
                (SystemSetIndex(9), ScheduleIndex::System(9)),
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
            schedule.conflicts,
            [
                // +1 on components for 0 = Disabled

                // b0, b1 conflict on 1
                conflict(2, 3, AccessConflict::Components(vec![2])),
                // c0, c1 conflict on 2, 3
                conflict(4, 5, AccessConflict::Components(vec![3, 4]))
            ]
        );
    }
}
