use crate::{
    change_detection::{traits::*, ComponentTickCells, MaybeLocation, Tick},
    ptr::PtrMut,
    resource::Resource,
};
use bevy_ptr::{Ptr, ThinSlicePtr, UnsafeCellDeref};
use core::{
    cell::UnsafeCell,
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

/// Data type storing contiguously lying ticks.
///
/// Retrievable via [`ContiguousRef::split`] and probably only useful if you want to use the following
/// methods:
/// - [`ContiguousComponentTicksRef::is_changed_iter`],
/// - [`ContiguousComponentTicksRef::is_added_iter`]
#[derive(Clone)]
pub struct ContiguousComponentTicksRef<'w> {
    pub(crate) added: &'w [Tick],
    pub(crate) changed: &'w [Tick],
    pub(crate) changed_by: MaybeLocation<&'w [&'static Location<'static>]>,
    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
}

impl<'w> ContiguousComponentTicksRef<'w> {
    /// # Safety
    /// - The caller must have permission for all given ticks to be read.
    /// - `len` must be the length of `added`, `changed` and `changed_by` (unless none) slices.
    pub(crate) unsafe fn from_slice_ptrs(
        added: ThinSlicePtr<'w, UnsafeCell<Tick>>,
        changed: ThinSlicePtr<'w, UnsafeCell<Tick>>,
        changed_by: MaybeLocation<ThinSlicePtr<'w, UnsafeCell<&'static Location<'static>>>>,
        len: usize,
        this_run: Tick,
        last_run: Tick,
    ) -> Self {
        Self {
            // SAFETY:
            // - The caller ensures that `len` is the length of the slice.
            // - The caller ensures we have permission to read the data.
            added: unsafe { added.cast().as_slice_unchecked(len) },
            // SAFETY: see above.
            changed: unsafe { changed.cast().as_slice_unchecked(len) },
            // SAFETY: see above.
            changed_by: changed_by.map(|v| unsafe { v.cast().as_slice_unchecked(len) }),
            last_run,
            this_run,
        }
    }

    /// Returns an iterator where the i-th item corresponds to whether the i-th component was
    /// marked as changed. If the value equals [`prim@true`], then the component was changed.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct A(pub i32);
    ///
    /// fn some_system(mut query: Query<Ref<A>>) {
    ///     for a in query.contiguous_iter().unwrap() {
    ///         let (a_values, a_ticks) = ContiguousRef::split(a);
    ///         for (value, is_changed) in a_values.iter().zip(a_ticks.is_changed_iter()) {
    ///             if is_changed {
    ///                 // do something
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn is_changed_iter(&self) -> impl Iterator<Item = bool> {
        self.changed
            .iter()
            .map(|v| v.is_newer_than(self.last_run, self.this_run))
    }

