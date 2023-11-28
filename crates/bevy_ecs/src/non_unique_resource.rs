//! Non-unique resource.
//!
//! See [`NonUniqueResourceRef`] for details.

use crate::archetype::ArchetypeComponentId;
use crate::component::{ComponentId, Tick};
use crate::prelude::World;
use crate::query::Access;
use crate::storage::non_unique_resource::NonUniqueResourceEntry;
use crate::storage::TableRow;
use crate::system::{check_system_change_tick, System, SystemMeta};
use crate::system::{In, IntoSystem};
use crate::world::unsafe_world_cell::UnsafeWorldCell;
use bevy_ptr::Ptr;
use std::any;
use std::any::TypeId;
use std::borrow::Cow;
use std::marker::PhantomData;

/// Non-unique resource (multiple instances of the same type are stored in the world).
///
/// Resource is allocated with [`World::new_non_unique_resource()`].
pub struct NonUniqueResourceRef<T: Sync + Send + 'static> {
    /// Unique per `NonUniqueResourceRef<T>` instance.
    component_id: ComponentId,
    /// Index in table.
    index: TableRow,
    /// Allow concurrent access to different instances of `NonUniqueResourceRef<T>`.
    archetype_component_id: ArchetypeComponentId,
    /// We secretly store a `T` here.
    _phantom: PhantomData<T>,
}

impl<T: Sync + Send + 'static> Clone for NonUniqueResourceRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Sync + Send + 'static> Copy for NonUniqueResourceRef<T> {}

struct NonUniqueResourceSystem<T: Sync + Send + 'static, In, Out, F, const WRITE: bool> {
    non_unique_resource_ref: NonUniqueResourceRef<T>,
    system_meta: SystemMeta,
    function: F,
    _phantom: PhantomData<(In, Out)>,
}

impl<T, In, Out, F, const WRITE: bool> NonUniqueResourceSystem<T, In, Out, F, WRITE>
where
    T: Sync + Send + 'static,
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    F: Fn(In, Ptr) -> Out + Sync + Send + 'static,
{
    pub fn new(
        non_unique_resource_ref: NonUniqueResourceRef<T>,
        function: F,
    ) -> NonUniqueResourceSystem<T, In, Out, F, WRITE> {
        NonUniqueResourceSystem {
            non_unique_resource_ref,
            system_meta: SystemMeta::new::<Self>(),
            function,
            _phantom: PhantomData,
        }
    }
}

impl<T, In, Out, F, const WRITE: bool> System for NonUniqueResourceSystem<T, In, Out, F, WRITE>
where
    T: Sync + Send + 'static,
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    F: Fn(In, Ptr) -> Out + Sync + Send + 'static,
{
    type In = In;
    type Out = Out;

    fn name(&self) -> Cow<'static, str> {
        self.system_meta.name.clone()
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        self.system_meta.component_access_set.combined_access()
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.system_meta.archetype_component_access
    }

    fn is_send(&self) -> bool {
        true
    }

    fn is_exclusive(&self) -> bool {
        WRITE
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: UnsafeWorldCell) -> Self::Out {
        let ptr = world.get_non_unique_resource_by_id(
            self.non_unique_resource_ref.component_id,
            self.non_unique_resource_ref.index,
        );
        (self.function)(input, ptr)
    }

    fn apply_deferred(&mut self, _world: &mut World) {}

    fn initialize(&mut self, _world: &mut World) {}

    fn update_archetype_component_access(&mut self, _world: UnsafeWorldCell) {
        // TODO: is it correct?
        // TODO: panic somewhere if the same system (this and combined with this)
        //   accesses the same resource incompatibly.
        if WRITE {
            self.system_meta
                .component_access_set
                .add_unfiltered_write(self.non_unique_resource_ref.component_id);

            self.system_meta
                .archetype_component_access
                .add_write(self.non_unique_resource_ref.archetype_component_id);
        } else {
            self.system_meta
                .component_access_set
                .add_unfiltered_read(self.non_unique_resource_ref.component_id);

            self.system_meta
                .archetype_component_access
                .add_read(self.non_unique_resource_ref.archetype_component_id);
        }
    }

    fn check_change_tick(&mut self, change_tick: Tick) {
        check_system_change_tick(
            &mut self.system_meta.last_run,
            change_tick,
            self.system_meta.name.as_ref(),
        );
    }

    fn get_last_run(&self) -> Tick {
        self.system_meta.last_run
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.system_meta.last_run = last_run;
    }
}

