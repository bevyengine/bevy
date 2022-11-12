//! Types that detect when their internal data mutate.

use crate::{
    component::{Tick, TickCells},
    ptr::PtrMut,
    system::Resource,
};
use bevy_ptr::UnsafeCellDeref;
use std::ops::{Deref, DerefMut};

/// The (arbitrarily chosen) minimum number of world tick increments between `check_tick` scans.
///
/// Change ticks can only be scanned when systems aren't running. Thus, if the threshold is `N`,
/// the maximum is `2 * N - 1` (i.e. the world ticks `N - 1` times, then `N` times).
///
/// If no change is older than `u32::MAX - (2 * N - 1)` following a scan, none of their ages can
/// overflow and cause false positives.
// (518,400,000 = 1000 ticks per frame * 144 frames per second * 3600 seconds per hour)
pub const CHECK_TICK_THRESHOLD: u32 = 518_400_000;

/// The maximum change tick difference that won't overflow before the next `check_tick` scan.
///
/// Changes stop being detected once they become this old.
pub const MAX_CHANGE_AGE: u32 = u32::MAX - (2 * CHECK_TICK_THRESHOLD - 1);

/// Types that implement reliable change detection.
///
/// ## Example
/// Using types that implement [`DetectChanges`], such as [`ResMut`], provide
/// a way to query if a value has been mutated in another system.
/// Normally change detecting is triggered by either [`DerefMut`] or [`AsMut`], however
/// it can be manually triggered via [`DetectChanges::set_changed`].
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// #[derive(Resource)]
/// struct MyResource(u32);
///
/// fn my_system(mut resource: ResMut<MyResource>) {
///     if resource.is_changed() {
///         println!("My resource was mutated!");
///     }
///
///    resource.0 = 42; // triggers change detection via [`DerefMut`]
/// }
/// ```
///
pub trait DetectChanges {
    /// The type contained within this smart pointer
    ///
    /// For example, for `Res<T>` this would be `T`.
    type Inner: ?Sized;

    /// Returns `true` if this value was added after the system last ran.
    fn is_added(&self) -> bool;

    /// Returns `true` if this value was added or mutably dereferenced after the system last ran.
    fn is_changed(&self) -> bool;

    /// Flags this value as having been changed.
    ///
    /// Mutably accessing this smart pointer will automatically flag this value as having been changed.
    /// However, mutation through interior mutability requires manual reporting.
    ///
    /// **Note**: This operation cannot be undone.
    fn set_changed(&mut self);

    /// Returns the change tick recording the previous time this data was changed.
    ///
    /// Note that components and resources are also marked as changed upon insertion.
    ///
    /// For comparison, the previous change tick of a system can be read using the
    /// [`SystemChangeTick`](crate::system::SystemChangeTick)
    /// [`SystemParam`](crate::system::SystemParam).
    fn last_changed(&self) -> u32;

    /// Manually sets the change tick recording the previous time this data was mutated.
    ///
    /// # Warning
    /// This is a complex and error-prone operation, primarily intended for use with rollback networking strategies.
    /// If you merely want to flag this data as changed, use [`set_changed`](DetectChanges::set_changed) instead.
    /// If you want to avoid triggering change detection, use [`bypass_change_detection`](DetectChanges::bypass_change_detection) instead.
    fn set_last_changed(&mut self, last_change_tick: u32);

    /// Manually bypasses change detection, allowing you to mutate the underlying value without updating the change tick.
    ///
    /// # Warning
    /// This is a risky operation, that can have unexpected consequences on any system relying on this code.
    /// However, it can be an essential escape hatch when, for example,
    /// you are trying to synchronize representations using change detection and need to avoid infinite recursion.
    fn bypass_change_detection(&mut self) -> &mut Self::Inner;
}