    /// Returns an iterator where the i-th item corresponds to whether the i-th component was
    /// marked as added. If the value equals [`prim@true`], then the component was added.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct A(pub i32);
    ///
    /// fn some_system(mut query: Query<Ref<A>>) {
    ///     for a in query.contiguous_iter().unwrap() {
    ///         let (a_values, a_ticks) = ContiguousRef::split(a);
    ///         for (value, is_added) in a_values.iter().zip(a_ticks.is_added_iter()) {
    ///             if is_added {
    ///                 // do something
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn is_added_iter(&self) -> impl Iterator<Item = bool> {
        self.added
            .iter()
            .map(|v| v.is_newer_than(self.last_run, self.this_run))
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

/// Data type storing contiguously lying ticks, which may be accessed to mutate.
///
/// Retrievable via [`ContiguousMut::split`] and probably only useful if you want to use the following
/// methods:
/// - [`ContiguousComponentTicksMut::is_changed_iter`],
/// - [`ContiguousComponentTicksMut::is_added_iter`]
pub struct ContiguousComponentTicksMut<'w> {
    pub(crate) added: &'w mut [Tick],
    pub(crate) changed: &'w mut [Tick],
    pub(crate) changed_by: MaybeLocation<&'w mut [&'static Location<'static>]>,
    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
}

impl<'w> ContiguousComponentTicksMut<'w> {
    /// # Safety
    /// - The caller must have permission to use all given ticks to be mutated.
    /// - `len` must be the length of `added`, `changed` and `changed_by` (unless none) slices.
    pub(crate) unsafe fn from_slice_ptrs(
        added: ThinSlicePtr<'w, UnsafeCell<Tick>>,
        changed: ThinSlicePtr<'w, UnsafeCell<Tick>>,
        changed_by: MaybeLocation<ThinSlicePtr<'w, UnsafeCell<&'static Location<'static>>>>,
        len: usize,
        this_run: Tick,
        last_run: Tick,
    ) -> Self {
        Self {
            // SAFETY:
            // - The caller ensures that `len` is the length of the slice.
            // - The caller ensures we have permission to mutate the data.
            added: unsafe { added.as_mut_slice_unchecked(len) },
            // SAFETY: see above.
            changed: unsafe { changed.as_mut_slice_unchecked(len) },
            // SAFETY: see above.
            changed_by: changed_by.map(|v| unsafe { v.as_mut_slice_unchecked(len) }),
            last_run,
            this_run,
        }
    }

    /// Returns an iterator where the i-th item corresponds to whether the i-th component was
    /// marked as changed. If the value equals [`prim@true`], then the component was changed.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct A(pub i32);
    ///
    /// fn some_system(mut query: Query<&mut A>) {
    ///     for a in query.contiguous_iter_mut().unwrap() {
    ///         let (a_values, a_ticks) = ContiguousMut::split(a);
    ///         for (value, is_changed) in a_values.iter_mut().zip(a_ticks.is_changed_iter()) {
    ///             if is_changed {
    ///                 value.0 *= 10;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn is_changed_iter(&self) -> impl Iterator<Item = bool> {
        self.changed
            .iter()
            .map(|v| v.is_newer_than(self.last_run, self.this_run))
    }

    /// Returns an iterator where the i-th item corresponds to whether the i-th component was
    /// marked as added. If the value equals [`prim@true`], then the component was added.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct A(pub i32);
    ///
    /// fn some_system(mut query: Query<&mut A>) {
    ///     for a in query.contiguous_iter_mut().unwrap() {
    ///         let (a_values, a_ticks) = ContiguousMut::split(a);
    ///         for (value, is_added) in a_values.iter_mut().zip(a_ticks.is_added_iter()) {
    ///             if is_added {
    ///                 value.0 = 10;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn is_added_iter(&self) -> impl Iterator<Item = bool> {
        self.added
            .iter()
            .map(|v| v.is_newer_than(self.last_run, self.this_run))
    }

    /// Marks every tick as changed.
    pub fn mark_all_as_changed(&mut self) {
        let this_run = self.this_run;

        self.changed_by.as_mut().map(|v| {
            for v in v.iter_mut() {
                *v = Location::caller();
            }
        });

        for t in self.changed.iter_mut() {
            *t = this_run;
        }
    }
}

