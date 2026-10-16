use bevy_platform::{collections::HashMap, sync::Arc};
use bevy_ptr::{OwningPtr, UnsafeCellDeref};
use bevy_utils::prelude::DebugName;
use core::marker::PhantomData;

pub use crate::change_detection::{SetLocal, SetLocalMut};

use crate::{
    change_detection::{ComponentTicksMut, ComponentTicksRef, MaybeLocation, Tick},
    component::{ComponentDescriptor, ComponentId, Mutable, IS_RESOURCE},
    query::FilteredAccess,
    resource::{IsResource, Resource},
    schedule::{InternedSystemSet, SystemSet},
    system::{
        ReadOnlySystemParam, SystemAccess, SystemMeta, SystemParam, SystemParamAccessConflict,
        SystemParamValidationError,
    },
    world::{unsafe_world_cell::UnsafeWorldCell, FromWorld, World},
};

/// The system sets sharing a resource of type `T`, mapped to the [`ComponentId`] of that
/// value.
#[derive(Resource)]
struct SetLocalIds<T: Resource> {
    /// This has to be an Arc so it can be cheaply cloned into each system parameter's state.
    ids: Arc<HashMap<InternedSystemSet, ComponentId>>,
    marker: PhantomData<T>,
}

impl World {
    /// Inserts a [`Resource`] to be shared by all systems
    /// in a [`SystemSet`], if there is no existing instance of `T`
    /// for that set.
    ///
    /// Returns the [`ComponentId`] of the set's value.
    ///
    /// Each set's value is a separate resource, which is also separate from a `T` added with
    /// [`World::insert_resource`].
    #[track_caller]
    pub fn init_set_local<T: Resource + FromWorld>(&mut self, set: impl SystemSet) -> ComponentId {
        let caller = MaybeLocation::caller();
        let set = set.intern();
        if let Some(&id) = self
            .get_resource::<SetLocalIds<T>>()
            .and_then(|set_local_ids| set_local_ids.ids.get(&set))
        {
            return id;
        }
        let value = T::from_world(self);
        self.insert_set_local_with_caller(set, value, caller)
    }

    /// Inserts a [`Resource`] to be shared by all systems
    /// in a [`SystemSet`], overwriting any existing data
    /// of the same type for the set.
    ///
    /// Returns the [`ComponentId`] of the set's value.
    ///
    /// Each set's value is a separate resource, which is also separate from a `T` added with
    /// [`World::insert_resource`].
    #[track_caller]
    pub fn insert_set_local<T: Resource>(&mut self, set: impl SystemSet, value: T) -> ComponentId {
        self.insert_set_local_with_caller(set.intern(), value, MaybeLocation::caller())
    }

    fn insert_set_local_with_caller<T: Resource>(
        &mut self,
        set: InternedSystemSet,
        value: T,
        caller: MaybeLocation,
    ) -> ComponentId {
        let existing = self
            .get_resource::<SetLocalIds<T>>()
            .and_then(|set_local_ids| set_local_ids.ids.get(&set))
            .copied();
        let id = existing.unwrap_or_else(|| {
            let id = self.register_component_with_descriptor(ComponentDescriptor::new::<T>());
            // Registering by descriptor skips `T`'s required components, which include the
            // `IsResource` that a resource's entity needs, so spawn that entity here.
            self.spawn(IsResource::new(id));
            id
        });
        OwningPtr::make(value, |ptr| {
            // SAFETY: `ptr` points to a `T`, which `id` was registered for.
            unsafe { self.insert_resource_by_id(id, ptr, caller) };
        });

        if existing.is_none() {
            let mut set_local_ids = self.get_resource_or_insert_with(|| SetLocalIds::<T> {
                ids: Arc::default(),
                marker: PhantomData,
            });
            Arc::make_mut(&mut set_local_ids.ids).insert(set, id);
        }
        id
    }
}

