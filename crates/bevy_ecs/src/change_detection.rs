//! Types that detect when their internal data mutate.

use crate::{
    component::{Tick, TickCells},
    ptr::PtrMut,
    resource::Resource,
};
use alloc::borrow::ToOwned;
use bevy_ptr::{Ptr, UnsafeCellDeref};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::{
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    panic::Location,
};

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

/// Types that can read change detection information.
/// This change detection is controlled by [`DetectChangesMut`] types such as [`ResMut`].
///
/// ## Example
/// Using types that implement [`DetectChanges`], such as [`Res`], provide
/// a way to query if a value has been mutated in another system.
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// #[derive(Resource)]
/// struct MyResource(u32);
///
/// fn my_system(mut resource: Res<MyResource>) {
///     if resource.is_changed() {
///         println!("My component was mutated!");
///     }
/// }
/// ```
pub trait DetectChanges {
    /// Returns `true` if this value was added after the system last ran.
    fn is_added(&self) -> bool;

    /// Returns `true` if this value was added or mutably dereferenced
    /// either since the last time the system ran or, if the system never ran,
    /// since the beginning of the program.
    ///
    /// To check if the value was mutably dereferenced only,
    /// use `this.is_changed() && !this.is_added()`.
    fn is_changed(&self) -> bool;

    /// Returns the change tick recording the time this data was most recently changed.
    ///
    /// Note that components and resources are also marked as changed upon insertion.
    ///
    /// For comparison, the previous change tick of a system can be read using the
    /// [`SystemChangeTick`](crate::system::SystemChangeTick)
    /// [`SystemParam`](crate::system::SystemParam).
    fn last_changed(&self) -> Tick;

    /// Returns the change tick recording the time this data was added.
    fn added(&self) -> Tick;

    /// The location that last caused this to change.
    fn changed_by(&self) -> MaybeLocation;
}

/// Types that implement reliable change detection.
///
/// ## Example
/// Using types that implement [`DetectChangesMut`], such as [`ResMut`], provide
/// a way to query if a value has been mutated in another system.
/// Normally change detection is triggered by either [`DerefMut`] or [`AsMut`], however
/// it can be manually triggered via [`set_changed`](DetectChangesMut::set_changed).
///
/// To ensure that changes are only triggered when the value actually differs,
/// check if the value would change before assignment, such as by checking that `new != old`.
/// You must be *sure* that you are not mutably dereferencing in this process.
///
/// [`set_if_neq`](DetectChangesMut::set_if_neq) is a helper
/// method for this common functionality.
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
pub trait DetectChangesMut: DetectChanges {
    /// The type contained within this smart pointer
    ///
    /// For example, for `ResMut<T>` this would be `T`.
    type Inner: ?Sized;

    /// Flags this value as having been changed.
    ///
    /// Mutably accessing this smart pointer will automatically flag this value as having been changed.
    /// However, mutation through interior mutability requires manual reporting.
    ///
    /// **Note**: This operation cannot be undone.
    fn set_changed(&mut self);

    /// Flags this value as having been added.
    ///
    /// It is not normally necessary to call this method.
    /// The 'added' tick is set when the value is first added,
    /// and is not normally changed afterwards.
    ///
    /// **Note**: This operation cannot be undone.
    fn set_added(&mut self);

    /// Manually sets the change tick recording the time when this data was last mutated.
    ///
    /// # Warning
    /// This is a complex and error-prone operation, primarily intended for use with rollback networking strategies.
    /// If you merely want to flag this data as changed, use [`set_changed`](DetectChangesMut::set_changed) instead.
    /// If you want to avoid triggering change detection, use [`bypass_change_detection`](DetectChangesMut::bypass_change_detection) instead.
    fn set_last_changed(&mut self, last_changed: Tick);

    /// Manually sets the added tick recording the time when this data was last added.
    ///
    /// # Warning
    /// The caveats of [`set_last_changed`](DetectChangesMut::set_last_changed) apply. This modifies both the added and changed ticks together.
    fn set_last_added(&mut self, last_added: Tick);

    /// Manually bypasses change detection, allowing you to mutate the underlying value without updating the change tick.
    ///
    /// # Warning
    /// This is a risky operation, that can have unexpected consequences on any system relying on this code.
    /// However, it can be an essential escape hatch when, for example,
    /// you are trying to synchronize representations using change detection and need to avoid infinite recursion.
    fn bypass_change_detection(&mut self) -> &mut Self::Inner;

    /// Overwrites this smart pointer with the given value, if and only if `*self != value`.
    /// Returns `true` if the value was overwritten, and returns `false` if it was not.
    ///
    /// This is useful to ensure change detection is only triggered when the underlying value
    /// changes, instead of every time it is mutably accessed.
    ///
    /// If you're dealing with non-trivial structs which have multiple fields of non-trivial size,
    /// then consider applying a `map_unchanged` beforehand to allow changing only the relevant
    /// field and prevent unnecessary copying and cloning.
    /// See the docs of [`Mut::map_unchanged`], [`MutUntyped::map_unchanged`],
    /// [`ResMut::map_unchanged`] or [`NonSendMut::map_unchanged`] for an example
    ///
    /// If you need the previous value, use [`replace_if_neq`](DetectChangesMut::replace_if_neq).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, schedule::common_conditions::resource_changed};
    /// #[derive(Resource, PartialEq, Eq)]
    /// pub struct Score(u32);
    ///
    /// fn reset_score(mut score: ResMut<Score>) {
    ///     // Set the score to zero, unless it is already zero.
    ///     score.set_if_neq(Score(0));
    /// }
    /// # let mut world = World::new();
    /// # world.insert_resource(Score(1));
    /// # let mut score_changed = IntoSystem::into_system(resource_changed::<Score>);
    /// # score_changed.initialize(&mut world);
    /// # score_changed.run((), &mut world);
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems(reset_score);
    /// #
    /// # // first time `reset_score` runs, the score is changed.
    /// # schedule.run(&mut world);
    /// # assert!(score_changed.run((), &mut world).unwrap());
    /// # // second time `reset_score` runs, the score is not changed.
    /// # schedule.run(&mut world);
    /// # assert!(!score_changed.run((), &mut world).unwrap());
    /// ```
    #[inline]
    #[track_caller]
    fn set_if_neq(&mut self, value: Self::Inner) -> bool
    where
        Self::Inner: Sized + PartialEq,
    {
        let old = self.bypass_change_detection();
        if *old != value {
            *old = value;
            self.set_changed();
            true
        } else {
            false
        }
    }

