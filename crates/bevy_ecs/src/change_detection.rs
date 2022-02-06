//! Types that detect when their internal data mutate.

use crate::{component::ComponentTicks, system::Resource};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use std::ops::{Deref, DerefMut, Index, IndexMut, Neg, Not, BitAnd, BitOr, BitOrAssign, BitXor, Div, RangeBounds, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Mul, Add, Sub, AddAssign, SubAssign, DivAssign, MulAssign};
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
impl_into_inner!(NonSendMut<'a, T>, T,);
impl_debug!(NonSendMut<'a, T>,);

/// Unique mutable borrow of an entity's component
pub struct Mut<'a, T> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: Ticks<'a>,
}

impl <'a, T, U, V> Add<U> for Mut<'a, T> where T: Add<U, Output = V> + Copy {
    type Output = V;

    fn add(self, rhs: U) -> Self::Output {
        self.value.clone() + rhs
    }
}

impl <'a, T, U> AddAssign<U> for Mut<'a, T> where T: AddAssign<U> {
    fn add_assign(&mut self, rhs: U) {
        *self.value += rhs;
    }
}

impl <'a, T, U, V> BitAnd<U> for Mut<'a, T> where T: BitAnd<U, Output = V> + Copy {
    type Output = V;

    fn bitand(self, rhs: U) -> Self::Output {
        *self & rhs
    }
}

impl <'a, T, U, V> BitOr<U> for Mut<'a, T> where T: BitOr<U, Output = V> + Copy {
    type Output = V;

    fn bitor(self, rhs: U) -> Self::Output {
        *self | rhs
    }
}

impl <'a, T, U> BitOrAssign<U> for Mut<'a, T> where T: BitOrAssign<U> {
    fn bitor_assign(&mut self, rhs: U) {
        *self.value |= rhs;
    }
}

impl <'a, T, U, V> BitXor<U> for Mut<'a, T> where T: BitXor<U, Output = V> + Copy {
    type Output = V;

    fn bitxor(self, rhs: U) -> Self::Output {
        *self ^ rhs
    }
}

impl <'a, T, U> Div<U> for Mut<'a, T> where T: Div<U, Output = T> + Copy {
    type Output = T;

    fn div(self, rhs: U) -> Self::Output {
        *self.value / rhs
    }
}

impl <'a, T, U> DivAssign<U> for Mut<'a, T> where T: DivAssign<U> {
    fn div_assign(&mut self, rhs: U) {
        *self.value /= rhs;
    }
}

impl <'a, T, U> Mul<U> for Mut<'a, T> where T: Mul<U, Output = T> + Copy {
    type Output = T;

    fn mul(self, rhs: U) -> Self::Output {
        *self.value * rhs
    }
}

impl <'a, T, U> MulAssign<U> for Mut<'a, T> where T: MulAssign<U> {
    fn mul_assign(&mut self, rhs: U) {
        *self.value *= rhs;
    }
}

impl <'a, T> RangeBounds<T> for Mut<'a, T> where T: RangeBounds<T> {
    fn start_bound(&self) -> std::ops::Bound<&T> {
        self.as_ref().start_bound()
    }

    fn end_bound(&self) -> std::ops::Bound<&T> {
        self.as_ref().end_bound()
    }
}

impl <'a, T, U> Rem<T> for Mut<'a, T> where T: Rem<T, Output = U> + Copy {
    type Output = U;
    fn rem(self, rhs: T) -> Self::Output {
        *self % rhs
    }
}

impl <'a, T, U> RemAssign<U> for Mut<'a, T> where T: RemAssign<U>  {
    fn rem_assign(&mut self, rhs: U) {
        *self.value %= rhs;
    }
}

impl <'a, T, U> Shl<T> for Mut<'a, T> where T: Shl<T, Output = U> + Copy {
    type Output = U;
    fn shl(self, rhs: T) -> Self::Output {
        *self << rhs
    }
}

impl <'a, T, U> ShlAssign<U> for Mut<'a, T> where T: ShlAssign<U> {
    fn shl_assign(&mut self, rhs: U) {
        *self.value <<= rhs;
    }
}

impl <'a, T, U> Shr<T> for Mut<'a, T> where T: Shr<T, Output = U> + Copy {
    type Output = U;
    fn shr(self, rhs: T) -> Self::Output {
        *self >> rhs
    }
}

impl <'a, T, U> ShrAssign<U> for Mut<'a, T> where T: ShrAssign<U> {
    fn shr_assign(&mut self, rhs: U) {
        *self.value >>= rhs;
    }
}

impl <'a, T, U> SubAssign<U> for Mut<'a, T> where T: SubAssign<U>  {
    fn sub_assign(&mut self, rhs: U) {
        *self.value -= rhs;
    }
}

impl <'a, T, U> Sub<U> for Mut <'a, T> where T: Sub<U, Output = T>, T: Copy {
    type Output = T;

    fn sub(self, rhs: U) -> Self::Output {
        *self.value - rhs
    }
}

impl <'a, T, U, V> Index<U> for Mut<'a, T> where T: Index<U, Output = V> {
    type Output = V;

    fn index(&self, index: U) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl <'a, T, U, V> IndexMut<U> for Mut<'a, T> where T: IndexMut<U, Output = V> {
    fn index_mut(&mut self, index: U) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}

impl <'a, T, U> Neg for Mut<'a, T> where T: Neg<Output = U> + Copy {
    type Output = U;

    fn neg(self) -> Self::Output {
       - *self
    }
}

impl <'a, T, U> Not for Mut<'a, T> where T: Not<Output = U> + Copy {
    type Output = U;

    fn not(self) -> Self::Output {
        ! *self
    }
}

change_detection_impl!(Mut<'a, T>, T,);
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