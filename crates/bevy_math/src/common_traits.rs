//! This module contains abstract mathematical traits shared by types used in `bevy_math`.

use crate::{ops, Dir2, Dir3, Dir3A, Isometry2d, Isometry3d, Quat, Rot2, Vec2, Vec3, Vec3A, Vec4};
use core::{
    fmt::Debug,
    ops::{Add, Div, Mul, Neg, Sub},
};
use variadics_please::all_tuples_enumerated;

/// A type with a natural method of smooth interpolation. This is intended to be the "nicest" or form of
/// interpolation available for a given type, and defines the interpolation method used by the animation
/// system. It may not necessarily be the fastest form of interpolation available.
///
/// Interpolation is a fairly fluid concept, so to make things a little more predictable we require the
/// following rules to hold:
///
/// 1. The notion of interpolation should follow naturally from the semantics of the type, so
///    that inferring the interpolation mode from the type alone is sensible.
///
/// 2. The path traced by interpolating between two points should be continuous and smooth,
///    as far as the limits of floating-point arithmetic permit. This trait should not be
///    implemented for types that don't have an apparent notion of continuity (like `bool`).
///
/// 3. Interpolation should recover something equivalent to the starting value at `t = 0.0`
///    and likewise with the ending value at `t = 1.0`. They do not have to be data-identical, but
///    they should be semantically identical. For example, [`Quat::slerp`] doesn't always yield its
///    second rotation input exactly at `t = 1.0`, but it always returns an equivalent rotation
///    (this trait is implemented for `Quat` using `slerp`).
///
/// 4. Interpolation should be the same forward and backwards. That is, `interp(a, b, t)` should
///    be equivalent to `interp(b, a, t - 1)`.
///
/// 5. Interpolating from a value to itself `interp(a, a, t)` should always return values equivalent
///    to the original value `a`.
///
///
/// We make no guarantees about the behavior of `interp` for values outside of the interval `[0, 1]`.
/// Other sub-traits (such as [`InterpolateStable`] or [`VectorSpace`]) may add additional guarantees,
/// such as linearity.
pub trait Interpolate: Sized {
    /// Smoothly interpolates between two values. There are often many ways to interpolate for a given
    /// type, but this method always represents a sane default interpolation method.
    ///
    /// Other sub-traits may add stronger properties to this method:
    /// - For types that implement `VectorSpace`, this is linear interpolation.
    /// - For types that implement `InterpolateStable`, the interpolation is stable under resampling.
    fn interp(&self, other: &Self, param: f32) -> Self;

    /// Performs interpolation in place. See the documentation on the [`interp`] method for more info.
    ///
    /// [`interp`]: Interpolate::interp
    #[inline]
    fn interp_assign(&mut self, other: &Self, param: f32) {
        *self = self.interp(other, param);
    }
}