fn find_set<T: Resource>(
    ids: &HashMap<InternedSystemSet, ComponentId>,
    system_meta: &SystemMeta,
) -> (InternedSystemSet, ComponentId) {
    let mut found = system_meta
        .system_sets()
        .iter()
        .filter_map(|set| Some((*set, *ids.get(set)?)));
    let Some(set) = found.next() else {
        panic!(
            "System {} uses a set-local `{}`, but isn't in a system set that has one. \
             Add one with `World::init_set_local`.",
            system_meta.name(),
            DebugName::type_name::<T>(),
        );
    };
    if found.next().is_some() {
        panic!(
            "System {} uses a set-local `{}`, but is in more than one system set that has one.",
            system_meta.name(),
            DebugName::type_name::<T>(),
        );
    }
    set
}

// SAFETY: `SetLocal` only reads the world.
unsafe impl<'a, T: Resource> ReadOnlySystemParam for SetLocal<'a, T> {}

// SAFETY: `init_access` declares read access to the resource that `get_param` reads, and
// returns an error if that conflicts with another param of the system.
unsafe impl<'a, T: Resource> SystemParam for SetLocal<'a, T> {
    /// The sets that have their own `T`, as of when the system is initialized.
    type State = Arc<HashMap<InternedSystemSet, ComponentId>>;
    type Item<'w, 's> = SetLocal<'w, T>;

    fn init_state(world: &mut World) -> Self::State {
        world
            .get_resource::<SetLocalIds<T>>()
            .map(|set_local_ids| set_local_ids.ids.clone())
            .unwrap_or_default()
    }

    fn init_access(
        ids: &Self::State,
        system_meta: &mut SystemMeta,
        system_access: &mut SystemAccess,
    ) -> Result<(), SystemParamAccessConflict> {
        let (_, id) = find_set::<T>(ids, system_meta);
        let mut filter = FilteredAccess::default();
        filter.add_read(id);
        filter.and_with(IS_RESOURCE);
        system_access
            .try_extend_single(filter)
            .map_err(|access| SystemParamAccessConflict::new::<Self>(access).with_code("B0002"))
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        ids: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Result<Self::Item<'w, 's>, SystemParamValidationError> {
        let (set, id) = find_set::<T>(ids, system_meta);
        // SAFETY: `init_access` declared read access to this resource, so no mutable
        // reference to it exists while the system runs.
        let (ptr, ticks) = unsafe { world.get_resource_with_ticks(id) }.ok_or_else(|| {
            SystemParamValidationError::invalid::<Self>("Set-local resource does not exist")
        })?;
        // SAFETY: The resource with this id is a `T`, and nothing else borrows it mutably.
        unsafe {
            Ok(SetLocal {
                value: ptr.deref(),
                ticks: ComponentTicksRef {
                    added: ticks.added.deref(),
                    changed: ticks.changed.deref(),
                    changed_by: ticks.changed_by.map(|changed_by| changed_by.deref()),
                    last_run: system_meta.last_run,
                    this_run: change_tick,
                },
                set,
            })
        }
    }
}

// SAFETY: `init_access` declares write access to the resource that `get_param` reads, and
// returns an error if that conflicts with another param of the system.
unsafe impl<'a, T: Resource<Mutability = Mutable>> SystemParam for SetLocalMut<'a, T> {
    // Cloned from [`SetLocalIds<T>`]
    type State = Arc<HashMap<InternedSystemSet, ComponentId>>;
    type Item<'w, 's> = SetLocalMut<'w, T>;

    fn init_state(world: &mut World) -> Self::State {
        <SetLocal<T> as SystemParam>::init_state(world)
    }