    /// Overwrites this smart pointer with the given value, if and only if `*self != value`,
    /// returning the previous value if this occurs.
    ///
    /// This is useful to ensure change detection is only triggered when the underlying value
    /// changes, instead of every time it is mutably accessed.
    ///
    /// If you're dealing with non-trivial structs which have multiple fields of non-trivial size,
    /// then consider applying a [`map_unchanged`](Mut::map_unchanged) beforehand to allow
    /// changing only the relevant field and prevent unnecessary copying and cloning.
    /// See the docs of [`Mut::map_unchanged`], [`MutUntyped::map_unchanged`],
    /// [`ResMut::map_unchanged`] or [`NonSendMut::map_unchanged`] for an example
    ///
    /// If you don't need the previous value, use [`set_if_neq`](DetectChangesMut::set_if_neq).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, schedule::common_conditions::{resource_changed, on_event}};
    /// #[derive(Resource, PartialEq, Eq)]
    /// pub struct Score(u32);
    ///
    /// #[derive(Event, BufferedEvent, PartialEq, Eq)]
    /// pub struct ScoreChanged {
    ///     current: u32,
    ///     previous: u32,
    /// }
    ///
    /// fn reset_score(mut score: ResMut<Score>, mut score_changed: EventWriter<ScoreChanged>) {
    ///     // Set the score to zero, unless it is already zero.
    ///     let new_score = 0;
    ///     if let Some(Score(previous_score)) = score.replace_if_neq(Score(new_score)) {
    ///         // If `score` change, emit a `ScoreChanged` event.
    ///         score_changed.write(ScoreChanged {
    ///             current: new_score,
    ///             previous: previous_score,
    ///         });
    ///     }
    /// }
    /// # let mut world = World::new();
    /// # world.insert_resource(Events::<ScoreChanged>::default());
    /// # world.insert_resource(Score(1));
    /// # let mut score_changed = IntoSystem::into_system(resource_changed::<Score>);
    /// # score_changed.initialize(&mut world);
    /// # score_changed.run((), &mut world);
    /// #
    /// # let mut score_changed_event = IntoSystem::into_system(on_event::<ScoreChanged>);
    /// # score_changed_event.initialize(&mut world);
    /// # score_changed_event.run((), &mut world);
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems(reset_score);
    /// #
    /// # // first time `reset_score` runs, the score is changed.
    /// # schedule.run(&mut world);
    /// # assert!(score_changed.run((), &mut world).unwrap());
    /// # assert!(score_changed_event.run((), &mut world).unwrap());
    /// # // second time `reset_score` runs, the score is not changed.
    /// # schedule.run(&mut world);
    /// # assert!(!score_changed.run((), &mut world).unwrap());
    /// # assert!(!score_changed_event.run((), &mut world).unwrap());
    /// ```
    #[inline]
    #[must_use = "If you don't need to handle the previous value, use `set_if_neq` instead."]
    fn replace_if_neq(&mut self, value: Self::Inner) -> Option<Self::Inner>
    where
        Self::Inner: Sized + PartialEq,
    {
        let old = self.bypass_change_detection();
        if *old != value {
            let previous = mem::replace(old, value);
            self.set_changed();
            Some(previous)
        } else {
            None
        }
    }

    /// Overwrites this smart pointer with a clone of the given value, if and only if `*self != value`.
    /// Returns `true` if the value was overwritten, and returns `false` if it was not.
    ///
    /// This method is useful when the caller only has a borrowed form of `Inner`,
    /// e.g. when writing a `&str` into a `Mut<String>`.
    ///
    /// # Examples
    /// ```
    /// # extern crate alloc;
    /// # use alloc::borrow::ToOwned;
    /// # use bevy_ecs::{prelude::*, schedule::common_conditions::resource_changed};
    /// #[derive(Resource)]
    /// pub struct Message(String);
    ///
    /// fn update_message(mut message: ResMut<Message>) {
    ///     // Set the score to zero, unless it is already zero.
    ///     ResMut::map_unchanged(message, |Message(msg)| msg).clone_from_if_neq("another string");
    /// }
    /// # let mut world = World::new();
    /// # world.insert_resource(Message("initial string".into()));
    /// # let mut message_changed = IntoSystem::into_system(resource_changed::<Message>);
    /// # message_changed.initialize(&mut world);
    /// # message_changed.run((), &mut world);
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems(update_message);
    /// #
    /// # // first time `reset_score` runs, the score is changed.
    /// # schedule.run(&mut world);
    /// # assert!(message_changed.run((), &mut world).unwrap());
    /// # // second time `reset_score` runs, the score is not changed.
    /// # schedule.run(&mut world);
    /// # assert!(!message_changed.run((), &mut world).unwrap());
    /// ```
    fn clone_from_if_neq<T>(&mut self, value: &T) -> bool
    where
        T: ToOwned<Owned = Self::Inner> + ?Sized,
        Self::Inner: PartialEq<T>,
    {
        let old = self.bypass_change_detection();
        if old != value {
            value.clone_into(old);
            self.set_changed();
            true
        } else {
            false
        }
    }
}

macro_rules! change_detection_impl {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> DetectChanges for $name<$($generics),*> {
            #[inline]
            fn is_added(&self) -> bool {
                self.ticks
                    .added
                    .is_newer_than(self.ticks.last_run, self.ticks.this_run)
            }

            #[inline]
            fn is_changed(&self) -> bool {
                self.ticks
                    .changed
                    .is_newer_than(self.ticks.last_run, self.ticks.this_run)
            }

            #[inline]
            fn last_changed(&self) -> Tick {
                *self.ticks.changed
            }

            #[inline]
            fn added(&self) -> Tick {
                *self.ticks.added
            }

            #[inline]
            fn changed_by(&self) -> MaybeLocation {
                self.changed_by.copied()
            }
        }

        impl<$($generics),*: ?Sized $(+ $traits)?> Deref for $name<$($generics),*> {
            type Target = $target;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.value
            }
        }

        impl<$($generics),* $(: $traits)?> AsRef<$target> for $name<$($generics),*> {
            #[inline]
            fn as_ref(&self) -> &$target {
                self.deref()
            }
        }
    }
}