impl<'w> From<ContiguousComponentTicksMut<'w>> for ContiguousComponentTicksRef<'w> {
    fn from(value: ContiguousComponentTicksMut<'w>) -> Self {
        Self {
            added: value.added,
            changed: value.changed,
            changed_by: value.changed_by.map(|v| &*v),
            last_run: value.last_run,
            this_run: value.this_run,
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

/// Contiguous equivalent of [`Ref<T>`].
///
/// Data type returned by [`ContiguousQueryData::fetch_contiguous`](crate::query::ContiguousQueryData::fetch_contiguous) for [`Ref<T>`].
#[derive(Clone)]
pub struct ContiguousRef<'w, T> {
    pub(crate) value: &'w [T],
    pub(crate) ticks: ContiguousComponentTicksRef<'w>,
}

impl<'w, T> ContiguousRef<'w, T> {
    /// Returns the reference wrapped by this type. The reference is allowed to outlive `self`, which makes this method more flexible than simply borrowing `self`.
    pub fn into_inner(self) -> &'w [T] {
        self.value
    }

    /// Returns the added ticks.
    #[inline]
    pub fn added_ticks_slice(&self) -> &'w [Tick] {
        self.ticks.added
    }

    /// Returns the changed ticks.
    #[inline]
    pub fn changed_ticks_slice(&self) -> &'w [Tick] {
        self.ticks.changed
    }

    /// Returns the changed by ticks.
    #[inline]
    pub fn changed_by_ticks_slice(&self) -> MaybeLocation<&[&'static Location<'static>]> {
        self.ticks.changed_by.as_deref()
    }

    /// Returns the tick when the system last ran.
    #[inline]
    pub fn last_run_tick(&self) -> Tick {
        self.ticks.last_run
    }

    /// Returns the tick of the system's current run.
    #[inline]
    pub fn this_run_tick(&self) -> Tick {
        self.ticks.this_run
    }

    /// Creates a new `ContiguousRef` using provided values.
    ///
    /// This is an advanced feature, `ContiguousRef`s are designed to be _created_ by
    /// engine-internal code and _consumed_ by end-user code.
    ///
    /// - `value` - The values wrapped by `ContiguousRef`.
    /// - `added` - [`Tick`]s that store the tick when the wrapped value was created.
    /// - `changed` - [`Tick`]s that store the last time the wrapped value was changed.
    /// - `last_run` - A [`Tick`], occurring before `this_run`, which is used
    ///   as a reference to determine whether the wrapped value is newly added or changed.
    /// - `this_run` - A [`Tick`] corresponding to the current point in time -- "now".
    /// - `caller` - [`Location`]s that store the location when the wrapper value was changed.
    ///
    /// See also: [`Ref::new`]
    pub fn new(
        value: &'w [T],
        added: &'w [Tick],
        changed: &'w [Tick],
        last_run: Tick,
        this_run: Tick,
        caller: MaybeLocation<&'w [&'static Location<'static>]>,
    ) -> Self {
        Self {
            value,
            ticks: ContiguousComponentTicksRef {
                added,
                changed,
                changed_by: caller,
                last_run,
                this_run,
            },
        }
    }

    /// Splits [`ContiguousRef`] into it's inner data types.
    pub fn split(this: Self) -> (&'w [T], ContiguousComponentTicksRef<'w>) {
        (this.value, this.ticks)
    }
}

impl<'w, T> Deref for ContiguousRef<'w, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> AsRef<[T]> for ContiguousRef<'w, T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.deref()
    }
}

impl<'w, T> IntoIterator for ContiguousRef<'w, T> {
    type Item = &'w T;

    type IntoIter = core::slice::Iter<'w, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.value.iter()
    }
}

impl<'w, T: core::fmt::Debug> core::fmt::Debug for ContiguousRef<'w, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ContiguousRef").field(&self.value).finish()
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

/// Data type returned by [`ContiguousQueryData::fetch_contiguous`](crate::query::ContiguousQueryData::fetch_contiguous)
/// for [`Mut<T>`] and `&mut T`
///
/// # Warning
/// Implementations of [`DerefMut`], [`AsMut`] and [`IntoIterator`] update change ticks, which may effect performance.
pub struct ContiguousMut<'w, T> {
    pub(crate) value: &'w mut [T],
    pub(crate) ticks: ContiguousComponentTicksMut<'w>,
}

impl<'w, T> ContiguousMut<'w, T> {
    /// Manually bypasses change detection, allowing you to mutate the underlying values without updating the change tick,
    /// which may be useful to reduce amount of work to be done.
    ///
    /// # Warning
    /// This is a risky operation, that can have unexpected consequences on any system relying on this code.
    /// However, it can be an essential escape hatch when, for example,
    /// you are trying to synchronize representations using change detection and need to avoid infinite recursion.
    #[inline]
    pub fn bypass_change_detection(&mut self) -> &mut [T] {
        self.value
    }

