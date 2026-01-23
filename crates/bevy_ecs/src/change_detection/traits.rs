use crate::{change_detection::MaybeLocation, change_detection::Tick};
use alloc::borrow::ToOwned;
use core::mem;

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
///
/// [`Res`]: crate::change_detection::params::Res
/// [`ResMut`]: crate::change_detection::params::ResMut
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
///
/// [`ResMut`]: crate::change_detection::params::ResMut
/// [`DerefMut`]: core::ops::DerefMut
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

    // NOTE: if you are changing the following comment also change the [`ContiguousMut::bypass_change_detection`] comment.
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
    ///
    /// [`Mut::map_unchanged`]: crate::change_detection::params::Mut::map_unchanged
    /// [`MutUntyped::map_unchanged`]: crate::change_detection::params::MutUntyped::map_unchanged
    /// [`ResMut::map_unchanged`]: crate::change_detection::params::ResMut::map_unchanged
    /// [`NonSendMut::map_unchanged`]: crate::change_detection::params::NonSendMut::map_unchanged
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
    /// then consider applying a `map_unchanged` beforehand to allow
    /// changing only the relevant field and prevent unnecessary copying and cloning.
    /// See the docs of [`Mut::map_unchanged`], [`MutUntyped::map_unchanged`],
    /// [`ResMut::map_unchanged`] or [`NonSendMut::map_unchanged`] for an example
    ///
    /// If you don't need the previous value, use [`set_if_neq`](DetectChangesMut::set_if_neq).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, schedule::common_conditions::{resource_changed, on_message}};
    /// #[derive(Resource, PartialEq, Eq)]
    /// pub struct Score(u32);
    ///
    /// #[derive(Message, PartialEq, Eq)]
    /// pub struct ScoreChanged {
    ///     current: u32,
    ///     previous: u32,
    /// }
    ///
    /// fn reset_score(mut score: ResMut<Score>, mut score_changed: MessageWriter<ScoreChanged>) {
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
    /// # world.insert_resource(Messages::<ScoreChanged>::default());
    /// # world.insert_resource(Score(1));
    /// # let mut score_changed = IntoSystem::into_system(resource_changed::<Score>);
    /// # score_changed.initialize(&mut world);
    /// # score_changed.run((), &mut world);
    /// #
    /// # let mut score_changed_event = IntoSystem::into_system(on_message::<ScoreChanged>);
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
    ///
    /// [`Mut::map_unchanged`]: crate::change_detection::params::Mut::map_unchanged
    /// [`MutUntyped::map_unchanged`]: crate::change_detection::params::MutUntyped::map_unchanged
    /// [`ResMut::map_unchanged`]: crate::change_detection::params::ResMut::map_unchanged
    /// [`NonSendMut::map_unchanged`]: crate::change_detection::params::NonSendMut::map_unchanged
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
                self.ticks.changed_by.copied()
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

pub(crate) use change_detection_impl;

macro_rules! change_detection_mut_impl {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> DetectChangesMut for $name<$($generics),*> {
            type Inner = $target;

            #[inline]
            #[track_caller]
            fn set_changed(&mut self) {
                *self.ticks.changed = self.ticks.this_run;
                self.ticks.changed_by.assign(MaybeLocation::caller());
            }

            #[inline]
            #[track_caller]
            fn set_added(&mut self) {
                *self.ticks.changed = self.ticks.this_run;
                *self.ticks.added = self.ticks.this_run;
                self.ticks.changed_by.assign(MaybeLocation::caller());
            }

            #[inline]
            #[track_caller]
            fn set_last_changed(&mut self, last_changed: Tick) {
                *self.ticks.changed = last_changed;
                self.ticks.changed_by.assign(MaybeLocation::caller());
            }

            #[inline]
            #[track_caller]
            fn set_last_added(&mut self, last_added: Tick) {
                *self.ticks.added = last_added;
                *self.ticks.changed = last_added;
                self.ticks.changed_by.assign(MaybeLocation::caller());
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
                self.ticks.changed_by.assign(MaybeLocation::caller());
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

pub(crate) use change_detection_mut_impl;

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
                    ticks: ComponentTicksMut {
                        added: self.ticks.added,
                        changed: self.ticks.changed,
                        changed_by: self.ticks.changed_by.as_deref_mut(),
                        last_run: self.ticks.last_run,
                        this_run: self.ticks.this_run,
                    },
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

pub(crate) use impl_methods;

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

pub(crate) use impl_debug;