macro_rules! change_detection_mut_impl {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> DetectChangesMut for $name<$($generics),*> {
            type Inner = $target;

            #[inline]
            #[track_caller]
            fn set_changed(&mut self) {
                *self.ticks.changed = self.ticks.this_run;
                self.changed_by.assign(MaybeLocation::caller());
            }

            #[inline]
            #[track_caller]
            fn set_added(&mut self) {
                *self.ticks.changed = self.ticks.this_run;
                *self.ticks.added = self.ticks.this_run;
                self.changed_by.assign(MaybeLocation::caller());
            }

            #[inline]
            #[track_caller]
            fn set_last_changed(&mut self, last_changed: Tick) {
                *self.ticks.changed = last_changed;
                self.changed_by.assign(MaybeLocation::caller());
            }

            #[inline]
            #[track_caller]
            fn set_last_added(&mut self, last_added: Tick) {
                *self.ticks.added = last_added;
                *self.ticks.changed = last_added;
                self.changed_by.assign(MaybeLocation::caller());
            }

            #[inline]
            fn bypass_change_detection(&mut self) -> &mut Self::Inner {
                self.value
            }
        }

        impl<$($generics),* : ?Sized $(+ $traits)?> DerefMut for $name<$($generics),*> {
            #[inline]
            #[track_caller]
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.set_changed();
                self.changed_by.assign(MaybeLocation::caller());
                self.value
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
            pub fn into_inner(mut self) -> &'w mut $target {
                self.set_changed();
                self.value
            }

            /// Returns a `Mut<>` with a smaller lifetime.
            /// This is useful if you have `&mut
            #[doc = stringify!($name)]
            /// <T>`, but you need a `Mut<T>`.
            pub fn reborrow(&mut self) -> Mut<'_, $target> {
                Mut {
                    value: self.value,
                    ticks: TicksMut {
                        added: self.ticks.added,
                        changed: self.ticks.changed,
                        last_run: self.ticks.last_run,
                        this_run: self.ticks.this_run,
                    },
                    changed_by: self.changed_by.as_deref_mut(),
                }
            }

            /// Maps to an inner value by applying a function to the contained reference, without flagging a change.
            ///
            /// You should never modify the argument passed to the closure -- if you want to modify the data
            /// without flagging a change, consider using [`DetectChangesMut::bypass_change_detection`] to make your intent explicit.
            ///
            /// ```
            /// # use bevy_ecs::prelude::*;
            /// # #[derive(PartialEq)] pub struct Vec2;
            /// # impl Vec2 { pub const ZERO: Self = Self; }
            /// # #[derive(Component)] pub struct Transform { translation: Vec2 }
            /// // When run, zeroes the translation of every entity.
            /// fn reset_positions(mut transforms: Query<&mut Transform>) {
            ///     for transform in &mut transforms {
            ///         // We pinky promise not to modify `t` within the closure.
            ///         // Breaking this promise will result in logic errors, but will never cause undefined behavior.
            ///         let mut translation = transform.map_unchanged(|t| &mut t.translation);
            ///         // Only reset the translation if it isn't already zero;
            ///         translation.set_if_neq(Vec2::ZERO);
            ///     }
            /// }
            /// # bevy_ecs::system::assert_is_system(reset_positions);
            /// ```
            pub fn map_unchanged<U: ?Sized>(self, f: impl FnOnce(&mut $target) -> &mut U) -> Mut<'w, U> {
                Mut {
                    value: f(self.value),
                    ticks: self.ticks,
                    changed_by: self.changed_by,
                }
            }

            /// Optionally maps to an inner value by applying a function to the contained reference.
            /// This is useful in a situation where you need to convert a `Mut<T>` to a `Mut<U>`, but only if `T` contains `U`.
            ///
            /// As with `map_unchanged`, you should never modify the argument passed to the closure.
            pub fn filter_map_unchanged<U: ?Sized>(self, f: impl FnOnce(&mut $target) -> Option<&mut U>) -> Option<Mut<'w, U>> {
                let value = f(self.value);
                value.map(|value| Mut {
                    value,
                    ticks: self.ticks,
                    changed_by: self.changed_by,
                })
            }

            /// Optionally maps to an inner value by applying a function to the contained reference, returns an error on failure.
            /// This is useful in a situation where you need to convert a `Mut<T>` to a `Mut<U>`, but only if `T` contains `U`.
            ///
            /// As with `map_unchanged`, you should never modify the argument passed to the closure.
            pub fn try_map_unchanged<U: ?Sized, E>(self, f: impl FnOnce(&mut $target) -> Result<&mut U, E>) -> Result<Mut<'w, U>, E> {
                let value = f(self.value);
                value.map(|value| Mut {
                    value,
                    ticks: self.ticks,
                    changed_by: self.changed_by,
                })
            }

            /// Allows you access to the dereferenced value of this pointer without immediately
            /// triggering change detection.
            pub fn as_deref_mut(&mut self) -> Mut<'_, <$target as Deref>::Target>
                where $target: DerefMut
            {
                self.reborrow().map_unchanged(|v| v.deref_mut())
            }

        }
    };
}

macro_rules! impl_debug {
    ($name:ident < $( $generics:tt ),+ >, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> core::fmt::Debug for $name<$($generics),*>
            where T: core::fmt::Debug
        {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.value)
                    .finish()
            }
        }

    };
}

#[derive(Clone)]
pub(crate) struct Ticks<'w> {
    pub(crate) added: &'w Tick,
    pub(crate) changed: &'w Tick,
    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
}

impl<'w> Ticks<'w> {
    /// # Safety
    /// This should never alias the underlying ticks with a mutable one such as `TicksMut`.
    #[inline]
    pub(crate) unsafe fn from_tick_cells(
        cells: TickCells<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            // SAFETY: Caller ensures there is no mutable access to the cell.
            added: unsafe { cells.added.deref() },
            // SAFETY: Caller ensures there is no mutable access to the cell.
            changed: unsafe { cells.changed.deref() },
            last_run,
            this_run,
        }
    }
}

pub(crate) struct TicksMut<'w> {
    pub(crate) added: &'w mut Tick,
    pub(crate) changed: &'w mut Tick,
    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
}

impl<'w> TicksMut<'w> {
    /// # Safety
    /// This should never alias the underlying ticks. All access must be unique.
    #[inline]
    pub(crate) unsafe fn from_tick_cells(
        cells: TickCells<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            // SAFETY: Caller ensures there is no alias to the cell.
            added: unsafe { cells.added.deref_mut() },
            // SAFETY: Caller ensures there is no alias to the cell.
            changed: unsafe { cells.changed.deref_mut() },
            last_run,
            this_run,
        }
    }
}

impl<'w> From<TicksMut<'w>> for Ticks<'w> {
    fn from(ticks: TicksMut<'w>) -> Self {
        Ticks {
            added: ticks.added,
            changed: ticks.changed,
            last_run: ticks.last_run,
            this_run: ticks.this_run,
        }
    }
}

/// Shared borrow of a [`Resource`].
///
/// See the [`Resource`] documentation for usage.
///
/// If you need a unique mutable borrow, use [`ResMut`] instead.
///
/// This [`SystemParam`](crate::system::SystemParam) fails validation if resource doesn't exist.
/// This will cause a panic, but can be configured to do nothing or warn once.
///
/// Use [`Option<Res<T>>`] instead if the resource might not always exist.
pub struct Res<'w, T: ?Sized + Resource> {
    pub(crate) value: &'w T,
    pub(crate) ticks: Ticks<'w>,
    pub(crate) changed_by: MaybeLocation<&'w &'static Location<'static>>,
}

impl<'w, T: Resource> Res<'w, T> {
    /// Copies a reference to a resource.
    ///
    /// Note that unless you actually need an instance of `Res<T>`, you should
    /// prefer to just convert it to `&T` which can be freely copied.
    #[expect(
        clippy::should_implement_trait,
        reason = "As this struct derefs to the inner resource, a `Clone` trait implementation would interfere with the common case of cloning the inner content. (A similar case of this happening can be found with `std::cell::Ref::clone()`.)"
    )]
    pub fn clone(this: &Self) -> Self {
        Self {
            value: this.value,
            ticks: this.ticks.clone(),
            changed_by: this.changed_by,
        }
    }

    /// Due to lifetime limitations of the `Deref` trait, this method can be used to obtain a
    /// reference of the [`Resource`] with a lifetime bound to `'w` instead of the lifetime of the
    /// struct itself.
    pub fn into_inner(self) -> &'w T {
        self.value
    }
}

