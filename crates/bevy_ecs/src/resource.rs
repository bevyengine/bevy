//! Resource is data stored in the world uniquely by type.
//!
//! See the [`Resource`] documentation for more details.

use crate::change_detection::{DetectChangesMut, Mut, Ticks, TicksMut};
use crate::component::{ComponentId, ComponentTicks, Tick};
use crate::impl_macros::{
    change_detection_impl, change_detection_mut_impl, impl_debug, impl_methods,
};
use crate::system::{ReadOnlySystemParam, SystemMeta, SystemParam};
use crate::world::unsafe_world_cell::UnsafeWorldCell;
use crate::world::World;
pub use bevy_ecs_macros::Resource;
use bevy_ptr::UnsafeCellDeref;
use std::fmt::Debug;
use std::ops::Deref;

/// A type that can be inserted into a [`World`] as a singleton.
///
/// You can access resource data in systems using the [`Res`] and [`ResMut`] system parameters
///
/// Only one resource of each type can be stored in a [`World`] at any given time.
///
/// # Examples
///
/// ```
/// # let mut world = World::default();
/// # let mut schedule = Schedule::default();
/// # use bevy_ecs::prelude::*;
/// #[derive(Resource)]
/// struct MyResource { value: u32 }
///
/// world.insert_resource(MyResource { value: 42 });
///
/// fn read_resource_system(resource: Res<MyResource>) {
///     assert_eq!(resource.value, 42);
/// }
///
/// fn write_resource_system(mut resource: ResMut<MyResource>) {
///     assert_eq!(resource.value, 42);
///     resource.value = 0;
///     assert_eq!(resource.value, 0);
/// }
/// # schedule.add_systems((read_resource_system, write_resource_system).chain());
/// # schedule.run(&mut world);
/// ```
///
/// # `!Sync` Resources
/// A `!Sync` type cannot implement `Resource`. However, it is possible to wrap a `Send` but not `Sync`
/// type in [`SyncCell`] or the currently unstable [`Exclusive`] to make it `Sync`. This forces only
/// having mutable access (`&mut T` only, never `&T`), but makes it safe to reference across multiple
/// threads.
///
/// This will fail to compile since `RefCell` is `!Sync`.
/// ```compile_fail
/// # use std::cell::RefCell;
/// # use bevy_ecs::resource::Resource;
///
/// #[derive(Resource)]
/// struct NotSync {
///    counter: RefCell<usize>,
/// }
/// ```
///
/// This will compile since the `RefCell` is wrapped with `SyncCell`.
/// ```
/// # use std::cell::RefCell;
/// # use bevy_ecs::resource::Resource;
/// # use bevy_utils::synccell::SyncCell;
///
/// #[derive(Resource)]
/// struct ActuallySync {
///    counter: SyncCell<RefCell<usize>>,
/// }
/// ```
///
/// [`SyncCell`]: bevy_utils::synccell::SyncCell
/// [`Exclusive`]: https://doc.rust-lang.org/nightly/std/sync/struct.Exclusive.html
pub trait Resource: Send + Sync + 'static {}

/// Shared borrow of a [`Resource`].
///
/// See the [`Resource`] documentation for usage.
///
/// If you need a unique mutable borrow, use [`ResMut`] instead.
///
/// # Panics
///
/// Panics when used as a [`SystemParameter`](crate::system::SystemParam) if the resource does not exist.
///
/// Use `Option<Res<T>>` instead if the resource might not always exist.
pub struct Res<'w, T: ?Sized + Resource> {
    pub(crate) value: &'w T,
    pub(crate) ticks: Ticks<'w>,
}

impl<'w, T: Resource> Res<'w, T> {
    /// Copies a reference to a resource.
    ///
    /// Note that unless you actually need an instance of `Res<T>`, you should
    /// prefer to just convert it to `&T` which can be freely copied.
    #[allow(clippy::should_implement_trait)]
    pub fn clone(this: &Self) -> Self {
        Self {
            value: this.value,
            ticks: this.ticks.clone(),
        }
    }

    /// Due to lifetime limitations of the `Deref` trait, this method can be used to obtain a
    /// reference of the [`Resource`] with a lifetime bound to `'w` instead of the lifetime of the
    /// struct itself.
    pub fn into_inner(self) -> &'w T {
        self.value
    }
}

