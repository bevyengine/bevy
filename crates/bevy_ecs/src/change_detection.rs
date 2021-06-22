use crate::component::Component;
use bevy_reflect::Reflect;
use std::ops::{Deref, DerefMut};

/// Types that implement reliable change detection.
///
/// ## Example
/// Using types that implement [`DetectChanges`], such as [`ResMut`], provide
/// a way to query if a value has been mutated in another system.
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// struct MyResource(u32);
///
/// fn my_system(mut resource: ResMut<MyResource>) {
///     if resource.is_changed() {
///         println!("My resource was mutated!");
///     }
/// }
/// ```
///
pub trait DetectChanges {
    /// Returns true if (and only if) this value been added since the last execution of this
    /// system.
    fn is_added(&self) -> bool;

    /// Returns true if (and only if) this value has been mutably accesses since the last execution
    /// of this system.
    fn is_changed(&self) -> bool;
}

/// Types that can trigger reliable change detection.
pub trait SetChanged {
    /// Manually flags this value as having been changed. This normally isn't
    /// required because accessing this pointer mutably automatically flags this
    /// value as "changed".
    ///
    /// **Note**: This operation is irreversible.
    fn set_changed(&mut self);
}

macro_rules! detect_changes_impl {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:tt)?) => {
        impl<$($generics),* $(: $traits)?> DetectChanges for $name<$($generics),*> {
            #[inline]
            fn is_added(&self) -> bool {
                self.ticks
                    .component_ticks
                    .is_added(self.ticks.last_change_tick, self.ticks.change_tick)
            }

            #[inline]
            fn is_changed(&self) -> bool {
                self.ticks
                    .component_ticks
                    .is_changed(self.ticks.last_change_tick, self.ticks.change_tick)
            }
        }

        impl<$($generics),* $(: $traits)?> Deref for $name<$($generics),*> {
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
    };
}

macro_rules! set_changed_impl {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:tt)?) => {
        impl<$($generics),* $(: $traits)?> SetChanged for $name<$($generics),*> {
            #[inline]
            fn set_changed(&mut self) {
                self.ticks
                    .component_ticks
                    .set_changed(self.ticks.change_tick);
            }
        }

        impl<$($generics),* $(: $traits)?> DerefMut for $name<$($generics),*> {
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

macro_rules! impl_into_inner {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:tt)?) => {
        impl<$($generics),* $(: $traits)?> $name<$($generics),*> {
            /// Consume `self` and return a mutable reference to the
            /// contained value while marking `self` as "changed".
            #[inline]
            pub fn into_inner(mut self) -> &'a mut $target {
                self.set_changed();
                self.value
            }
        }
    };
}

macro_rules! impl_debug {
    ($name:ident < $( $generics:tt ),+ >, $($traits:tt)?) => {
        impl<$($generics),* $(: $traits)?> std::fmt::Debug for $name<$($generics),*>
            where T: std::fmt::Debug
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(self.value)
                    .finish()
            }
        }

    };
}

#[derive(Copy, Clone, Debug)]
pub struct ComponentTicks {
    pub(crate) added: u32,
    pub(crate) changed: u32,
}

impl ComponentTicks {
    #[inline]
    pub fn is_added(&self, last_change_tick: u32, change_tick: u32) -> bool {
        // The comparison is relative to `change_tick` so that we can detect changes over the whole
        // `u32` range. Comparing directly the ticks would limit to half that due to overflow
        // handling.
        let component_delta = change_tick.wrapping_sub(self.added);
        let system_delta = change_tick.wrapping_sub(last_change_tick);

        component_delta < system_delta
    }

    #[inline]
    pub fn is_changed(&self, last_change_tick: u32, change_tick: u32) -> bool {
        let component_delta = change_tick.wrapping_sub(self.changed);
        let system_delta = change_tick.wrapping_sub(last_change_tick);

        component_delta < system_delta
    }

    pub(crate) fn new(change_tick: u32) -> Self {
        Self {
            added: change_tick,
            changed: change_tick,
        }
    }

    pub(crate) fn check_ticks(&mut self, change_tick: u32) {
        check_tick(&mut self.added, change_tick);
        check_tick(&mut self.changed, change_tick);
    }