impl<'w, T: Resource> From<ResMut<'w, T>> for Res<'w, T> {
    fn from(res: ResMut<'w, T>) -> Self {
        Self {
            value: res.value,
            ticks: res.ticks.into(),
            changed_by: res.changed_by.map(|changed_by| &*changed_by),
        }
    }
}

impl<'w, T: Resource> From<Res<'w, T>> for Ref<'w, T> {
    /// Convert a `Res` into a `Ref`. This allows keeping the change-detection feature of `Ref`
    /// while losing the specificity of `Res` for resources.
    fn from(res: Res<'w, T>) -> Self {
        Self {
            value: res.value,
            ticks: res.ticks,
            changed_by: res.changed_by,
        }
    }
}

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
change_detection_impl!(Res<'w, T>, T, Resource);
impl_debug!(Res<'w, T>, Resource);

/// Unique mutable borrow of a [`Resource`].
///
/// See the [`Resource`] documentation for usage.
///
/// If you need a shared borrow, use [`Res`] instead.
///
/// This [`SystemParam`](crate::system::SystemParam) fails validation if resource doesn't exist.
/// This will cause a panic, but can be configured to do nothing or warn once.
///
/// Use [`Option<ResMut<T>>`] instead if the resource might not always exist.
pub struct ResMut<'w, T: ?Sized + Resource> {
    pub(crate) value: &'w mut T,
    pub(crate) ticks: TicksMut<'w>,
    pub(crate) changed_by: MaybeLocation<&'w mut &'static Location<'static>>,
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

change_detection_impl!(ResMut<'w, T>, T, Resource);
change_detection_mut_impl!(ResMut<'w, T>, T, Resource);
impl_methods!(ResMut<'w, T>, T, Resource);
impl_debug!(ResMut<'w, T>, Resource);

impl<'w, T: Resource> From<ResMut<'w, T>> for Mut<'w, T> {
    /// Convert this `ResMut` into a `Mut`. This allows keeping the change-detection feature of `Mut`
    /// while losing the specificity of `ResMut` for resources.
    fn from(other: ResMut<'w, T>) -> Mut<'w, T> {
        Mut {
            value: other.value,
            ticks: other.ticks,
            changed_by: other.changed_by,
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
/// This [`SystemParam`](crate::system::SystemParam) fails validation if non-send resource doesn't exist.
/// This will cause a panic, but can be configured to do nothing or warn once.
///
/// Use [`Option<NonSendMut<T>>`] instead if the resource might not always exist.
pub struct NonSendMut<'w, T: ?Sized + 'static> {
    pub(crate) value: &'w mut T,
    pub(crate) ticks: TicksMut<'w>,
    pub(crate) changed_by: MaybeLocation<&'w mut &'static Location<'static>>,
}

change_detection_impl!(NonSendMut<'w, T>, T,);
change_detection_mut_impl!(NonSendMut<'w, T>, T,);
impl_methods!(NonSendMut<'w, T>, T,);
impl_debug!(NonSendMut<'w, T>,);

impl<'w, T: 'static> From<NonSendMut<'w, T>> for Mut<'w, T> {
    /// Convert this `NonSendMut` into a `Mut`. This allows keeping the change-detection feature of `Mut`
    /// while losing the specificity of `NonSendMut`.
    fn from(other: NonSendMut<'w, T>) -> Mut<'w, T> {
        Mut {
            value: other.value,
            ticks: other.ticks,
            changed_by: other.changed_by,
        }
    }
}

/// Shared borrow of an entity's component with access to change detection.
/// Similar to [`Mut`] but is immutable and so doesn't require unique access.
///
/// # Examples
///
/// These two systems produce the same output.
///
/// ```
/// # use bevy_ecs::change_detection::DetectChanges;
/// # use bevy_ecs::query::{Changed, With};
/// # use bevy_ecs::system::Query;
/// # use bevy_ecs::world::Ref;
/// # use bevy_ecs_macros::Component;
/// # #[derive(Component)]
/// # struct MyComponent;
///
/// fn how_many_changed_1(query: Query<(), Changed<MyComponent>>) {
///     println!("{} changed", query.iter().count());
/// }
///
/// fn how_many_changed_2(query: Query<Ref<MyComponent>>) {
///     println!("{} changed", query.iter().filter(|c| c.is_changed()).count());
/// }
/// ```
pub struct Ref<'w, T: ?Sized> {
    pub(crate) value: &'w T,
    pub(crate) ticks: Ticks<'w>,
    pub(crate) changed_by: MaybeLocation<&'w &'static Location<'static>>,
}

impl<'w, T: ?Sized> Ref<'w, T> {
    /// Returns the reference wrapped by this type. The reference is allowed to outlive `self`, which makes this method more flexible than simply borrowing `self`.
    pub fn into_inner(self) -> &'w T {
        self.value
    }

    /// Map `Ref` to a different type using `f`.
    ///
    /// This doesn't do anything else than call `f` on the wrapped value.
    /// This is equivalent to [`Mut::map_unchanged`].
    pub fn map<U: ?Sized>(self, f: impl FnOnce(&T) -> &U) -> Ref<'w, U> {
        Ref {
            value: f(self.value),
            ticks: self.ticks,
            changed_by: self.changed_by,
        }
    }

    /// Create a new `Ref` using provided values.
    ///
    /// This is an advanced feature, `Ref`s are designed to be _created_ by
    /// engine-internal code and _consumed_ by end-user code.
    ///
    /// - `value` - The value wrapped by `Ref`.
    /// - `added` - A [`Tick`] that stores the tick when the wrapped value was created.
    /// - `changed` - A [`Tick`] that stores the last time the wrapped value was changed.
    /// - `last_run` - A [`Tick`], occurring before `this_run`, which is used
    ///   as a reference to determine whether the wrapped value is newly added or changed.
    /// - `this_run` - A [`Tick`] corresponding to the current point in time -- "now".
    pub fn new(
        value: &'w T,
        added: &'w Tick,
        changed: &'w Tick,
        last_run: Tick,
        this_run: Tick,
        caller: MaybeLocation<&'w &'static Location<'static>>,
    ) -> Ref<'w, T> {
        Ref {
            value,
            ticks: Ticks {
                added,
                changed,
                last_run,
                this_run,
            },
            changed_by: caller,
        }
    }

    /// Overwrite the `last_run` and `this_run` tick that are used for change detection.
    ///
    /// This is an advanced feature. `Ref`s are usually _created_ by engine-internal code and
    /// _consumed_ by end-user code.
    pub fn set_ticks(&mut self, last_run: Tick, this_run: Tick) {
        self.ticks.last_run = last_run;
        self.ticks.this_run = this_run;
    }
}

impl<'w, 'a, T> IntoIterator for &'a Ref<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}
change_detection_impl!(Ref<'w, T>, T,);
impl_debug!(Ref<'w, T>,);

/// Unique mutable borrow of an entity's component or of a resource.
///
/// This can be used in queries to access change detection from immutable query methods, as opposed
/// to `&mut T` which only provides access to change detection from mutable query methods.
///
/// ```rust
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::query::QueryData;
/// #
/// #[derive(Component, Clone, Debug)]
/// struct Name(String);
///
/// #[derive(Component, Clone, Copy, Debug)]
/// struct Health(f32);
///
/// fn my_system(mut query: Query<(Mut<Name>, &mut Health)>) {
///     // Mutable access provides change detection information for both parameters:
///     // - `name` has type `Mut<Name>`
///     // - `health` has type `Mut<Health>`
///     for (name, health) in query.iter_mut() {
///         println!("Name: {:?} (last changed {:?})", name, name.last_changed());
///         println!("Health: {:?} (last changed: {:?})", health, health.last_changed());
/// #        println!("{}{}", name.0, health.0); // Silence dead_code warning
///     }
///
///     // Immutable access only provides change detection for `Name`:
///     // - `name` has type `Ref<Name>`
///     // - `health` has type `&Health`
///     for (name, health) in query.iter() {
///         println!("Name: {:?} (last changed {:?})", name, name.last_changed());
///         println!("Health: {:?}", health);
///     }
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
pub struct Mut<'w, T: ?Sized> {
    pub(crate) value: &'w mut T,
    pub(crate) ticks: TicksMut<'w>,
    pub(crate) changed_by: MaybeLocation<&'w mut &'static Location<'static>>,
}

impl<'w, T: ?Sized> Mut<'w, T> {
    /// Creates a new change-detection enabled smart pointer.
    /// In almost all cases you do not need to call this method manually,
    /// as instances of `Mut` will be created by engine-internal code.
    ///
    /// Many use-cases of this method would be better served by [`Mut::map_unchanged`]
    /// or [`Mut::reborrow`].
    ///
    /// - `value` - The value wrapped by this smart pointer.
    /// - `added` - A [`Tick`] that stores the tick when the wrapped value was created.
    /// - `last_changed` - A [`Tick`] that stores the last time the wrapped value was changed.
    ///   This will be updated to the value of `change_tick` if the returned smart pointer
    ///   is modified.
    /// - `last_run` - A [`Tick`], occurring before `this_run`, which is used
    ///   as a reference to determine whether the wrapped value is newly added or changed.
    /// - `this_run` - A [`Tick`] corresponding to the current point in time -- "now".
    pub fn new(
        value: &'w mut T,
        added: &'w mut Tick,
        last_changed: &'w mut Tick,
        last_run: Tick,
        this_run: Tick,
        caller: MaybeLocation<&'w mut &'static Location<'static>>,
    ) -> Self {
        Self {
            value,
            ticks: TicksMut {
                added,
                changed: last_changed,
                last_run,
                this_run,
            },
            changed_by: caller,
        }
    }

    /// Overwrite the `last_run` and `this_run` tick that are used for change detection.
    ///
    /// This is an advanced feature. `Mut`s are usually _created_ by engine-internal code and
    /// _consumed_ by end-user code.
    pub fn set_ticks(&mut self, last_run: Tick, this_run: Tick) {
        self.ticks.last_run = last_run;
        self.ticks.this_run = this_run;
    }
}

impl<'w, T: ?Sized> From<Mut<'w, T>> for Ref<'w, T> {
    fn from(mut_ref: Mut<'w, T>) -> Self {
        Self {
            value: mut_ref.value,
            ticks: mut_ref.ticks.into(),
            changed_by: mut_ref.changed_by.map(|changed_by| &*changed_by),
        }
    }
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

change_detection_impl!(Mut<'w, T>, T,);
change_detection_mut_impl!(Mut<'w, T>, T,);
impl_methods!(Mut<'w, T>, T,);
impl_debug!(Mut<'w, T>,);

/// Unique mutable borrow of resources or an entity's component.
///
/// Similar to [`Mut`], but not generic over the component type, instead
/// exposing the raw pointer as a `*mut ()`.
///
/// Usually you don't need to use this and can instead use the APIs returning a
/// [`Mut`], but in situations where the types are not known at compile time
/// or are defined outside of rust this can be used.
pub struct MutUntyped<'w> {
    pub(crate) value: PtrMut<'w>,
    pub(crate) ticks: TicksMut<'w>,
    pub(crate) changed_by: MaybeLocation<&'w mut &'static Location<'static>>,
}

impl<'w> MutUntyped<'w> {
    /// Returns the pointer to the value, marking it as changed.
    ///
    /// In order to avoid marking the value as changed, you need to call [`bypass_change_detection`](DetectChangesMut::bypass_change_detection).
    #[inline]
    pub fn into_inner(mut self) -> PtrMut<'w> {
        self.set_changed();
        self.value
    }

    /// Returns a [`MutUntyped`] with a smaller lifetime.
    /// This is useful if you have `&mut MutUntyped`, but you need a `MutUntyped`.
    #[inline]
    pub fn reborrow(&mut self) -> MutUntyped {
        MutUntyped {
            value: self.value.reborrow(),
            ticks: TicksMut {
                added: self.ticks.added,
                changed: self.ticks.changed,
                last_run: self.ticks.last_run,
                this_run: self.ticks.this_run,
            },
            changed_by: self.changed_by.as_deref_mut(),
        }
    }

    /// Returns `true` if this value was changed or mutably dereferenced
    /// either since a specific change tick.
    pub fn has_changed_since(&self, tick: Tick) -> bool {
        self.ticks.changed.is_newer_than(tick, self.ticks.this_run)
    }

    /// Returns a pointer to the value without taking ownership of this smart pointer, marking it as changed.
    ///
    /// In order to avoid marking the value as changed, you need to call [`bypass_change_detection`](DetectChangesMut::bypass_change_detection).
    #[inline]
    pub fn as_mut(&mut self) -> PtrMut<'_> {
        self.set_changed();
        self.value.reborrow()
    }

    /// Returns an immutable pointer to the value without taking ownership.
    #[inline]
    pub fn as_ref(&self) -> Ptr<'_> {
        self.value.as_ref()
    }

    /// Turn this [`MutUntyped`] into a [`Mut`] by mapping the inner [`PtrMut`] to another value,
    /// without flagging a change.
    /// This function is the untyped equivalent of [`Mut::map_unchanged`].
    ///
    /// You should never modify the argument passed to the closure – if you want to modify the data without flagging a change, consider using [`bypass_change_detection`](DetectChangesMut::bypass_change_detection) to make your intent explicit.
    ///
    /// If you know the type of the value you can do
    /// ```no_run
    /// # use bevy_ecs::change_detection::{Mut, MutUntyped};
    /// # let mut_untyped: MutUntyped = unimplemented!();
    /// // SAFETY: ptr is of type `u8`
    /// mut_untyped.map_unchanged(|ptr| unsafe { ptr.deref_mut::<u8>() });
    /// ```
    /// If you have a [`ReflectFromPtr`](bevy_reflect::ReflectFromPtr) that you know belongs to this [`MutUntyped`],
    /// you can do
    /// ```no_run
    /// # use bevy_ecs::change_detection::{Mut, MutUntyped};
    /// # let mut_untyped: MutUntyped = unimplemented!();
    /// # let reflect_from_ptr: bevy_reflect::ReflectFromPtr = unimplemented!();
    /// // SAFETY: from the context it is known that `ReflectFromPtr` was made for the type of the `MutUntyped`
    /// mut_untyped.map_unchanged(|ptr| unsafe { reflect_from_ptr.as_reflect_mut(ptr) });
    /// ```
    pub fn map_unchanged<T: ?Sized>(self, f: impl FnOnce(PtrMut<'w>) -> &'w mut T) -> Mut<'w, T> {
        Mut {
            value: f(self.value),
            ticks: self.ticks,
            changed_by: self.changed_by,
        }
    }

    /// Transforms this [`MutUntyped`] into a [`Mut<T>`] with the same lifetime.
    ///
    /// # Safety
    /// - `T` must be the erased pointee type for this [`MutUntyped`].
    pub unsafe fn with_type<T>(self) -> Mut<'w, T> {
        Mut {
            // SAFETY: `value` is `Aligned` and caller ensures the pointee type is `T`.
            value: unsafe { self.value.deref_mut() },
            ticks: self.ticks,
            // SAFETY: `caller` is `Aligned`.
            changed_by: self.changed_by,
        }
    }
}

impl<'w> DetectChanges for MutUntyped<'w> {
    #[inline]
    fn is_added(&self) -> bool {
        self.ticks
            .added
            .is_newer_than(self.ticks.last_run, self.ticks.this_run)
    }

    #[inline]
    fn is_changed(&self) -> bool {
        self.ticks
            .changed
            .is_newer_than(self.ticks.last_run, self.ticks.this_run)
    }

    #[inline]
    fn last_changed(&self) -> Tick {
        *self.ticks.changed
    }

    #[inline]
    fn changed_by(&self) -> MaybeLocation {
        self.changed_by.copied()
    }

    #[inline]
    fn added(&self) -> Tick {
        *self.ticks.added
    }
}

impl<'w> DetectChangesMut for MutUntyped<'w> {
    type Inner = PtrMut<'w>;

    #[inline]
    #[track_caller]
    fn set_changed(&mut self) {
        *self.ticks.changed = self.ticks.this_run;
        self.changed_by.assign(MaybeLocation::caller());
    }

    #[inline]
    #[track_caller]
    fn set_added(&mut self) {
        *self.ticks.changed = self.ticks.this_run;
        *self.ticks.added = self.ticks.this_run;
        self.changed_by.assign(MaybeLocation::caller());
    }

    #[inline]
    #[track_caller]
    fn set_last_changed(&mut self, last_changed: Tick) {
        *self.ticks.changed = last_changed;
        self.changed_by.assign(MaybeLocation::caller());
    }

    #[inline]
    #[track_caller]
    fn set_last_added(&mut self, last_added: Tick) {
        *self.ticks.added = last_added;
        *self.ticks.changed = last_added;
        self.changed_by.assign(MaybeLocation::caller());
    }

    #[inline]
    #[track_caller]
    fn bypass_change_detection(&mut self) -> &mut Self::Inner {
        &mut self.value
    }
}

impl core::fmt::Debug for MutUntyped<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("MutUntyped")
            .field(&self.value.as_ptr())
            .finish()
    }
}

impl<'w, T> From<Mut<'w, T>> for MutUntyped<'w> {
    fn from(value: Mut<'w, T>) -> Self {
        MutUntyped {
            value: value.value.into(),
            ticks: value.ticks,
            changed_by: value.changed_by,
        }
    }
}

/// A value that contains a `T` if the `track_location` feature is enabled,
/// and is a ZST if it is not.
///
/// The overall API is similar to [`Option`], but whether the value is `Some` or `None` is set at compile
/// time and is the same for all values.
///
/// If the `track_location` feature is disabled, then all functions on this type that return
/// an `MaybeLocation` will have an empty body and should be removed by the optimizer.
///
/// This allows code to be written that will be checked by the compiler even when the feature is disabled,
/// but that will be entirely removed during compilation.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MaybeLocation<T: ?Sized = &'static Location<'static>> {
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore, clone))]
    marker: PhantomData<T>,
    #[cfg(feature = "track_location")]
    value: T,
}

impl<T: core::fmt::Display> core::fmt::Display for MaybeLocation<T> {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(feature = "track_location")]
        {
            self.value.fmt(_f)?;
        }
        Ok(())
    }
}

impl<T> MaybeLocation<T> {
    /// Constructs a new `MaybeLocation` that wraps the given value.
    ///
    /// This may only accept `Copy` types,
    /// since it needs to drop the value if the `track_location` feature is disabled,
    /// and non-`Copy` types cannot be dropped in `const` context.
    /// Use [`new_with`][Self::new_with] if you need to construct a non-`Copy` value.
    ///
    /// # See also
    /// - [`new_with`][Self::new_with] to initialize using a closure.
    /// - [`new_with_flattened`][Self::new_with_flattened] to initialize using a closure that returns an `Option<MaybeLocation<T>>`.
    #[inline]
    pub const fn new(_value: T) -> Self
    where
        T: Copy,
    {
        Self {
            #[cfg(feature = "track_location")]
            value: _value,
            marker: PhantomData,
        }
    }

    /// Constructs a new `MaybeLocation` that wraps the result of the given closure.
    ///
    /// # See also
    /// - [`new`][Self::new] to initialize using a value.
    /// - [`new_with_flattened`][Self::new_with_flattened] to initialize using a closure that returns an `Option<MaybeLocation<T>>`.
    #[inline]
    pub fn new_with(_f: impl FnOnce() -> T) -> Self {
        Self {
            #[cfg(feature = "track_location")]
            value: _f(),
            marker: PhantomData,
        }
    }

    /// Maps an `MaybeLocation<T> `to `MaybeLocation<U>` by applying a function to a contained value.
    #[inline]
    pub fn map<U>(self, _f: impl FnOnce(T) -> U) -> MaybeLocation<U> {
        MaybeLocation {
            #[cfg(feature = "track_location")]
            value: _f(self.value),
            marker: PhantomData,
        }
    }

    /// Converts a pair of `MaybeLocation` values to an `MaybeLocation` of a tuple.
    #[inline]
    pub fn zip<U>(self, _other: MaybeLocation<U>) -> MaybeLocation<(T, U)> {
        MaybeLocation {
            #[cfg(feature = "track_location")]
            value: (self.value, _other.value),
            marker: PhantomData,
        }
    }

    /// Returns the contained value or a default.
    /// If the `track_location` feature is enabled, this always returns the contained value.
    /// If it is disabled, this always returns `T::Default()`.
    #[inline]
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        self.into_option().unwrap_or_default()
    }

    /// Converts an `MaybeLocation` to an [`Option`] to allow run-time branching.
    /// If the `track_location` feature is enabled, this always returns `Some`.
    /// If it is disabled, this always returns `None`.
    #[inline]
    pub fn into_option(self) -> Option<T> {
        #[cfg(feature = "track_location")]
        {
            Some(self.value)
        }
        #[cfg(not(feature = "track_location"))]
        {
            None
        }
    }
}