change_detection_impl!(Res<'w, T>, T, Resource);
impl_debug!(Res<'w, T>, Resource);

impl<'w, 'a, T: Resource> IntoIterator for &'a Res<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}

impl<'w, T: Resource> From<ResMut<'w, T>> for Res<'w, T> {
    fn from(res: ResMut<'w, T>) -> Self {
        Self {
            value: res.value,
            ticks: res.ticks.into(),
        }
    }
}

/// Unique mutable borrow of a [`Resource`].
///
/// See the [`Resource`] documentation for usage.
///
/// If you need a shared borrow, use [`Res`] instead.
///
/// # Panics
///
/// Panics when used as a [`SystemParam`] if the resource does not exist.
///
/// Use `Option<ResMut<T>>` instead if the resource might not always exist.
pub struct ResMut<'a, T: ?Sized + Resource> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: TicksMut<'a>,
}

change_detection_impl!(ResMut<'a, T>, T, Resource);
change_detection_mut_impl!(ResMut<'a, T>, T, Resource);
impl_debug!(ResMut<'a, T>, Resource);
impl_methods!(ResMut<'a, T>, T, Resource);

impl<'w, 'a, T: Resource> IntoIterator for &'a ResMut<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}

impl<'w, 'a, T: Resource> IntoIterator for &'a mut ResMut<'w, T>
where
    &'a mut T: IntoIterator,
{
    type Item = <&'a mut T as IntoIterator>::Item;
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.set_changed();
        self.value.into_iter()
    }
}

impl<'a, T: Resource> From<ResMut<'a, T>> for Mut<'a, T> {
    /// Convert this `ResMut` into a `Mut`. This allows keeping the change-detection feature of `Mut`
    /// while losing the specificity of `ResMut` for resources.
    fn from(other: ResMut<'a, T>) -> Mut<'a, T> {
        Mut {
            value: other.value,
            ticks: other.ticks,
        }
    }
}

// SAFETY: Res only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for Res<'a, T> {}

// SAFETY: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: Resource> SystemParam for Res<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = Res<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let component_id = world.initialize_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access();
        assert!(
            !combined_access.has_write(component_id),
            "error[B0002]: Res<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access.",
            std::any::type_name::<T>(),
            system_meta.name,
        );
        system_meta
            .component_access_set
            .add_unfiltered_read(component_id);

        let archetype_component_id = world
            .get_resource_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_read(archetype_component_id);

        component_id
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks) = world
            .get_resource_with_ticks(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        Res {
            value: ptr.deref(),
            ticks: Ticks {
                added: ticks.added.deref(),
                changed: ticks.changed.deref(),
                last_run: system_meta.last_run,
                this_run: change_tick,
            },
        }
    }
}

// SAFETY: Only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for Option<Res<'a, T>> {}

// SAFETY: this impl defers to `Res`, which initializes and validates the correct world access.
unsafe impl<'a, T: Resource> SystemParam for Option<Res<'a, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<Res<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        Res::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_resource_with_ticks(component_id)
            .map(|(ptr, ticks)| Res {
                value: ptr.deref(),
                ticks: Ticks {
                    added: ticks.added.deref(),
                    changed: ticks.changed.deref(),
                    last_run: system_meta.last_run,
                    this_run: change_tick,
                },
            })
    }
}

// SAFETY: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: Resource> SystemParam for ResMut<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = ResMut<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let component_id = world.initialize_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access();
        if combined_access.has_write(component_id) {
            panic!(
                "error[B0002]: ResMut<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        } else if combined_access.has_read(component_id) {
            panic!(
                "error[B0002]: ResMut<{}> in system {} conflicts with a previous Res<{0}> access. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        }
        system_meta
            .component_access_set
            .add_unfiltered_write(component_id);

        let archetype_component_id = world
            .get_resource_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_write(archetype_component_id);

        component_id
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let value = world
            .get_resource_mut_by_id(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        ResMut {
            value: value.value.deref_mut::<T>(),
            ticks: TicksMut {
                added: value.ticks.added,
                changed: value.ticks.changed,
                last_run: system_meta.last_run,
                this_run: change_tick,
            },
        }
    }
}

// SAFETY: this impl defers to `ResMut`, which initializes and validates the correct world access.
unsafe impl<'a, T: Resource> SystemParam for Option<ResMut<'a, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<ResMut<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        ResMut::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_resource_mut_by_id(component_id)
            .map(|value| ResMut {
                value: value.value.deref_mut::<T>(),
                ticks: TicksMut {
                    added: value.ticks.added,
                    changed: value.ticks.changed,
                    last_run: system_meta.last_run,
                    this_run: change_tick,
                },
            })
    }
}

/// Shared borrow of a non-[`Send`] resource.
///
/// Only `Send` resources may be accessed with the [`Res`] [`SystemParam`].
/// In case that the resource does not implement `Send`, this `SystemParam` wrapper can be used.
/// This will instruct the scheduler to instead run the system on the main thread
/// so that it doesn't send the resource over to another thread.
///
/// # Panics
///
/// Panics when used as a `SystemParameter` if the resource does not exist.
///
/// Use `Option<NonSend<T>>` instead if the resource might not always exist.
pub struct NonSend<'w, T: 'static> {
    pub(crate) value: &'w T,
    ticks: ComponentTicks,
    last_run: Tick,
    this_run: Tick,
}