    fn init_access(
        ids: &Self::State,
        system_meta: &mut SystemMeta,
        system_access: &mut SystemAccess,
    ) -> Result<(), SystemParamAccessConflict> {
        let (_, id) = find_set::<T>(ids, system_meta);
        let mut filter = FilteredAccess::default();
        filter.add_write(id);
        filter.and_with(IS_RESOURCE);
        system_access
            .try_extend_single(filter)
            .map_err(|access| SystemParamAccessConflict::new::<Self>(access).with_code("B0002"))
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        ids: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Result<Self::Item<'w, 's>, SystemParamValidationError> {
        let (set, id) = find_set::<T>(ids, system_meta);
        // SAFETY: `init_access` declared write access to this resource, so no other
        // reference to it exists while the system runs.
        let value = unsafe { world.get_resource_mut_by_id(id) }.ok_or_else(|| {
            SystemParamValidationError::invalid::<Self>("Set-local resource does not exist")
        })?;
        Ok(SetLocalMut {
            // SAFETY: The resource with this id is a `T`.
            value: unsafe { value.value.deref_mut::<T>() },
            ticks: ComponentTicksMut {
                added: value.ticks.added,
                changed: value.ticks.changed,
                changed_by: value.ticks.changed_by,
                last_run: system_meta.last_run,
                this_run: change_tick,
                summary_tick: None,
            },
            set,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use crate::{
        prelude::*,
        schedule::{InternedSystemSet, LogLevel, ScheduleBuildSettings},
    };

    #[derive(Resource, Default)]
    struct Counter(u32);

    #[derive(Resource, Default)]
    struct Seen(Vec<(InternedSystemSet, u32)>);

    #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    enum TestSet {
        A,
        B,
        Inner,
    }

    fn increment(mut counter: SetLocalMut<Counter>) {
        counter.0 += 1;
    }

    fn record(counter: SetLocal<Counter>, mut seen: ResMut<Seen>) {
        seen.0.push((counter.set(), counter.0));
    }

    fn world_with_sets() -> World {
        let mut world = World::new();
        world.init_resource::<Seen>();
        world.init_set_local::<Counter>(TestSet::A);
        world.init_set_local::<Counter>(TestSet::B);
        world
    }

    #[test]
    fn each_set_has_its_own_value() {
        let mut world = world_with_sets();
        let mut schedule = Schedule::default();
        schedule.add_systems((
            (increment, record).chain().in_set(TestSet::A),
            (increment, increment, record).chain().in_set(TestSet::B),
        ));
        schedule.configure_sets(TestSet::A.before(TestSet::B));
        schedule.run(&mut world);

        assert_eq!(
            world.resource::<Seen>().0,
            vec![(TestSet::A.intern(), 1), (TestSet::B.intern(), 2)]
        );
    }

    #[test]
    fn nested_sets_share_the_outer_value() {
        let mut world = world_with_sets();
        let mut schedule = Schedule::default();
        schedule.configure_sets(TestSet::Inner.in_set(TestSet::A));
        schedule.add_systems((increment.in_set(TestSet::Inner), record.in_set(TestSet::A)).chain());
        schedule.run(&mut world);

        assert_eq!(world.resource::<Seen>().0, vec![(TestSet::A.intern(), 1)]);
    }

    #[test]
    fn init_set_local_is_idempotent() {
        let mut world = World::new();
        let id = world.init_set_local::<Counter>(TestSet::A);
        assert_eq!(world.init_set_local::<Counter>(TestSet::A), id);
        assert_ne!(world.init_set_local::<Counter>(TestSet::B), id);
    }

    #[test]
    fn insert_set_local_replaces_the_value() {
        let mut world = world_with_sets();
        let id = world.insert_set_local(TestSet::A, Counter(5));
        assert_eq!(world.insert_set_local(TestSet::A, Counter(7)), id);

        let mut schedule = Schedule::default();
        schedule.add_systems(record.in_set(TestSet::A));
        schedule.run(&mut world);

        assert_eq!(world.resource::<Seen>().0, vec![(TestSet::A.intern(), 7)]);
    }

    #[test]
    fn separate_from_the_global_resource() {
        let mut world = world_with_sets();
        world.insert_resource(Counter(10));

        let mut schedule = Schedule::default();
        schedule.add_systems(increment.in_set(TestSet::A));
        schedule.run(&mut world);

        assert_eq!(world.resource::<Counter>().0, 10);
    }

    #[test]
    fn run_conditions_use_the_value_of_their_set() {
        let mut world = world_with_sets();
        let mut schedule = Schedule::default();
        // A set's condition reads the value of that set.
        schedule.configure_sets(TestSet::B.run_if(|counter: SetLocal<Counter>| counter.0 < 1));
        schedule.add_systems((
            // A system's condition reads the value of the system's set.
            increment
                .run_if(|counter: SetLocal<Counter>| counter.0 < 2)
                .in_set(TestSet::A),
            increment.in_set(TestSet::B),
        ));
        for _ in 0..3 {
            schedule.run(&mut world);
        }

        let mut check = Schedule::default();
        check.add_systems((record.in_set(TestSet::A), record.in_set(TestSet::B)).chain());
        check.run(&mut world);
        assert_eq!(
            world.resource::<Seen>().0,
            vec![(TestSet::A.intern(), 2), (TestSet::B.intern(), 1)]
        );
    }

    #[test]
    fn nested_set_conditions_see_outer_sets() {
        let mut world = world_with_sets();
        let mut schedule = Schedule::default();
        // `Inner` has no value of its own, so its condition reads the value of `A`.
        schedule.configure_sets(
            TestSet::Inner
                .in_set(TestSet::A)
                .run_if(|counter: SetLocal<Counter>| counter.0 < 1),
        );
        schedule.add_systems(increment.in_set(TestSet::Inner));
        for _ in 0..3 {
            schedule.run(&mut world);
        }

        let mut check = Schedule::default();
        check.add_systems(record.in_set(TestSet::A));
        check.run(&mut world);
        assert_eq!(world.resource::<Seen>().0, vec![(TestSet::A.intern(), 1)]);
    }

    #[test]
    fn change_detection() {
        fn assert_changed(counter: SetLocal<Counter>) {
            assert!(counter.is_changed());
        }

        let mut world = world_with_sets();
        let mut schedule = Schedule::default();
        schedule.add_systems((increment, assert_changed).chain().in_set(TestSet::A));
        schedule.run(&mut world);
    }

    /// Builds a schedule that errors on conflicting systems with no order between them.
    fn ambiguity_checked_schedule() -> Schedule {
        let mut schedule = Schedule::default();
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Error,
            ..Default::default()
        });
        schedule
    }

    #[test]
    fn readers_in_a_set_do_not_conflict() {
        let mut world = world_with_sets();
        let mut schedule = ambiguity_checked_schedule();
        schedule
            .add_systems((|_: SetLocal<Counter>| {}, |_: SetLocal<Counter>| {}).in_set(TestSet::A));
        assert!(schedule.initialize(&mut world).is_ok());
    }

    #[test]
    fn writers_in_different_sets_do_not_conflict() {
        let mut world = world_with_sets();
        let mut schedule = ambiguity_checked_schedule();
        schedule.add_systems((increment.in_set(TestSet::A), increment.in_set(TestSet::B)));
        assert!(schedule.initialize(&mut world).is_ok());
    }

    #[test]
    fn writers_in_a_set_conflict() {
        let mut world = world_with_sets();
        let mut schedule = ambiguity_checked_schedule();
        schedule.add_systems((increment, increment).in_set(TestSet::A));
        assert!(schedule.initialize(&mut world).is_err());
    }

    #[test]
    #[should_panic = "error[B0002]"]
    fn reading_and_writing_in_one_system_conflicts() {
        fn system(_: SetLocal<Counter>, _: SetLocalMut<Counter>) {}

        let mut world = world_with_sets();
        let mut schedule = Schedule::default();
        schedule.add_systems(system.in_set(TestSet::A));
        schedule.run(&mut world);
    }

    #[test]
    #[should_panic = "isn't in a system set that has one"]
    fn system_outside_a_set_panics() {
        let mut world = world_with_sets();
        let mut schedule = Schedule::default();
        schedule.add_systems(increment);
        schedule.run(&mut world);
    }

    #[test]
    #[should_panic = "is in more than one system set that has one"]
    fn system_in_two_sets_panics() {
        let mut world = world_with_sets();
        let mut schedule = Schedule::default();
        schedule.add_systems(increment.in_set(TestSet::A).in_set(TestSet::B));
        schedule.run(&mut world);
    }
}