impl<T> MaybeLocation<Option<T>> {
    /// Constructs a new `MaybeLocation` that wraps the result of the given closure.
    /// If the closure returns `Some`, it unwraps the inner value.
    ///
    /// # See also
    /// - [`new`][Self::new] to initialize using a value.
    /// - [`new_with`][Self::new_with] to initialize using a closure.
    #[inline]
    pub fn new_with_flattened(_f: impl FnOnce() -> Option<MaybeLocation<T>>) -> Self {
        Self {
            #[cfg(feature = "track_location")]
            value: _f().map(|value| value.value),
            marker: PhantomData,
        }
    }

    /// Transposes a `MaybeLocation` of an [`Option`] into an [`Option`] of a `MaybeLocation`.
    ///
    /// This can be useful if you want to use the `?` operator to exit early
    /// if the `track_location` feature is enabled but the value is not found.
    ///
    /// If the `track_location` feature is enabled,
    /// this returns `Some` if the inner value is `Some`
    /// and `None` if the inner value is `None`.
    ///
    /// If it is disabled, this always returns `Some`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::{change_detection::MaybeLocation, world::World};
    /// # use core::panic::Location;
    /// #
    /// # fn test() -> Option<()> {
    /// let mut world = World::new();
    /// let entity = world.spawn(()).id();
    /// let location: MaybeLocation<Option<&'static Location<'static>>> =
    ///     world.entities().entity_get_spawned_or_despawned_by(entity);
    /// let location: MaybeLocation<&'static Location<'static>> = location.transpose()?;
    /// # Some(())
    /// # }
    /// # test();
    /// ```
    ///
    /// # See also
    ///
    /// - [`into_option`][Self::into_option] to convert to an `Option<Option<T>>`.
    ///   When used with [`Option::flatten`], this will have a similar effect,
    ///   but will return `None` when the `track_location` feature is disabled.
    #[inline]
    pub fn transpose(self) -> Option<MaybeLocation<T>> {
        #[cfg(feature = "track_location")]
        {
            self.value.map(|value| MaybeLocation {
                value,
                marker: PhantomData,
            })
        }
        #[cfg(not(feature = "track_location"))]
        {
            Some(MaybeLocation {
                marker: PhantomData,
            })
        }
    }
}