macro_rules! impl_interpolate_tuple {
    ($(#[$meta:meta])* $(($n:tt, $T:ident)),*) => {
        $(#[$meta])*
        impl<$($T: Interpolate),*> Interpolate for ($($T,)*) {
            #[inline]
            fn interp(&self, other: &Self, param: f32) -> Self {
                (
                    $(
                        <$T as Interpolate>::interp(&self.$n, &other.$n, param),
                    )*
                )
            }
        }
    };
}

all_tuples_enumerated!(
    #[doc(fake_variadic)]
    impl_interpolate_tuple,
    1,
    11,
    T
);

impl Interpolate for Rot2 {
    #[inline]
    fn interp(&self, other: &Self, param: f32) -> Self {
        self.slerp(*other, param)
    }
}

impl Interpolate for Quat {
    #[inline]
    fn interp(&self, other: &Self, param: f32) -> Self {
        self.slerp(*other, param)
    }
}

impl Interpolate for Dir2 {
    #[inline]
    fn interp(&self, other: &Self, param: f32) -> Self {
        self.slerp(*other, param)
    }
}

impl Interpolate for Dir3 {
    #[inline]
    fn interp(&self, other: &Self, param: f32) -> Self {
        self.slerp(*other, param)
    }
}

impl Interpolate for Dir3A {
    #[inline]
    fn interp(&self, other: &Self, param: f32) -> Self {
        self.slerp(*other, param)
    }
}

impl Interpolate for Isometry2d {
    fn interp(&self, other: &Self, param: f32) -> Self {
        Isometry2d {
            rotation: self.rotation.interp(&other.rotation, param),
            translation: self.translation.interp(&other.translation, param),
        }
    }
}

impl Interpolate for Isometry3d {
    fn interp(&self, other: &Self, param: f32) -> Self {
        Isometry3d {
            rotation: self.rotation.interp(&other.rotation, param),
            translation: self.translation.interp(&other.translation, param),
        }
    }
}

/// A type that supports the mathematical operations of a real vector space, irrespective of dimension.
/// In particular, this means that the implementing type supports:
/// - Scalar multiplication and division on the right by elements of `f32`
/// - Negation
/// - Addition and subtraction
/// - Zero
///
/// Within the limitations of floating point arithmetic, all the following are required to hold:
/// - (Associativity of addition) For all `u, v, w: Self`, `(u + v) + w == u + (v + w)`.
/// - (Commutativity of addition) For all `u, v: Self`, `u + v == v + u`.
/// - (Additive identity) For all `v: Self`, `v + Self::ZERO == v`.
/// - (Additive inverse) For all `v: Self`, `v - v == v + (-v) == Self::ZERO`.
/// - (Compatibility of multiplication) For all `a, b: f32`, `v: Self`, `v * (a * b) == (v * a) * b`.
/// - (Multiplicative identity) For all `v: Self`, `v * 1.0 == v`.
/// - (Distributivity for vector addition) For all `a: f32`, `u, v: Self`, `(u + v) * a == u * a + v * a`.
/// - (Distributivity for scalar addition) For all `a, b: f32`, `v: Self`, `v * (a + b) == v * a + v * b`.
///
/// Note that, because implementing types use floating point arithmetic, they are not required to actually
/// implement `PartialEq` or `Eq`.
///
/// Also note that all vector spaces implement [`Interpolate`] with linear interpolation via a blanket-impl.
pub trait VectorSpace:
    Interpolate
    + Mul<f32, Output = Self>
    + Div<f32, Output = Self>
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Neg<Output = Self>
    + Default
    + Debug
    + Clone
    + Copy
{
    /// The zero vector, which is the identity of addition for the vector space type.
    const ZERO: Self;
}

// Equip all vector spaces with linear interpolation. This will conflict with other implementations of
// interpolation for vector spaces; that's intentional, linear interpolation is the only sane default
// for a vector-space.
impl<V> Interpolate for V
where
    V: VectorSpace,
{
    #[inline]
    fn interp(&self, other: &Self, param: f32) -> Self {
        *self * (1. - param) + *other * param
    }
}

impl VectorSpace for Vec4 {
    const ZERO: Self = Vec4::ZERO;
}

impl VectorSpace for Vec3 {
    const ZERO: Self = Vec3::ZERO;
}

impl VectorSpace for Vec3A {
    const ZERO: Self = Vec3A::ZERO;
}

impl VectorSpace for Vec2 {
    const ZERO: Self = Vec2::ZERO;
}

impl VectorSpace for f32 {
    const ZERO: Self = 0.0;
}

/// A type consisting of formal sums of elements from `V` and `W`. That is,
/// each value `Sum(v, w)` is thought of as `v + w`, with no available
/// simplification. In particular, if `V` and `W` are [vector spaces], then
/// `Sum<V, W>` is a vector space whose dimension is the sum of those of `V`
/// and `W`, and the field accessors `.0` and `.1` are vector space projections.
///
/// [vector spaces]: VectorSpace
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct Sum<V, W>(pub V, pub W);

impl<V, W> Mul<f32> for Sum<V, W>
where
    V: VectorSpace,
    W: VectorSpace,
{
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Sum(self.0 * rhs, self.1 * rhs)
    }
}

impl<V, W> Div<f32> for Sum<V, W>
where
    V: VectorSpace,
    W: VectorSpace,
{
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Sum(self.0 / rhs, self.1 / rhs)
    }
}

impl<V, W> Add<Self> for Sum<V, W>
where
    V: VectorSpace,
    W: VectorSpace,
{
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Sum(self.0 + other.0, self.1 + other.1)
    }
}

impl<V, W> Sub<Self> for Sum<V, W>
where
    V: VectorSpace,
    W: VectorSpace,
{
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Sum(self.0 - other.0, self.1 - other.1)
    }
}