impl<T: Sync + Send + 'static> NonUniqueResourceRef<T> {
    // Technically this function is unsafe because argument must match.
    pub(crate) fn new(
        component_id: ComponentId,
        index: TableRow,
        archetype_component_id: ArchetypeComponentId,
    ) -> Self {
        NonUniqueResourceRef {
            component_id,
            index,
            archetype_component_id,
            _phantom: PhantomData,
        }
    }

    /// Read the value if it is set, return `None` otherwise.
    pub fn read_opt_system(&self) -> impl System<In = (), Out = Option<T>> {
        // SAFETY: `NonUniqueResourceSystem` guarantees that the pointer is correct.
        NonUniqueResourceSystem::<_, _, _, _, true>::new(*self, |(), ptr| unsafe {
            ptr.assert_unique()
                .deref_mut::<NonUniqueResourceEntry<T>>()
                .value
                .take()
        })
    }

    /// Read the value if it is set, panic otherwise.
    pub fn read_system(&self) -> impl System<In = (), Out = T> {
        // Slightly inefficient: we store index twice in the resulting system.
        let index = self.index.index();
        self.read_opt_system().map(move |opt| match opt {
            Some(v) => v,
            None => panic!(
                "Non-unique resource {}.{} is not set",
                any::type_name::<T>(),
                index
            ),
        })
    }

    /// Read the value if it is set, return `None` otherwise.
    ///
    /// Keeps the value in the resource.
    pub fn read_opt_clone_system(&self) -> impl System<In = (), Out = Option<T>>
    where
        T: Clone,
    {
        // SAFETY: `NonUniqueResourceSystem` guarantees that the pointer is correct.
        NonUniqueResourceSystem::<_, _, _, _, false>::new(*self, |(), ptr| unsafe {
            ptr.deref::<NonUniqueResourceEntry<T>>().value.clone()
        })
    }

    /// Read the value if it is set, panic otherwise.
    ///
    /// Keeps the value in the resource.
    pub fn read_clone_system(&self) -> impl System<In = (), Out = T>
    where
        T: Clone,
    {
        // Slightly inefficient: we store index twice in the resulting system.
        let index = self.index.index();
        self.read_opt_clone_system().map(move |opt| match opt {
            Some(v) => v,
            None => panic!(
                "Non-unique resource {}.{} is not set",
                any::type_name::<T>(),
                index
            ),
        })
    }

    /// Write the value overwriting the previous one.
    pub fn write_opt_system(&self) -> impl System<In = Option<T>, Out = ()> {
        // SAFETY: `NonUniqueResourceSystem` guarantees that the pointer is correct.
        NonUniqueResourceSystem::<_, _, _, _, true>::new(*self, |value, ptr| unsafe {
            ptr.assert_unique()
                .deref_mut::<NonUniqueResourceEntry<T>>()
                .value = value;
        })
    }

    /// Write the value overwriting the previous one.
    pub fn write_system(&self) -> impl System<In = T, Out = ()> {
        (|In(value)| Some(value)).pipe(self.write_opt_system())
    }

    /// Write the given value into the resource.
    pub fn write_value_system(&self, value: T) -> impl System<In = (), Out = ()>
    where
        T: Clone,
    {
        (move || value.clone()).pipe(self.write_system())
    }

    /// Unset the resource.
    pub fn remove_system(&self) -> impl System<In = (), Out = ()> {
        (|| None).pipe(self.write_opt_system())
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::schedule::IntoSystemConfigs;
    use crate::schedule::Schedule;
    use crate::schedule::SystemSet;
    use crate::system::Resource;
    use crate::system::{In, IntoSystem, ResMut};
    use crate::world::World;

    #[test]
    fn test_write_read() {
        let mut world = World::default();

        let res = world.new_non_unique_resource::<String>();

        #[derive(Resource, Default)]
        struct TestState(bool);

        world.init_resource::<TestState>();

        let a = res.write_value_system("a".to_owned());

        let b = res
            .read_system()
            .pipe(|In(v), mut result: ResMut<TestState>| {
                assert_eq!("a", v);
                assert!(!result.0);
                result.0 = true;
            });

        #[derive(SystemSet, Clone, Debug, Eq, PartialEq, Hash)]
        struct Between;

        let mut schedule = Schedule::default();
        schedule.add_systems(a.before(Between));
        schedule.add_systems(b.after(Between));

        schedule.run(&mut world);

        assert!(world.get_resource::<TestState>().unwrap().0);
    }

    #[test]
    fn test_write_read_clone() {
        let mut world = World::default();

        let res = world.new_non_unique_resource::<String>();

        #[derive(Resource, Default)]
        struct TestState {
            b_read: bool,
            c_read: bool,
        }

        world.init_resource::<TestState>();

        let a = res.write_value_system("a".to_owned());

        let b = res
            .read_clone_system()
            .pipe(|In(v): In<String>, mut result: ResMut<TestState>| {
                assert_eq!("a", v);
                assert!(!result.b_read);
                result.b_read = true;
            });
        let c = res
            .read_clone_system()
            .pipe(|In(v): In<String>, mut result: ResMut<TestState>| {
                assert_eq!("a", v);
                assert!(!result.c_read);
                result.c_read = true;
            });

        #[derive(SystemSet, Clone, Debug, Eq, PartialEq, Hash)]
        struct Between;

        let mut schedule = Schedule::default();
        schedule.add_systems(a.before(Between));
        schedule.add_systems(b.after(Between));
        schedule.add_systems(c.after(Between));

        schedule.run(&mut world);

        assert!(world.get_resource::<TestState>().unwrap().b_read);
        assert!(world.get_resource::<TestState>().unwrap().c_read);
    }
}