// SAFETY: Only reads a single World non-send resource
unsafe impl<'w, T> ReadOnlySystemParam for NonSend<'w, T> {}

// SAFETY: NonSendComponentId and ArchetypeComponentId access is applied to SystemMeta. If this
// NonSend conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: 'static> SystemParam for NonSend<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = NonSend<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.set_non_send();

        let component_id = world.initialize_non_send_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access();
        assert!(
            !combined_access.has_write(component_id),
            "error[B0002]: NonSend<{}> in system {} conflicts with a previous mutable resource access ({0}). Consider removing the duplicate access.",
            std::any::type_name::<T>(),
            system_meta.name,
        );
        system_meta
            .component_access_set
            .add_unfiltered_read(component_id);

        let archetype_component_id = world
            .get_non_send_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_read(archetype_component_id);

        component_id
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks) = world
            .get_non_send_with_ticks(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Non-send resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });

        NonSend {
            value: ptr.deref(),
            ticks: ticks.read(),
            last_run: system_meta.last_run,
            this_run: change_tick,
        }
    }
}

impl<'w, T> Debug for NonSend<'w, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NonSend").field(&self.value).finish()
    }
}

impl<'w, T: 'static> NonSend<'w, T> {
    /// Returns `true` if the resource was added after the system last ran.
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_run, self.this_run)
    }

    /// Returns `true` if the resource was added or mutably dereferenced after the system last ran.
    pub fn is_changed(&self) -> bool {
        self.ticks.is_changed(self.last_run, self.this_run)
    }
}

impl<'w, T> Deref for NonSend<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}
impl<'a, T> From<NonSendMut<'a, T>> for NonSend<'a, T> {
    fn from(nsm: NonSendMut<'a, T>) -> Self {
        Self {
            value: nsm.value,
            ticks: ComponentTicks {
                added: nsm.ticks.added.to_owned(),
                changed: nsm.ticks.changed.to_owned(),
            },
            this_run: nsm.ticks.this_run,
            last_run: nsm.ticks.last_run,
        }
    }
}

// SAFETY: Only reads a single World non-send resource
unsafe impl<T: 'static> ReadOnlySystemParam for Option<NonSend<'_, T>> {}

// SAFETY: this impl defers to `NonSend`, which initializes and validates the correct world access.
unsafe impl<T: 'static> SystemParam for Option<NonSend<'_, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<NonSend<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        NonSend::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_non_send_with_ticks(component_id)
            .map(|(ptr, ticks)| NonSend {
                value: ptr.deref(),
                ticks: ticks.read(),
                last_run: system_meta.last_run,
                this_run: change_tick,
            })
    }
}

/// Unique borrow of a non-[`Send`] resource.
///
/// Only [`Send`] resources may be accessed with the [`ResMut`] [`SystemParam`]. In case that the
/// resource does not implement `Send`, this `SystemParam` wrapper can be used. This will instruct
/// the scheduler to instead run the system on the main thread so that it doesn't send the resource
/// over to another thread.
///
/// # Panics
///
/// Panics when used as a `SystemParameter` if the resource does not exist.
///
/// Use `Option<NonSendMut<T>>` instead if the resource might not always exist.
pub struct NonSendMut<'a, T: ?Sized + 'static> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: TicksMut<'a>,
}