    /// Manually sets the change tick.
    /// Usually, this is done automatically via the [`DerefMut`](std::ops::DerefMut) implementation
    /// on [`Mut`](crate::world::Mut) or [`ResMut`](crate::system::ResMut) etc.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bevy_ecs::{world::World, change_detection::ComponentTicks};
    /// let world: World = unimplemented!();
    /// let component_ticks: ComponentTicks = unimplemented!();
    ///
    /// component_ticks.set_changed(world.read_change_tick());
    /// ```
    #[inline]
    pub fn set_changed(&mut self, change_tick: u32) {
        self.changed = change_tick;
    }
}

fn check_tick(last_change_tick: &mut u32, change_tick: u32) {
    let tick_delta = change_tick.wrapping_sub(*last_change_tick);
    const MAX_DELTA: u32 = (u32::MAX / 4) * 3;
    // Clamp to max delta
    if tick_delta > MAX_DELTA {
        *last_change_tick = change_tick.wrapping_sub(MAX_DELTA);
    }
}

pub(crate) struct Ticks {
    pub(crate) component_ticks: ComponentTicks,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

pub(crate) struct TicksMut<'a> {
    pub(crate) component_ticks: &'a mut ComponentTicks,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

/// Unique mutable borrow of a resource.
///
/// # Panics
///
/// Panics when used as a [`SystemParameter`](crate::system::SystemParam) if the resource does not exist.
///
/// Use `Option<ResMut<T>>` instead if the resource might not always exist.
pub struct ResMut<'a, T: Component> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: TicksMut<'a>,
}

detect_changes_impl!(ResMut<'a, T>, T, Component);
set_changed_impl!(ResMut<'a, T>, T, Component);
impl_into_inner!(ResMut<'a, T>, T, Component);
impl_debug!(ResMut<'a, T>, Component);

/// Shared borrow of a resource.
///
/// # Panics
///
/// Panics when used as a [`SystemParameter`](SystemParam) if the resource does not exist.
///
/// Use `Option<Res<T>>` instead if the resource might not always exist.
pub struct Res<'a, T: Component> {
    pub(crate) value: &'a T,
    pub(crate) ticks: Ticks,
}

detect_changes_impl!(Res<'a, T>, T, Component);
impl_debug!(Res<'a, T>, Component);

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
pub struct NonSendMut<'a, T: 'static> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: TicksMut<'a>,
}

detect_changes_impl!(NonSendMut<'a, T>, T, 'static);
set_changed_impl!(NonSendMut<'a, T>, T, 'static);
impl_into_inner!(NonSendMut<'a, T>, T, 'static);
impl_debug!(NonSendMut<'a, T>, 'static);

/// Shared borrow of a non-[`Send`] resource.
///
/// Only `Send` resources may be accessed with the [`Res`] [`SystemParam`]. In case that the
/// resource does not implement `Send`, this `SystemParam` wrapper can be used. This will instruct
/// the scheduler to instead run the system on the main thread so that it doesn't send the resource
/// over to another thread.
///
/// # Panics
///
/// Panics when used as a `SystemParameter` if the resource does not exist.
pub struct NonSend<'a, T: 'static> {
    pub(crate) value: &'a T,
    pub(crate) ticks: Ticks,
}

detect_changes_impl!(NonSend<'a, T>, T, 'static);
impl_debug!(NonSend<'a, T>, 'static);

/// Unique mutable borrow of an entity's component
pub struct Mut<'a, T> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: TicksMut<'a>,
}

detect_changes_impl!(Mut<'a, T>, T,);
set_changed_impl!(Mut<'a, T>, T,);
impl_into_inner!(Mut<'a, T>, T,);
impl_debug!(Mut<'a, T>,);

/// Unique mutable borrow of a Reflected component
pub struct ReflectMut<'a> {
    pub(crate) value: &'a mut dyn Reflect,
    pub(crate) ticks: TicksMut<'a>,
}

detect_changes_impl!(ReflectMut<'a>, dyn Reflect,);
set_changed_impl!(ReflectMut<'a>, dyn Reflect,);