macro_rules! change_detection_impl {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> DetectChanges for $name<$($generics),*> {
            type Inner = $target;

            #[inline]
            fn is_added(&self) -> bool {
                self.ticks
                    .added
                    .is_older_than(self.ticks.last_change_tick, self.ticks.change_tick)
            }

            #[inline]
            fn is_changed(&self) -> bool {
                self.ticks
                    .changed
                    .is_older_than(self.ticks.last_change_tick, self.ticks.change_tick)
            }

            #[inline]
            fn set_changed(&mut self) {
                self.ticks
                    .changed
                    .set_changed(self.ticks.change_tick);
            }

            #[inline]
            fn last_changed(&self) -> u32 {
                self.ticks.last_change_tick
            }

            #[inline]
            fn set_last_changed(&mut self, last_change_tick: u32) {
                self.ticks.last_change_tick = last_change_tick
            }

            #[inline]
            fn bypass_change_detection(&mut self) -> &mut Self::Inner {
                self.value
            }
        }

        impl<$($generics),*: ?Sized $(+ $traits)?> Deref for $name<$($generics),*> {
            type Target = $target;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.value
            }
        }

        impl<$($generics),* : ?Sized $(+ $traits)?> DerefMut for $name<$($generics),*> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.set_changed();
                self.value
            }
        }

        impl<$($generics),* $(: $traits)?> AsRef<$target> for $name<$($generics),*> {
            #[inline]
            fn as_ref(&self) -> &$target {
                self.deref()
            }
        }

        impl<$($generics),* $(: $traits)?> AsMut<$target> for $name<$($generics),*> {
            #[inline]
            fn as_mut(&mut self) -> &mut $target {
                self.deref_mut()
            }
        }
    };
}

macro_rules! impl_methods {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> $name<$($generics),*> {
            /// Consume `self` and return a mutable reference to the
            /// contained value while marking `self` as "changed".
            #[inline]
            pub fn into_inner(mut self) -> &'a mut $target {
                self.set_changed();
                self.value
            }

            /// Maps to an inner value by applying a function to the contained reference, without flagging a change.
            ///
            /// You should never modify the argument passed to the closure -- if you want to modify the data
            /// without flagging a change, consider using [`DetectChanges::bypass_change_detection`] to make your intent explicit.
            ///
            /// ```rust
            /// # use bevy_ecs::prelude::*;
            /// # pub struct Vec2;
            /// # impl Vec2 { pub const ZERO: Self = Self; }
            /// # #[derive(Component)] pub struct Transform { translation: Vec2 }
            /// # mod my_utils {
            /// #   pub fn set_if_not_equal<T>(x: bevy_ecs::prelude::Mut<T>, val: T) { unimplemented!() }
            /// # }
            /// // When run, zeroes the translation of every entity.
            /// fn reset_positions(mut transforms: Query<&mut Transform>) {
            ///     for transform in &mut transforms {
            ///         // We pinky promise not to modify `t` within the closure.
            ///         // Breaking this promise will result in logic errors, but will never cause undefined behavior.
            ///         let translation = transform.map_unchanged(|t| &mut t.translation);
            ///         // Only reset the translation if it isn't already zero;
            ///         my_utils::set_if_not_equal(translation, Vec2::ZERO);
            ///     }
            /// }
            /// # bevy_ecs::system::assert_is_system(reset_positions);
            /// ```
            pub fn map_unchanged<U: ?Sized>(self, f: impl FnOnce(&mut $target) -> &mut U) -> Mut<'a, U> {
                Mut {
                    value: f(self.value),
                    ticks: self.ticks,
                }
            }
        }
    };
}

macro_rules! impl_debug {
    ($name:ident < $( $generics:tt ),+ >, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> std::fmt::Debug for $name<$($generics),*>
            where T: std::fmt::Debug
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.value)
                    .finish()
            }
        }

    };
}

pub(crate) struct Ticks<'a> {
    pub(crate) added: &'a mut Tick,
    pub(crate) changed: &'a mut Tick,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

impl<'a> Ticks<'a> {
    /// # Safety
    /// This should never alias the underlying ticks. All access must be unique.
    #[inline]
    pub(crate) unsafe fn from_tick_cells(
        cells: TickCells<'a>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        Self {
            added: cells.added.deref_mut(),
            changed: cells.changed.deref_mut(),
            last_change_tick,
            change_tick,
        }
    }
}

/// Unique mutable borrow of a [`Resource`].
///
/// See the [`Resource`] documentation for usage.
///
/// If you need a shared borrow, use [`Res`](crate::system::Res) instead.
///
/// # Panics
///
/// Panics when used as a [`SystemParam`](crate::system::SystemParam) if the resource does not exist.
///
/// Use `Option<ResMut<T>>` instead if the resource might not always exist.
pub struct ResMut<'a, T: ?Sized + Resource> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: Ticks<'a>,
}

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