change_detection_impl!(NonSendMut<'a, T>, T,);
change_detection_mut_impl!(NonSendMut<'a, T>, T,);
impl_methods!(NonSendMut<'a, T>, T,);
impl_debug!(NonSendMut<'a, T>,);

// SAFETY: NonSendMut ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this
// NonSendMut conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: 'static> SystemParam for NonSendMut<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = NonSendMut<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.set_non_send();

        let component_id = world.initialize_non_send_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access();
        if combined_access.has_write(component_id) {
            panic!(
                "error[B0002]: NonSendMut<{}> in system {} conflicts with a previous mutable resource access ({0}). Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        } else if combined_access.has_read(component_id) {
            panic!(
                "error[B0002]: NonSendMut<{}> in system {} conflicts with a previous immutable resource access ({0}). Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        }
        system_meta
            .component_access_set
            .add_unfiltered_write(component_id);

        let archetype_component_id = world
            .get_non_send_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_write(archetype_component_id);

        component_id
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks) = world
            .get_non_send_with_ticks(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Non-send resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        NonSendMut {
            value: ptr.assert_unique().deref_mut(),
            ticks: TicksMut::from_tick_cells(ticks, system_meta.last_run, change_tick),
        }
    }
}

impl<'a, T: 'static> From<NonSendMut<'a, T>> for Mut<'a, T> {
    /// Convert this `NonSendMut` into a `Mut`. This allows keeping the change-detection feature of `Mut`
    /// while losing the specificity of `NonSendMut`.
    fn from(other: NonSendMut<'a, T>) -> Mut<'a, T> {
        Mut {
            value: other.value,
            ticks: other.ticks,
        }
    }
}

// SAFETY: this impl defers to `NonSendMut`, which initializes and validates the correct world access.
unsafe impl<'a, T: 'static> SystemParam for Option<NonSendMut<'a, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<NonSendMut<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        NonSendMut::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_non_send_with_ticks(component_id)
            .map(|(ptr, ticks)| NonSendMut {
                value: ptr.assert_unique().deref_mut(),
                ticks: TicksMut::from_tick_cells(ticks, system_meta.last_run, change_tick),
            })
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::change_detection::{Mut, TicksMut};
    use crate::component::{ComponentTicks, Tick};
    use crate::resource::Resource;
    use crate::resource::{NonSend, NonSendMut, ResMut};
    use crate::schedule::Schedule;
    use crate::system::{IntoSystem, ParamSet};
    use crate::world::World;

    #[derive(Resource)]
    struct R;

    #[derive(Resource, PartialEq, Debug)]
    enum SystemRan {
        Yes,
        No,
    }

    fn run_system<Marker, S: IntoSystem<(), (), Marker>>(world: &mut World, system: S) {
        let mut schedule = Schedule::default();
        schedule.add_systems(system);
        schedule.run(world);
    }

    #[test]
    fn non_send_option_system() {
        let mut world = World::default();

        world.insert_resource(SystemRan::No);
        struct NotSend1(std::rc::Rc<i32>);
        struct NotSend2(std::rc::Rc<i32>);
        world.insert_non_send_resource(NotSend1(std::rc::Rc::new(0)));

        fn sys(
            op: Option<NonSend<NotSend1>>,
            mut _op2: Option<NonSendMut<NotSend2>>,
            mut system_ran: ResMut<SystemRan>,
        ) {
            op.expect("NonSend should exist");
            *system_ran = SystemRan::Yes;
        }

        run_system(&mut world, sys);
        // ensure the system actually ran
        assert_eq!(*world.resource::<SystemRan>(), SystemRan::Yes);
    }

    #[test]
    fn non_send_system() {
        let mut world = World::default();

        world.insert_resource(SystemRan::No);
        struct NotSend1(std::rc::Rc<i32>);
        struct NotSend2(std::rc::Rc<i32>);

        world.insert_non_send_resource(NotSend1(std::rc::Rc::new(1)));
        world.insert_non_send_resource(NotSend2(std::rc::Rc::new(2)));

        fn sys(
            _op: NonSend<NotSend1>,
            mut _op2: NonSendMut<NotSend2>,
            mut system_ran: ResMut<SystemRan>,
        ) {
            *system_ran = SystemRan::Yes;
        }

        run_system(&mut world, sys);
        assert_eq!(*world.resource::<SystemRan>(), SystemRan::Yes);
    }

    #[test]
    fn mut_from_non_send_mut() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let ticks = TicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            last_run: Tick::new(3),
            this_run: Tick::new(4),
        };
        let mut res = R {};
        let non_send_mut = NonSendMut {
            value: &mut res,
            ticks,
        };

        let into_mut: Mut<R> = non_send_mut.into();
        assert_eq!(1, into_mut.ticks.added.get());
        assert_eq!(2, into_mut.ticks.changed.get());
        assert_eq!(3, into_mut.ticks.last_run.get());
        assert_eq!(4, into_mut.ticks.this_run.get());
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/10207.
    #[test]
    fn param_set_non_send_first() {
        fn non_send_param_set(mut p: ParamSet<(NonSend<*mut u8>, ())>) {
            let _ = p.p0();
            p.p1();
        }

        let mut world = World::new();
        world.insert_non_send_resource(std::ptr::null_mut::<u8>());
        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems((non_send_param_set, non_send_param_set, non_send_param_set));
        schedule.run(&mut world);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/10207.
    #[test]
    fn param_set_non_send_second() {
        fn non_send_param_set(mut p: ParamSet<((), NonSendMut<*mut u8>)>) {
            p.p0();
            let _ = p.p1();
        }

        let mut world = World::new();
        world.insert_non_send_resource(std::ptr::null_mut::<u8>());
        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems((non_send_param_set, non_send_param_set, non_send_param_set));
        schedule.run(&mut world);
    }
}
