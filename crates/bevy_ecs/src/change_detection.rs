use crate::component::{Component, ComponentTicks};
use bevy_reflect::Reflect;
use std::ops::{Deref, DerefMut};

pub trait ChangeDetectable {
    /// Returns true if (and only if) this value been added since the last execution of this
    /// system.
    fn is_added(&self) -> bool;

    /// Returns true if (and only if) this value been changed since the last execution of this
    /// system.
    fn is_changed(&self) -> bool;

    /// Manually flags this value as having been changed. This normally isn't
    /// required because accessing this pointer mutably automatically flags this
    /// value as "changed".
    ///
    /// **Note**: This operation is irreversible.
    fn set_changed(&mut self);
}

macro_rules! change_detection_impl {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident),*) => {
        impl<$($generics),*: $($traits),*> ChangeDetectable for $name<$($generics),*>
        {
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

            #[inline]
            fn set_changed(&mut self) {
                self.ticks
                    .component_ticks
                    .set_changed(self.ticks.change_tick);
            }
        }

        impl<$($generics),*: $($traits),*> Deref for $name<$($generics),*>
        {
            type Target = $target;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.value
            }
        }

        impl<$($generics),*: $($traits),*> DerefMut for $name<$($generics),*>
        {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.set_changed();
                self.value
            }
        }

        impl<$($generics),*: $($traits),*> AsRef<$target> for $name<$($generics),*>
        {
            #[inline]
            fn as_ref(&self) -> &$target {
                self.deref()
            }
        }

        impl<$($generics),*: $($traits),*> AsMut<$target> for $name<$($generics),*>
        {
            #[inline]
            fn as_mut(&mut self) -> &mut $target {
                self.deref_mut()
            }
        }
    }
}

macro_rules! impl_into_inner {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident),*) => {
        impl<$($generics),*: $($traits),*> $name<$($generics),*>
        {
            /// Consume `self` and return the contained value while marking `self` as "changed".
            #[inline]
            pub fn into_inner(mut self) -> &'a mut $target {
                self.set_changed();
                self.value
            }
        }
    };
}

macro_rules! impl_debug {
    ($name:ident < $( $generics:tt ),+ >, $($traits:ident),*) => {
        impl<$($generics),*: $($traits),*> std::fmt::Debug for $name<$($generics),*>
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

pub(crate) struct Ticks<'a> {
    pub(crate) component_ticks: &'a mut ComponentTicks,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

/// Unique borrow of a resource.
///
/// # Panics
///
/// Panics when used as a [`SystemParameter`](SystemParam) if the resource does not exist.
///
/// Use `Option<ResMut<T>>` instead if the resource might not always exist.
pub struct ResMut<'a, T: Component> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: Ticks<'a>,
}

change_detection_impl!(ResMut<'a, T>, T, Component);
impl_into_inner!(ResMut<'a, T>, T, Component);
impl_debug!(ResMut<'a, T>, Component);

/// Unique borrow of an entity's component
pub struct Mut<'a, T> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: Ticks<'a>,
}

change_detection_impl!(Mut<'a, T>, T,);
impl_into_inner!(Mut<'a, T>, T,);
impl_debug!(Mut<'a, T>,);

/// Unique borrow of a Reflected component
pub struct ReflectMut<'a> {
    pub(crate) value: &'a mut dyn Reflect,
    pub(crate) ticks: Ticks<'a>,
}

change_detection_impl!(ReflectMut<'a>, dyn Reflect,);
