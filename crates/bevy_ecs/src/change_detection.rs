//! Types that detect when their internal data mutate.

use crate::{
    component::{Tick, TickCells},
    ptr::PtrMut,
    system::Resource,
};
use bevy_ptr::{Ptr, UnsafeCellDeref};
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

    /// Returns `true` if this value was added or mutably dereferenced after the system last ran.
    fn is_changed(&self) -> bool;

    /// Returns the change tick recording the time this data was most recently changed.
    ///
    /// Note that components and resources are also marked as changed upon insertion.
    ///
    /// For comparison, the previous change tick of a system can be read using the
    /// [`SystemChangeTick`](crate::system::SystemChangeTick)
    /// [`SystemParam`](crate::system::SystemParam).
    fn last_changed(&self) -> u32;
}

/// Types that implement reliable change detection.
///
/// ## Example
/// Using types that implement [`DetectChangesMut`], such as [`ResMut`], provide
/// a way to query if a value has been mutated in another system.
/// Normally change detection is triggered by either [`DerefMut`] or [`AsMut`], however
/// it can be manually triggered via [`set_if_neq`](`DetectChangesMut::set_changed`).
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

    /// Manually sets the change tick recording the time when this data was last mutated.
    ///
    /// # Warning
    /// This is a complex and error-prone operation, primarily intended for use with rollback networking strategies.
    /// If you merely want to flag this data as changed, use [`set_changed`](DetectChangesMut::set_changed) instead.
    /// If you want to avoid triggering change detection, use [`bypass_change_detection`](DetectChangesMut::bypass_change_detection) instead.
    fn set_last_changed(&mut self, last_change_tick: u32);

    /// Manually bypasses change detection, allowing you to mutate the underlying value without updating the change tick.
    ///
    /// # Warning
    /// This is a risky operation, that can have unexpected consequences on any system relying on this code.
    /// However, it can be an essential escape hatch when, for example,
    /// you are trying to synchronize representations using change detection and need to avoid infinite recursion.
    fn bypass_change_detection(&mut self) -> &mut Self::Inner;

    /// Sets `self` to `value`, if and only if `*self != *value`
    ///
    /// `T` is the type stored within the smart pointer (e.g. [`Mut`] or [`ResMut`]).
    ///
    /// This is useful to ensure change detection is only triggered when the underlying value
    /// changes, instead of every time [`DerefMut`] is used.
    fn set_if_neq<Target>(&mut self, value: Target)
    where
        Self: Deref<Target = Target> + DerefMut<Target = Target>,
        Target: PartialEq;
}

macro_rules! change_detection_impl {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> DetectChanges for $name<$($generics),*> {
            #[inline]
            fn is_added(&self) -> bool {
                self.ticks
                    .added
                    .is_newer_than(self.ticks.last_change_tick, self.ticks.change_tick)
            }

            #[inline]
            fn is_changed(&self) -> bool {
                self.ticks
                    .changed
                    .is_newer_than(self.ticks.last_change_tick, self.ticks.change_tick)
            }

            #[inline]
            fn last_changed(&self) -> u32 {
                self.ticks.changed.tick
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
            fn set_changed(&mut self) {
                self.ticks
                    .changed
                    .set_changed(self.ticks.change_tick);
            }

            #[inline]
            fn set_last_changed(&mut self, last_changed: u32) {
                self.ticks
                    .changed
                    .set_changed(last_changed);
            }

            #[inline]
            fn bypass_change_detection(&mut self) -> &mut Self::Inner {
                self.value
            }

            #[inline]
            fn set_if_neq<Target>(&mut self, value: Target)
            where
                Self: Deref<Target = Target> + DerefMut<Target = Target>,
                Target: PartialEq,
            {
                // This dereference is immutable, so does not trigger change detection
                if *<Self as Deref>::deref(self) != value {
                    // `DerefMut` usage triggers change detection
                    *<Self as DerefMut>::deref_mut(self) = value;
                }
            }
        }

        impl<$($generics),* : ?Sized $(+ $traits)?> DerefMut for $name<$($generics),*> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.set_changed();
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
            pub fn into_inner(mut self) -> &'a mut $target {
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
                        last_change_tick: self.ticks.last_change_tick,
                        change_tick: self.ticks.change_tick,
                    }
                }
            }

            /// Maps to an inner value by applying a function to the contained reference, without flagging a change.
            ///
            /// You should never modify the argument passed to the closure -- if you want to modify the data
            /// without flagging a change, consider using [`DetectChangesMut::bypass_change_detection`] to make your intent explicit.
            ///
            /// ```rust
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

#[derive(Clone)]
pub(crate) struct Ticks<'a> {
    pub(crate) added: &'a Tick,
    pub(crate) changed: &'a Tick,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

impl<'a> Ticks<'a> {
    /// # Safety
    /// This should never alias the underlying ticks with a mutable one such as `TicksMut`.
    #[inline]
    pub(crate) unsafe fn from_tick_cells(
        cells: TickCells<'a>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        Self {
            added: cells.added.deref(),
            changed: cells.changed.deref(),
            last_change_tick,
            change_tick,
        }
    }
}

pub(crate) struct TicksMut<'a> {
    pub(crate) added: &'a mut Tick,
    pub(crate) changed: &'a mut Tick,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