impl<T> MaybeLocation<&T> {
    /// Maps an `MaybeLocation<&T>` to an `MaybeLocation<T>` by copying the contents.
    #[inline]
    pub const fn copied(&self) -> MaybeLocation<T>
    where
        T: Copy,
    {
        MaybeLocation {
            #[cfg(feature = "track_location")]
            value: *self.value,
            marker: PhantomData,
        }
    }
}

impl<T> MaybeLocation<&mut T> {
    /// Maps an `MaybeLocation<&mut T>` to an `MaybeLocation<T>` by copying the contents.
    #[inline]
    pub const fn copied(&self) -> MaybeLocation<T>
    where
        T: Copy,
    {
        MaybeLocation {
            #[cfg(feature = "track_location")]
            value: *self.value,
            marker: PhantomData,
        }
    }

    /// Assigns the contents of an `MaybeLocation<T>` to an `MaybeLocation<&mut T>`.
    #[inline]
    pub fn assign(&mut self, _value: MaybeLocation<T>) {
        #[cfg(feature = "track_location")]
        {
            *self.value = _value.value;
        }
    }
}

impl<T: ?Sized> MaybeLocation<T> {
    /// Converts from `&MaybeLocation<T>` to `MaybeLocation<&T>`.
    #[inline]
    pub const fn as_ref(&self) -> MaybeLocation<&T> {
        MaybeLocation {
            #[cfg(feature = "track_location")]
            value: &self.value,
            marker: PhantomData,
        }
    }