    /// Returns the immutable added ticks' slice.
    #[inline]
    pub fn added_ticks_slice(&self) -> &[Tick] {
        self.ticks.added
    }

    /// Returns the immutable changed ticks' slice.
    #[inline]
    pub fn changed_ticks_slice(&self) -> &[Tick] {
        self.ticks.changed
    }

    /// Returns the mutable changed by ticks' slice
    #[inline]
    pub fn changed_by_ticks_mut(&self) -> MaybeLocation<&[&'static Location<'static>]> {
        self.ticks.changed_by.as_deref()
    }

    /// Returns the tick when the system last ran.
    #[inline]
    pub fn last_run_tick(&self) -> Tick {
        self.ticks.last_run
    }

    /// Returns the tick of the system's current run.
    #[inline]
    pub fn this_run_tick(&self) -> Tick {
        self.ticks.this_run
    }

    /// Returns the mutable added ticks' slice.
    #[inline]
    pub fn added_ticks_slice_mut(&mut self) -> &mut [Tick] {
        self.ticks.added
    }

    /// Returns the mutable changed ticks' slice.
    #[inline]
    pub fn changed_ticks_slice_mut(&mut self) -> &mut [Tick] {
        self.ticks.changed
    }

    /// Returns the mutable changed by ticks' slice
    #[inline]
    pub fn changed_by_ticks_slice_mut(
        &mut self,
    ) -> MaybeLocation<&mut [&'static Location<'static>]> {
        self.ticks.changed_by.as_deref_mut()
    }

    /// Marks all components as changed.
    ///
    /// **Runs in O(n), where n is the amount of rows**
    #[inline]
    pub fn mark_all_as_changed(&mut self) {
        self.ticks.mark_all_as_changed();
    }

    /// Returns a `ContiguousMut<T>` with a smaller lifetime.
    pub fn reborrow(&mut self) -> ContiguousMut<'_, T> {
        ContiguousMut {
            value: self.value,
            ticks: ContiguousComponentTicksMut {
                added: self.ticks.added,
                changed: self.ticks.changed,
                changed_by: self.ticks.changed_by.as_deref_mut(),
                last_run: self.ticks.last_run,
                this_run: self.ticks.this_run,
            },
        }
    }

    /// Splits [`ContiguousMut`] into it's inner data types. It may be useful, when you want to
    /// have an iterator over component values and check ticks simultaneously (using
    /// [`ContiguousComponentTicksMut::is_changed_iter`] and
    /// [`ContiguousComponentTicksMut::is_added_iter`]).
    ///
    /// # Warning
    /// **Bypasses change detection**
    pub fn split(this: Self) -> (&'w mut [T], ContiguousComponentTicksMut<'w>) {
        (this.value, this.ticks)
    }
}

impl<'w, T> Deref for ContiguousMut<'w, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> DerefMut for ContiguousMut<'w, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mark_all_as_changed();
        self.value
    }
}

impl<'w, T> AsRef<[T]> for ContiguousMut<'w, T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.deref()
    }
}

impl<'w, T> AsMut<[T]> for ContiguousMut<'w, T> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self.deref_mut()
    }
}

impl<'w, T> IntoIterator for ContiguousMut<'w, T> {
    type Item = &'w mut T;

    type IntoIter = core::slice::IterMut<'w, T>;

    fn into_iter(mut self) -> Self::IntoIter {
        self.mark_all_as_changed();
        self.value.iter_mut()
    }
}

impl<'w, T: core::fmt::Debug> core::fmt::Debug for ContiguousMut<'w, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ContiguousMut").field(&self.value).finish()
    }
}

impl<'w, T> From<ContiguousMut<'w, T>> for ContiguousRef<'w, T> {
    fn from(value: ContiguousMut<'w, T>) -> Self {
        Self {
            value: value.value,
            ticks: value.ticks.into(),
        }
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