impl<V, W> Neg for Sum<V, W>
where
    V: VectorSpace,
    W: VectorSpace,
{
    type Output = Self;
    fn neg(self) -> Self::Output {
        Sum(-self.0, -self.1)
    }
}

impl<V, W> Default for Sum<V, W>
where
    V: VectorSpace,
    W: VectorSpace,
{
    fn default() -> Self {
        Sum(V::default(), W::default())
    }
}

impl<V, W> VectorSpace for Sum<V, W>
where
    V: VectorSpace,
    W: VectorSpace,
{
    const ZERO: Self = Sum(V::ZERO, W::ZERO);
}

/// A type that supports the operations of a normed vector space; i.e. a norm operation in addition
/// to those of [`VectorSpace`]. Specifically, the implementer must guarantee that the following
/// relationships hold, within the limitations of floating point arithmetic:
/// - (Nonnegativity) For all `v: Self`, `v.norm() >= 0.0`.
/// - (Positive definiteness) For all `v: Self`, `v.norm() == 0.0` implies `v == Self::ZERO`.
/// - (Absolute homogeneity) For all `c: f32`, `v: Self`, `(v * c).norm() == v.norm() * c.abs()`.
/// - (Triangle inequality) For all `v, w: Self`, `(v + w).norm() <= v.norm() + w.norm()`.
///
/// Note that, because implementing types use floating point arithmetic, they are not required to actually
/// implement `PartialEq` or `Eq`.
pub trait NormedVectorSpace: VectorSpace {
    /// The size of this element. The return value should always be nonnegative.
    fn norm(self) -> f32;

    /// The squared norm of this element. Computing this is often faster than computing
    /// [`NormedVectorSpace::norm`].
    #[inline]
    fn norm_squared(self) -> f32 {
        self.norm() * self.norm()
    }

    /// The distance between this element and another, as determined by the norm.
    #[inline]
    fn distance(self, rhs: Self) -> f32 {
        (rhs - self).norm()
    }

    /// The squared distance between this element and another, as determined by the norm. Note that
    /// this is often faster to compute in practice than [`NormedVectorSpace::distance`].
    #[inline]
    fn distance_squared(self, rhs: Self) -> f32 {
        (rhs - self).norm_squared()
    }
}

impl NormedVectorSpace for Vec4 {
    #[inline]
    fn norm(self) -> f32 {
        self.length()
    }

    #[inline]
    fn norm_squared(self) -> f32 {
        self.length_squared()
    }
}

impl NormedVectorSpace for Vec3 {
    #[inline]
    fn norm(self) -> f32 {
        self.length()
    }

    #[inline]
    fn norm_squared(self) -> f32 {
        self.length_squared()
    }
}

impl NormedVectorSpace for Vec3A {
    #[inline]
    fn norm(self) -> f32 {
        self.length()
    }

    #[inline]
    fn norm_squared(self) -> f32 {
        self.length_squared()
    }
}

impl NormedVectorSpace for Vec2 {
    #[inline]
    fn norm(self) -> f32 {
        self.length()
    }

    #[inline]
    fn norm_squared(self) -> f32 {
        self.length_squared()
    }
}

impl NormedVectorSpace for f32 {
    #[inline]
    fn norm(self) -> f32 {
        ops::abs(self)
    }

    #[inline]
    fn norm_squared(self) -> f32 {
        self * self
    }
}

