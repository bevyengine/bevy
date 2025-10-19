use crate::{
    change_detection::{traits::*, MaybeLocation},
    component::{ComponentTickCells, Tick},
    ptr::PtrMut,
    resource::Resource,
};
use bevy_ptr::{Ptr, UnsafeCellDeref};
use core::{
    ops::{Deref, DerefMut},
    panic::Location,
};

/// Used by immutable query parameters (such as [`Ref`] and [`Res`])
/// to store immutable access to the [`Tick`]s of a single component or resource.
#[derive(Clone)]
pub(crate) struct ComponentTicksRef<'w> {
    pub(crate) added: &'w Tick,
    pub(crate) changed: &'w Tick,
    pub(crate) changed_by: MaybeLocation<&'w &'static Location<'static>>,
    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
}

impl<'w> ComponentTicksRef<'w> {
    /// # Safety
    /// This should never alias the underlying ticks with a mutable one such as `ComponentTicksMut`.
    #[inline]
    pub(crate) unsafe fn from_tick_cells(
        cells: ComponentTickCells<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            // SAFETY: Caller ensures there is no mutable access to the cell.
            added: unsafe { cells.added.deref() },
            // SAFETY: Caller ensures there is no mutable access to the cell.
            changed: unsafe { cells.changed.deref() },
            // SAFETY: Caller ensures there is no mutable access to the cell.
            changed_by: unsafe { cells.changed_by.map(|changed_by| changed_by.deref()) },
            last_run,
            this_run,
        }
    }
}

/// Used by mutable query parameters (such as [`Mut`] and [`ResMut`])
/// to store mutable access to the [`Tick`]s of a single component or resource.
pub(crate) struct ComponentTicksMut<'w> {
    pub(crate) added: &'w mut Tick,
    pub(crate) changed: &'w mut Tick,
    pub(crate) changed_by: MaybeLocation<&'w mut &'static Location<'static>>,
    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
}

impl<'w> ComponentTicksMut<'w> {
    /// # Safety
    /// This should never alias the underlying ticks. All access must be unique.
    #[inline]
    pub(crate) unsafe fn from_tick_cells(
        cells: ComponentTickCells<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            // SAFETY: Caller ensures there is no alias to the cell.
            added: unsafe { cells.added.deref_mut() },
            // SAFETY: Caller ensures there is no alias to the cell.
            changed: unsafe { cells.changed.deref_mut() },
            // SAFETY: Caller ensures there is no alias to the cell.
            changed_by: unsafe { cells.changed_by.map(|changed_by| changed_by.deref_mut()) },
            last_run,
            this_run,
        }
    }
}

impl<'w> From<ComponentTicksMut<'w>> for ComponentTicksRef<'w> {
    fn from(ticks: ComponentTicksMut<'w>) -> Self {
        ComponentTicksRef {
            added: ticks.added,
            changed: ticks.changed,
            changed_by: ticks.changed_by.map(|changed_by| &*changed_by),
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
    pub(crate) ticks: ComponentTicksRef<'w>,
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
    pub(crate) ticks: ComponentTicksMut<'w>,
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
        }
    }
}

/// Shared borrow of a non-[`Send`] resource.
///
/// Only [`Send`] resources may be accessed with the [`Res`] [`SystemParam`](crate::system::SystemParam). In case that the
/// resource does not implement `Send`, this `SystemParam` wrapper can be used. This will instruct
/// the scheduler to instead run the system on the main thread so that it doesn't send the resource
/// over to another thread.
///
/// This [`SystemParam`](crate::system::SystemParam) fails validation if the non-send resource doesn't exist.
/// This will cause a panic, but can be configured to do nothing or warn once.
///
/// Use [`Option<NonSend<T>>`] instead if the resource might not always exist.
pub struct NonSend<'w, T: ?Sized + 'static> {
    pub(crate) value: &'w T,
    pub(crate) ticks: ComponentTicksRef<'w>,
}

change_detection_impl!(NonSend<'w, T>, T,);
impl_debug!(NonSend<'w, T>,);

impl<'w, T> From<NonSendMut<'w, T>> for NonSend<'w, T> {
    fn from(other: NonSendMut<'w, T>) -> Self {
        Self {
            value: other.value,
            ticks: other.ticks.into(),
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
    pub(crate) ticks: ComponentTicksMut<'w>,
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
    pub(crate) ticks: ComponentTicksRef<'w>,
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
            ticks: ComponentTicksRef {
                added,
                changed,
                changed_by: caller,
                last_run,
                this_run,
            },
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
    pub(crate) ticks: ComponentTicksMut<'w>,
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
            ticks: ComponentTicksMut {
                added,
                changed: last_changed,
                changed_by: caller,
                last_run,
                this_run,
            },
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
    pub(crate) ticks: ComponentTicksMut<'w>,
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
    pub fn reborrow(&mut self) -> MutUntyped<'_> {
        MutUntyped {
            value: self.value.reborrow(),
            ticks: ComponentTicksMut {
                added: self.ticks.added,
                changed: self.ticks.changed,
                changed_by: self.ticks.changed_by.as_deref_mut(),
                last_run: self.ticks.last_run,
                this_run: self.ticks.this_run,
            },
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
        self.ticks.changed_by.copied()
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
        }
    }
}