change_detection_impl!(ResMut<'a, T>, T, Resource);
impl_methods!(ResMut<'a, T>, T, Resource);
impl_debug!(ResMut<'a, T>, Resource);

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

/// Unique borrow of a non-[`Send`] resource.
///
/// Only [`Send`] resources may be accessed with the [`ResMut`] [`SystemParam`](crate::system::SystemParam). In case that the
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
    pub(crate) ticks: Ticks<'a>,
}

change_detection_impl!(NonSendMut<'a, T>, T,);
impl_methods!(NonSendMut<'a, T>, T,);
impl_debug!(NonSendMut<'a, T>,);

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

/// Unique mutable borrow of an entity's component
pub struct Mut<'a, T: ?Sized> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: Ticks<'a>,
}

impl<'w, 'a, T> IntoIterator for &'a Mut<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}

impl<'w, 'a, T> IntoIterator for &'a mut Mut<'w, T>
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

change_detection_impl!(Mut<'a, T>, T,);
impl_methods!(Mut<'a, T>, T,);
impl_debug!(Mut<'a, T>,);

/// Unique mutable borrow of resources or an entity's component.
///
/// Similar to [`Mut`], but not generic over the component type, instead
/// exposing the raw pointer as a `*mut ()`.
///
/// Usually you don't need to use this and can instead use the APIs returning a
/// [`Mut`], but in situations where the types are not known at compile time
/// or are defined outside of rust this can be used.
pub struct MutUntyped<'a> {
    pub(crate) value: PtrMut<'a>,
    pub(crate) ticks: Ticks<'a>,
}

impl<'a> MutUntyped<'a> {
    /// Returns the pointer to the value, without marking it as changed.
    ///
    /// In order to mark the value as changed, you need to call [`set_changed`](DetectChanges::set_changed) manually.
    #[inline]
    pub fn into_inner(self) -> PtrMut<'a> {
        self.value
    }
}

impl<'a> DetectChanges for MutUntyped<'a> {
    type Inner = PtrMut<'a>;

    #[inline]
    fn is_added(&self) -> bool {
        self.ticks
            .added
            .is_older_than(self.ticks.last_change_tick, self.ticks.change_tick)
    }

    #[inline]
    fn is_changed(&self) -> bool {
        self.ticks
            .changed
            .is_older_than(self.ticks.last_change_tick, self.ticks.change_tick)
    }

    #[inline]
    fn set_changed(&mut self) {
        self.ticks.changed.set_changed(self.ticks.change_tick);
    }

    #[inline]
    fn last_changed(&self) -> u32 {
        self.ticks.last_change_tick
    }

    #[inline]
    fn set_last_changed(&mut self, last_change_tick: u32) {
        self.ticks.last_change_tick = last_change_tick;
    }

    #[inline]
    fn bypass_change_detection(&mut self) -> &mut Self::Inner {
        &mut self.value
    }
}

impl std::fmt::Debug for MutUntyped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MutUntyped")
            .field(&self.value.as_ptr())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs_macros::Resource;

    use crate::{
        self as bevy_ecs,
        change_detection::{Mut, NonSendMut, ResMut, Ticks, CHECK_TICK_THRESHOLD, MAX_CHANGE_AGE},
        component::{Component, ComponentTicks, Tick},
        query::ChangeTrackers,
        system::{IntoSystem, Query, System},
        world::World,
    };

    #[derive(Component)]
    struct C;

    #[derive(Resource)]
    struct R;

    #[test]
    fn change_expiration() {
        fn change_detected(query: Query<ChangeTrackers<C>>) -> bool {
            query.single().is_changed()
        }

        fn change_expired(query: Query<ChangeTrackers<C>>) -> bool {
            query.single().is_changed()
        }

        let mut world = World::new();

        // component added: 1, changed: 1
        world.spawn(C);

        let mut change_detected_system = IntoSystem::into_system(change_detected);
        let mut change_expired_system = IntoSystem::into_system(change_expired);
        change_detected_system.initialize(&mut world);
        change_expired_system.initialize(&mut world);

        // world: 1, system last ran: 0, component changed: 1
        // The spawn will be detected since it happened after the system "last ran".
        assert!(change_detected_system.run((), &mut world));

        // world: 1 + MAX_CHANGE_AGE
        let change_tick = world.change_tick.get_mut();
        *change_tick = change_tick.wrapping_add(MAX_CHANGE_AGE);

        // Both the system and component appeared `MAX_CHANGE_AGE` ticks ago.
        // Since we clamp things to `MAX_CHANGE_AGE` for determinism,
        // `ComponentTicks::is_changed` will now see `MAX_CHANGE_AGE > MAX_CHANGE_AGE`
        // and return `false`.
        assert!(!change_expired_system.run((), &mut world));
    }

    #[test]
    fn change_tick_wraparound() {
        fn change_detected(query: Query<ChangeTrackers<C>>) -> bool {
            query.single().is_changed()
        }

        let mut world = World::new();
        world.last_change_tick = u32::MAX;
        *world.change_tick.get_mut() = 0;

        // component added: 0, changed: 0
        world.spawn(C);

        // system last ran: u32::MAX
        let mut change_detected_system = IntoSystem::into_system(change_detected);
        change_detected_system.initialize(&mut world);

        // Since the world is always ahead, as long as changes can't get older than `u32::MAX` (which we ensure),
        // the wrapping difference will always be positive, so wraparound doesn't matter.
        assert!(change_detected_system.run((), &mut world));
    }

    #[test]
    fn change_tick_scan() {
        let mut world = World::new();

        // component added: 1, changed: 1
        world.spawn(C);

        // a bunch of stuff happens, the component is now older than `MAX_CHANGE_AGE`
        *world.change_tick.get_mut() += MAX_CHANGE_AGE + CHECK_TICK_THRESHOLD;
        let change_tick = world.change_tick();

        let mut query = world.query::<ChangeTrackers<C>>();
        for tracker in query.iter(&world) {
            let ticks_since_insert = change_tick.wrapping_sub(tracker.component_ticks.added.tick);
            let ticks_since_change = change_tick.wrapping_sub(tracker.component_ticks.changed.tick);
            assert!(ticks_since_insert > MAX_CHANGE_AGE);
            assert!(ticks_since_change > MAX_CHANGE_AGE);
        }

        // scan change ticks and clamp those at risk of overflow
        world.check_change_ticks();

        for tracker in query.iter(&world) {
            let ticks_since_insert = change_tick.wrapping_sub(tracker.component_ticks.added.tick);
            let ticks_since_change = change_tick.wrapping_sub(tracker.component_ticks.changed.tick);
            assert!(ticks_since_insert == MAX_CHANGE_AGE);
            assert!(ticks_since_change == MAX_CHANGE_AGE);
        }
    }

    #[test]
    fn mut_from_res_mut() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let ticks = Ticks {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            last_change_tick: 3,
            change_tick: 4,
        };
        let mut res = R {};
        let res_mut = ResMut {
            value: &mut res,
            ticks,
        };

        let into_mut: Mut<R> = res_mut.into();
        assert_eq!(1, into_mut.ticks.added.tick);
        assert_eq!(2, into_mut.ticks.changed.tick);
        assert_eq!(3, into_mut.ticks.last_change_tick);
        assert_eq!(4, into_mut.ticks.change_tick);
    }

    #[test]
    fn mut_from_non_send_mut() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let ticks = Ticks {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            last_change_tick: 3,
            change_tick: 4,
        };
        let mut res = R {};
        let non_send_mut = NonSendMut {
            value: &mut res,
            ticks,
        };

        let into_mut: Mut<R> = non_send_mut.into();
        assert_eq!(1, into_mut.ticks.added.tick);
        assert_eq!(2, into_mut.ticks.changed.tick);
        assert_eq!(3, into_mut.ticks.last_change_tick);
        assert_eq!(4, into_mut.ticks.change_tick);
    }

    #[test]
    fn map_mut() {
        use super::*;
        struct Outer(i64);

        let (last_change_tick, change_tick) = (2, 3);
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let ticks = Ticks {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            last_change_tick,
            change_tick,
        };

        let mut outer = Outer(0);
        let ptr = Mut {
            value: &mut outer,
            ticks,
        };
        assert!(!ptr.is_changed());

        // Perform a mapping operation.
        let mut inner = ptr.map_unchanged(|x| &mut x.0);
        assert!(!inner.is_changed());

        // Mutate the inner value.
        *inner = 64;
        assert!(inner.is_changed());
        // Modifying one field of a component should flag a change for the entire component.
        assert!(component_ticks.is_changed(last_change_tick, change_tick));
    }
}