/// This trait extends [`Interpolate`] with strong subdivision guarantees.
///
/// The interpolation (`Interpolate::interp`) must be *subdivision-stable*: for any interpolation curve
/// between two (unnamed) values and any parameter-value pairs `(t0, p)` and `(t1, q)`, the
/// interpolation curve between `p` and `q` must be the *linear* reparameterization of the original
/// interpolation curve restricted to the interval `[t0, t1]`.
///
/// This condition is very strong, and indicates something like constant speed. It  is called
/// "subdivision stability" because it guarantees that breaking up the interpolation into segments and
/// joining them back together has no effect.
///
/// Here is a diagram depicting it:
/// ```text
/// top curve = T::interp(u, v, t)
///
///              t0 => p   t1 => q    
///   |-------------|---------|-------------|
/// 0 => u         /           \          1 => v
///              /               \
///            /                   \
///          /        linear         \
///        /     reparameterization    \
///      /   t = t0 * (1 - s) + t1 * s   \
///    /                                   \
///   |-------------------------------------|
/// 0 => p                                1 => q
///
/// bottom curve = T::interp(p, q, s)
/// ```
///
/// Note that some common forms of interpolation do not satisfy this criterion. For example,
/// [`Quat::lerp`] and [`Rot2::nlerp`] are not subdivision-stable.
///
/// [`Quat::slerp`]: crate::Quat::slerp
/// [`Quat::lerp`]: crate::Quat::lerp
/// [`Rot2::nlerp`]: crate::Rot2::nlerp
pub trait InterpolateStable: Interpolate {
    /// Smoothly nudge this value towards the `target` at a given decay rate. The `decay_rate`
    /// parameter controls how fast the distance between `self` and `target` decays relative to
    /// the units of `delta`; the intended usage is for `decay_rate` to generally remain fixed,
    /// while `delta` is something like `delta_time` from an updating system. This produces a
    /// smooth following of the target that is independent of framerate.
    ///
    /// More specifically, when this is called repeatedly, the result is that the distance between
    /// `self` and a fixed `target` attenuates exponentially, with the rate of this exponential
    /// decay given by `decay_rate`.
    ///
    /// For example, at `decay_rate = 0.0`, this has no effect.
    /// At `decay_rate = f32::INFINITY`, `self` immediately snaps to `target`.
    /// In general, higher rates mean that `self` moves more quickly towards `target`.
    ///
    /// # Example
    /// ```
    /// # use bevy_math::{Vec3, InterpolateStable};
    /// # let delta_time: f32 = 1.0 / 60.0;
    /// let mut object_position: Vec3 = Vec3::ZERO;
    /// let target_position: Vec3 = Vec3::new(2.0, 3.0, 5.0);
    /// // Decay rate of ln(10) => after 1 second, remaining distance is 1/10th
    /// let decay_rate = f32::ln(10.0);
    /// // Calling this repeatedly will move `object_position` towards `target_position`:
    /// object_position.smooth_nudge(&target_position, decay_rate, delta_time);
    /// ```
    #[inline]
    fn smooth_nudge(&mut self, target: &Self, decay_rate: f32, delta: f32) {
        self.interp_assign(target, 1.0 - ops::exp(-decay_rate * delta));
    }
}

// Conservatively, we presently only apply this for normed vector spaces, where the notion
// of being constant-speed is literally true. The technical axioms are satisfied for any
// VectorSpace type, but the "natural from the semantics" part is less clear in general.
impl<V> InterpolateStable for V where V: NormedVectorSpace {}

impl InterpolateStable for Rot2 {}

impl InterpolateStable for Quat {}

impl InterpolateStable for Dir2 {}

impl InterpolateStable for Dir3 {}

impl InterpolateStable for Dir3A {}

impl InterpolateStable for Isometry2d {}

impl InterpolateStable for Isometry3d {}

macro_rules! impl_stable_interpolate_tuple {
    ($(#[$meta:meta])* $(($n:tt, $T:ident)),*) => {
        $(#[$meta])*
        impl<$($T: InterpolateStable),*> InterpolateStable for ($($T,)*) {}
    };
}

all_tuples_enumerated!(
    #[doc(fake_variadic)]
    impl_stable_interpolate_tuple,
    1,
    11,
    T
);

/// A type that has tangents.
pub trait HasTangent {
    /// The tangent type.
    type Tangent: VectorSpace;
}

/// A value with its derivative.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct WithDerivative<T>
where
    T: HasTangent,
{
    /// The underlying value.
    pub value: T,

    /// The derivative at `value`.
    pub derivative: T::Tangent,
}

/// A value together with its first and second derivatives.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct WithTwoDerivatives<T>
where
    T: HasTangent,
{
    /// The underlying value.
    pub value: T,

    /// The derivative at `value`.
    pub derivative: T::Tangent,

    /// The second derivative at `value`.
    pub second_derivative: <T::Tangent as HasTangent>::Tangent,
}

impl<V: VectorSpace> HasTangent for V {
    type Tangent = V;
}

impl<M, N> HasTangent for (M, N)
where
    M: HasTangent,
    N: HasTangent,
{
    type Tangent = Sum<M::Tangent, N::Tangent>;
}