    /// Converts from `&mut MaybeLocation<T>` to `MaybeLocation<&mut T>`.
    #[inline]
    pub const fn as_mut(&mut self) -> MaybeLocation<&mut T> {
        MaybeLocation {
            #[cfg(feature = "track_location")]
            value: &mut self.value,
            marker: PhantomData,
        }
    }

    /// Converts from `&MaybeLocation<T>` to `MaybeLocation<&T::Target>`.
    #[inline]
    pub fn as_deref(&self) -> MaybeLocation<&T::Target>
    where
        T: Deref,
    {
        MaybeLocation {
            #[cfg(feature = "track_location")]
            value: &*self.value,
            marker: PhantomData,
        }
    }

    /// Converts from `&mut MaybeLocation<T>` to `MaybeLocation<&mut T::Target>`.
    #[inline]
    pub fn as_deref_mut(&mut self) -> MaybeLocation<&mut T::Target>
    where
        T: DerefMut,
    {
        MaybeLocation {
            #[cfg(feature = "track_location")]
            value: &mut *self.value,
            marker: PhantomData,
        }
    }
}

impl MaybeLocation {
    /// Returns the source location of the caller of this function. If that function's caller is
    /// annotated then its call location will be returned, and so on up the stack to the first call
    /// within a non-tracked function body.
    #[inline]
    #[track_caller]
    pub const fn caller() -> Self {
        // Note that this cannot use `new_with`, since `FnOnce` invocations cannot be annotated with `#[track_caller]`.
        MaybeLocation {
            #[cfg(feature = "track_location")]
            value: Location::caller(),
            marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs_macros::Resource;
    use bevy_ptr::PtrMut;
    use bevy_reflect::{FromType, ReflectFromPtr};
    use core::ops::{Deref, DerefMut};

    use crate::{
        change_detection::{
            MaybeLocation, Mut, NonSendMut, Ref, ResMut, TicksMut, CHECK_TICK_THRESHOLD,
            MAX_CHANGE_AGE,
        },
        component::{Component, ComponentTicks, Tick},
        system::{IntoSystem, Single, System},
        world::World,
    };

    use super::{DetectChanges, DetectChangesMut, MutUntyped};

    #[derive(Component, PartialEq)]
    struct C;

    #[derive(Resource)]
    struct R;

    #[derive(Resource, PartialEq)]
    struct R2(u8);

    impl Deref for R2 {
        type Target = u8;
        fn deref(&self) -> &u8 {
            &self.0
        }
    }

    impl DerefMut for R2 {
        fn deref_mut(&mut self) -> &mut u8 {
            &mut self.0
        }
    }

    #[test]
    fn change_expiration() {
        fn change_detected(query: Option<Single<Ref<C>>>) -> bool {
            query.unwrap().is_changed()
        }

        fn change_expired(query: Option<Single<Ref<C>>>) -> bool {
            query.unwrap().is_changed()
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
        assert!(change_detected_system.run((), &mut world).unwrap());

        // world: 1 + MAX_CHANGE_AGE
        let change_tick = world.change_tick.get_mut();
        *change_tick = change_tick.wrapping_add(MAX_CHANGE_AGE);

        // Both the system and component appeared `MAX_CHANGE_AGE` ticks ago.
        // Since we clamp things to `MAX_CHANGE_AGE` for determinism,
        // `ComponentTicks::is_changed` will now see `MAX_CHANGE_AGE > MAX_CHANGE_AGE`
        // and return `false`.
        assert!(!change_expired_system.run((), &mut world).unwrap());
    }

    #[test]
    fn change_tick_wraparound() {
        let mut world = World::new();
        world.last_change_tick = Tick::new(u32::MAX);
        *world.change_tick.get_mut() = 0;

        // component added: 0, changed: 0
        world.spawn(C);

        world.increment_change_tick();

        // Since the world is always ahead, as long as changes can't get older than `u32::MAX` (which we ensure),
        // the wrapping difference will always be positive, so wraparound doesn't matter.
        let mut query = world.query::<Ref<C>>();
        assert!(query.single(&world).unwrap().is_changed());
    }

    #[test]
    fn change_tick_scan() {
        let mut world = World::new();

        // component added: 1, changed: 1
        world.spawn(C);

        // a bunch of stuff happens, the component is now older than `MAX_CHANGE_AGE`
        *world.change_tick.get_mut() += MAX_CHANGE_AGE + CHECK_TICK_THRESHOLD;
        let change_tick = world.change_tick();

        let mut query = world.query::<Ref<C>>();
        for tracker in query.iter(&world) {
            let ticks_since_insert = change_tick.relative_to(*tracker.ticks.added).get();
            let ticks_since_change = change_tick.relative_to(*tracker.ticks.changed).get();
            assert!(ticks_since_insert > MAX_CHANGE_AGE);
            assert!(ticks_since_change > MAX_CHANGE_AGE);
        }

        // scan change ticks and clamp those at risk of overflow
        world.check_change_ticks();

        for tracker in query.iter(&world) {
            let ticks_since_insert = change_tick.relative_to(*tracker.ticks.added).get();
            let ticks_since_change = change_tick.relative_to(*tracker.ticks.changed).get();
            assert_eq!(ticks_since_insert, MAX_CHANGE_AGE);
            assert_eq!(ticks_since_change, MAX_CHANGE_AGE);
        }
    }

    #[test]
    fn mut_from_res_mut() {
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
        let mut caller = MaybeLocation::caller();

        let res_mut = ResMut {
            value: &mut res,
            ticks,
            changed_by: caller.as_mut(),
        };

        let into_mut: Mut<R> = res_mut.into();
        assert_eq!(1, into_mut.ticks.added.get());
        assert_eq!(2, into_mut.ticks.changed.get());
        assert_eq!(3, into_mut.ticks.last_run.get());
        assert_eq!(4, into_mut.ticks.this_run.get());
    }

    #[test]
    fn mut_new() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(3),
        };
        let mut res = R {};
        let mut caller = MaybeLocation::caller();

        let val = Mut::new(
            &mut res,
            &mut component_ticks.added,
            &mut component_ticks.changed,
            Tick::new(2), // last_run
            Tick::new(4), // this_run
            caller.as_mut(),
        );

        assert!(!val.is_added());
        assert!(val.is_changed());
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
        let mut caller = MaybeLocation::caller();

        let non_send_mut = NonSendMut {
            value: &mut res,
            ticks,
            changed_by: caller.as_mut(),
        };

        let into_mut: Mut<R> = non_send_mut.into();
        assert_eq!(1, into_mut.ticks.added.get());
        assert_eq!(2, into_mut.ticks.changed.get());
        assert_eq!(3, into_mut.ticks.last_run.get());
        assert_eq!(4, into_mut.ticks.this_run.get());
    }

    #[test]
    fn map_mut() {
        use super::*;
        struct Outer(i64);

        let last_run = Tick::new(2);
        let this_run = Tick::new(3);
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let ticks = TicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            last_run,
            this_run,
        };

        let mut outer = Outer(0);
        let mut caller = MaybeLocation::caller();

        let ptr = Mut {
            value: &mut outer,
            ticks,
            changed_by: caller.as_mut(),
        };
        assert!(!ptr.is_changed());

        // Perform a mapping operation.
        let mut inner = ptr.map_unchanged(|x| &mut x.0);
        assert!(!inner.is_changed());

        // Mutate the inner value.
        *inner = 64;
        assert!(inner.is_changed());
        // Modifying one field of a component should flag a change for the entire component.
        assert!(component_ticks.is_changed(last_run, this_run));
    }

