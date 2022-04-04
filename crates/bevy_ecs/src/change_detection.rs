//! Types that detect when their internal data mutate.

use crate::{component::ComponentTicks, lens::Lens, system::Resource};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use std::ops::{Deref, DerefMut};

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
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
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

            #[inline]
            fn set_changed(&mut self) {
                self.ticks
                    .component_ticks
                    .set_changed(self.ticks.change_tick);
            }
        }

        impl<$($generics),* $(: $traits)?> Deref for $name<$($generics),*> {
            type Target = $target;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.value
            }
        }

        impl<$($generics),* $(: $traits)?> DerefMut for $name<$($generics),*> {
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

macro_rules! impl_into_inner {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
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

macro_rules! impl_lens {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* $(: $traits)?> $name<$($generics),*> {
            #[inline]
            pub fn lens<L: Lens<In = $target>>(self) -> Mut<'a, L::Out> {
                Mut {
                    value: L::get_mut(self.value),
                    ticks: self.ticks,
                }
            }

            #[inline]
            pub fn lens_borrow<'b, L: Lens<In = $target>>(&'b mut self) -> Mut<'b, L::Out> {
                Mut {
                    value: L::get_mut(self.value),
                    ticks: Ticks {
                        component_ticks: self.ticks.component_ticks,
                        last_change_tick: self.ticks.last_change_tick,
                        change_tick: self.ticks.change_tick,
                    },
                }
            }
        }
    };
}

macro_rules! impl_debug {
    ($name:ident < $( $generics:tt ),+ >, $($traits:ident)?) => {
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

pub(crate) struct Ticks<'a> {
    pub(crate) component_ticks: &'a mut ComponentTicks,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

/// Unique mutable borrow of a resource.
///
/// See the [`World`](crate::world::World) documentation to see the usage of a resource.
///
/// If you need a shared borrow, use [`Res`](crate::system::Res) instead.
///
/// # Panics
///
/// Panics when used as a [`SystemParam`](crate::system::SystemParam) if the resource does not exist.
///
/// Use `Option<ResMut<T>>` instead if the resource might not always exist.
pub struct ResMut<'a, T: Resource> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: Ticks<'a>,
}

change_detection_impl!(ResMut<'a, T>, T, Resource);
impl_lens!(ResMut<'a, T>, T, Resource);
impl_into_inner!(ResMut<'a, T>, T, Resource);
impl_debug!(ResMut<'a, T>, Resource);

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
pub struct NonSendMut<'a, T: 'static> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: Ticks<'a>,
}

change_detection_impl!(NonSendMut<'a, T>, T,);
impl_lens!(NonSendMut<'a, T>, T,);
impl_into_inner!(NonSendMut<'a, T>, T,);
impl_debug!(NonSendMut<'a, T>,);

/// Unique mutable borrow of an entity's component
pub struct Mut<'a, T> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: Ticks<'a>,
}

change_detection_impl!(Mut<'a, T>, T,);
impl_lens!(Mut<'a, T>, T,);
impl_into_inner!(Mut<'a, T>, T,);
impl_debug!(Mut<'a, T>,);

/// Unique mutable borrow of a Reflected component
#[cfg(feature = "bevy_reflect")]
pub struct ReflectMut<'a> {
    pub(crate) value: &'a mut dyn Reflect,
    pub(crate) ticks: Ticks<'a>,
}

#[cfg(feature = "bevy_reflect")]
change_detection_impl!(ReflectMut<'a>, dyn Reflect,);
#[cfg(feature = "bevy_reflect")]
impl_into_inner!(ReflectMut<'a>, dyn Reflect,);

mod ops_passthrough {
    use std::ops::*;

    use super::Mut;

    macro_rules! binary_ops {
        ($trait:ident, $method:ident) => {
            impl<Rhs, T: $trait<Rhs> + Copy> $trait<Rhs> for Mut<'_, T> {
                type Output = T::Output;

                fn $method(self, rhs: Rhs) -> Self::Output {
                    T::$method(*self, rhs)
                }
            }
        };
    }
    macro_rules! unary_ops {
        ($trait:ident, $method:ident) => {
            impl<T: $trait + Copy> $trait for Mut<'_, T> {
                type Output = T::Output;

                fn $method(self) -> Self::Output {
                    T::$method(*self)
                }
            }
        };
    }
    macro_rules! assign_ops {
        ($trait:ident, $method:ident) => {
            impl<Rhs, T: $trait<Rhs>> $trait<Rhs> for Mut<'_, T> {
                fn $method(&mut self, rhs: Rhs) {
                    T::$method(&mut *self, rhs)
                }
            }
        };
    }

    binary_ops!(Add, add);
    binary_ops!(Sub, sub);
    binary_ops!(Mul, mul);
    binary_ops!(Div, div);

    binary_ops!(Rem, rem);

    binary_ops!(BitAnd, bitand);
    binary_ops!(BitOr, bitor);
    binary_ops!(BitXor, bitxor);
    binary_ops!(Shr, shr);
    binary_ops!(Shl, shl);

    unary_ops!(Neg, neg);
    unary_ops!(Not, not);

    assign_ops!(AddAssign, add_assign);
    assign_ops!(SubAssign, sub_assign);
    assign_ops!(MulAssign, mul_assign);
    assign_ops!(DivAssign, div_assign);

    assign_ops!(RemAssign, rem_assign);

    assign_ops!(BitAndAssign, bitand_assign);
    assign_ops!(BitOrAssign, bitor_assign);
    assign_ops!(BitXorAssign, bitxor_assign);
    assign_ops!(ShrAssign, shr_assign);
    assign_ops!(ShlAssign, shl_assign);
}