impl<'a> TicksMut<'a> {
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

impl<'a> From<TicksMut<'a>> for Ticks<'a> {
    fn from(ticks: TicksMut<'a>) -> Self {
        Ticks {
            added: ticks.added,
            changed: ticks.changed,
            last_change_tick: ticks.last_change_tick,
            change_tick: ticks.change_tick,
        }
    }
}

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
    // no it shouldn't clippy
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

impl<'w, T: Resource> From<ResMut<'w, T>> for Res<'w, T> {
    fn from(res: ResMut<'w, T>) -> Self {
        Self {
            value: res.value,
            ticks: res.ticks.into(),
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
/// If you need a shared borrow, use [`Res`](crate::system::Res) instead.
///
/// # Panics
///
/// Panics when used as a [`SystemParam`](crate::system::SystemParam) if the resource does not exist.
///
/// Use `Option<ResMut<T>>` instead if the resource might not always exist.
pub struct ResMut<'a, T: ?Sized + Resource> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: TicksMut<'a>,
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
change_detection_mut_impl!(ResMut<'a, T>, T, Resource);
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
    pub(crate) ticks: TicksMut<'a>,
}

change_detection_impl!(NonSendMut<'a, T>, T,);
change_detection_mut_impl!(NonSendMut<'a, T>, T,);
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

/// Shared borrow of an entity's component with access to change detection.
/// Similar to [`Mut`] but is immutable and so doesn't require unique access.
pub struct Ref<'a, T: ?Sized> {
    pub(crate) value: &'a T,
    pub(crate) ticks: Ticks<'a>,
}

impl<'a, T: ?Sized> Ref<'a, T> {
    pub fn into_inner(self) -> &'a T {
        self.value
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
change_detection_impl!(Ref<'a, T>, T,);
impl_debug!(Ref<'a, T>,);

/// Unique mutable borrow of an entity's component
pub struct Mut<'a, T: ?Sized> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: TicksMut<'a>,
}

impl<'a, T: ?Sized> From<Mut<'a, T>> for Ref<'a, T> {
    fn from(mut_ref: Mut<'a, T>) -> Self {
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

change_detection_impl!(Mut<'a, T>, T,);
change_detection_mut_impl!(Mut<'a, T>, T,);
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
    pub(crate) ticks: TicksMut<'a>,
}

impl<'a> MutUntyped<'a> {
    /// Returns the pointer to the value, marking it as changed.
    ///
    /// In order to avoid marking the value as changed, you need to call [`bypass_change_detection`](DetectChangesMut::bypass_change_detection).
    #[inline]
    pub fn into_inner(mut self) -> PtrMut<'a> {
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
                last_change_tick: self.ticks.last_change_tick,
                change_tick: self.ticks.change_tick,
            },
        }
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

    /// Transforms this [`MutUntyped`] into a [`Mut<T>`] with the same lifetime.
    ///
    /// # Safety
    /// - `T` must be the erased pointee type for this [`MutUntyped`].
    pub unsafe fn with_type<T>(self) -> Mut<'a, T> {
        Mut {
            value: self.value.deref_mut(),
            ticks: self.ticks,
        }
    }
}

impl<'a> DetectChanges for MutUntyped<'a> {
    #[inline]
    fn is_added(&self) -> bool {
        self.ticks
            .added
            .is_newer_than(self.ticks.last_change_tick, self.ticks.change_tick)
    }

    #[inline]
    fn is_changed(&self) -> bool {
        self.ticks
            .changed
            .is_newer_than(self.ticks.last_change_tick, self.ticks.change_tick)
    }

    #[inline]
    fn last_changed(&self) -> u32 {
        self.ticks.changed.tick
    }
}

impl<'a> DetectChangesMut for MutUntyped<'a> {
    type Inner = PtrMut<'a>;

    #[inline]
    fn set_changed(&mut self) {
        self.ticks.changed.set_changed(self.ticks.change_tick);
    }

    #[inline]
    fn set_last_changed(&mut self, last_changed: u32) {
        self.ticks.changed.set_changed(last_changed);
    }

    #[inline]
    fn bypass_change_detection(&mut self) -> &mut Self::Inner {
        &mut self.value
    }

    #[inline]
    fn set_if_neq<Target>(&mut self, value: Target)
    where
        Self: Deref<Target = Target> + DerefMut<Target = Target>,
        Target: PartialEq,
    {
        // This dereference is immutable, so does not trigger change detection
        if *<Self as Deref>::deref(self) != value {
            // `DerefMut` usage triggers change detection
            *<Self as DerefMut>::deref_mut(self) = value;
        }
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
        change_detection::{
            Mut, NonSendMut, ResMut, TicksMut, CHECK_TICK_THRESHOLD, MAX_CHANGE_AGE,
        },
        component::{Component, ComponentTicks, Tick},
        query::ChangeTrackers,
        system::{IntoSystem, Query, System},
        world::World,
    };

    use super::DetectChanges;
    use super::DetectChangesMut;

    #[derive(Component, PartialEq)]
    struct C;

    #[derive(Resource)]
    struct R;

    #[derive(Resource, PartialEq)]
    struct R2(u8);

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
        let ticks = TicksMut {
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
        let ticks = TicksMut {
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
        let ticks = TicksMut {
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
}