    #[test]
    fn set_if_neq() {
        let mut world = World::new();

        world.insert_resource(R2(0));
        // Resources are Changed when first added
        world.increment_change_tick();
        // This is required to update world::last_change_tick
        world.clear_trackers();

        let mut r = world.resource_mut::<R2>();
        assert!(!r.is_changed(), "Resource must begin unchanged.");

        r.set_if_neq(R2(0));
        assert!(
            !r.is_changed(),
            "Resource must not be changed after setting to the same value."
        );

        r.set_if_neq(R2(3));
        assert!(
            r.is_changed(),
            "Resource must be changed after setting to a different value."
        );
    }

    #[test]
    fn as_deref_mut() {
        let mut world = World::new();

        world.insert_resource(R2(0));
        // Resources are Changed when first added
        world.increment_change_tick();
        // This is required to update world::last_change_tick
        world.clear_trackers();

        let mut r = world.resource_mut::<R2>();
        assert!(!r.is_changed(), "Resource must begin unchanged.");

        let mut r = r.as_deref_mut();
        assert!(
            !r.is_changed(),
            "Dereferencing should not mark the item as changed yet"
        );

        r.set_if_neq(3);
        assert!(
            r.is_changed(),
            "Resource must be changed after setting to a different value."
        );
    }

    #[test]
    fn mut_untyped_to_reflect() {
        let last_run = Tick::new(2);
        let this_run = Tick::new(3);
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let ticks = TicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            last_run,
            this_run,
        };

        let mut value: i32 = 5;
        let mut caller = MaybeLocation::caller();

        let value = MutUntyped {
            value: PtrMut::from(&mut value),
            ticks,
            changed_by: caller.as_mut(),
        };

        let reflect_from_ptr = <ReflectFromPtr as FromType<i32>>::from_type();

        let mut new = value.map_unchanged(|ptr| {
            // SAFETY: The underlying type of `ptr` matches `reflect_from_ptr`.
            unsafe { reflect_from_ptr.as_reflect_mut(ptr) }
        });

        assert!(!new.is_changed());

        new.reflect_mut();

        assert!(new.is_changed());
    }

    #[test]
    fn mut_untyped_from_mut() {
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
        let mut c = C {};
        let mut caller = MaybeLocation::caller();

        let mut_typed = Mut {
            value: &mut c,
            ticks,
            changed_by: caller.as_mut(),
        };

        let into_mut: MutUntyped = mut_typed.into();
        assert_eq!(1, into_mut.ticks.added.get());
        assert_eq!(2, into_mut.ticks.changed.get());
        assert_eq!(3, into_mut.ticks.last_run.get());
        assert_eq!(4, into_mut.ticks.this_run.get());
    }
}
