//! This module contains abstract mathematical traits shared by types used in `bevy_math`.

use crate::{ops, DVec2, DVec3, DVec4, Dir2, Dir3, Dir3A, Quat, Rot2, Vec2, Vec3, Vec3A, Vec4};
use core::{
    fmt::Debug,
    ops::{Add, Div, Mul, Neg, Sub},
};
use variadics_please::all_tuples_enumerated;

/// A type that supports the mathematical operations of a real vector space, irrespective of dimension.
/// In particular, this means that the implementing type supports:
/// - Scalar multiplication and division on the right by elements of `Self::Scalar`
/// - Negation
/// - Addition and subtraction
/// - Zero
///
/// Within the limitations of floating point arithmetic, all the following are required to hold:
/// - (Associativity of addition) For all `u, v, w: Self`, `(u + v) + w == u + (v + w)`.
/// - (Commutativity of addition) For all `u, v: Self`, `u + v == v + u`.
/// - (Additive identity) For all `v: Self`, `v + Self::ZERO == v`.
/// - (Additive inverse) For all `v: Self`, `v - v == v + (-v) == Self::ZERO`.
/// - (Compatibility of multiplication) For all `a, b: Self::Scalar`, `v: Self`, `v * (a * b) == (v * a) * b`.
/// - (Multiplicative identity) For all `v: Self`, `v * 1.0 == v`.
/// - (Distributivity for vector addition) For all `a: Self::Scalar`, `u, v: Self`, `(u + v) * a == u * a + v * a`.
/// - (Distributivity for scalar addition) For all `a, b: Self::Scalar`, `v: Self`, `v * (a + b) == v * a + v * b`.
///
/// Note that, because implementing types use floating point arithmetic, they are not required to actually
/// implement `PartialEq` or `Eq`.
pub trait VectorSpace:
    Mul<Self::Scalar, Output = Self>
    + Div<Self::Scalar, Output = Self>
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Neg<Output = Self>
    + Default
    + Debug
    + Clone
    + Copy
{
    /// The scalar type of this vector space.
    type Scalar: ScalarField;

    /// The zero vector, which is the identity of addition for the vector space type.
    const ZERO: Self;

    /// Perform vector space linear interpolation between this element and another, based
    /// on the parameter `t`. When `t` is `0`, `self` is recovered. When `t` is `1`, `rhs`
    /// is recovered.
    ///
    /// Note that the value of `t` is not clamped by this function, so extrapolating outside
    /// of the interval `[0,1]` is allowed.
    #[inline]
    fn lerp(self, rhs: Self, t: Self::Scalar) -> Self {
        self * (Self::Scalar::ONE - t) + rhs * t
    }
}

impl VectorSpace for Vec4 {
    type Scalar = f32;
    const ZERO: Self = Vec4::ZERO;
}

impl VectorSpace for Vec3 {
    type Scalar = f32;
    const ZERO: Self = Vec3::ZERO;
}

impl VectorSpace for Vec3A {
    type Scalar = f32;
    const ZERO: Self = Vec3A::ZERO;
}

impl VectorSpace for Vec2 {
    type Scalar = f32;
    const ZERO: Self = Vec2::ZERO;
}

impl VectorSpace for DVec4 {
    type Scalar = f64;
    const ZERO: Self = DVec4::ZERO;
}

impl VectorSpace for DVec3 {
    type Scalar = f64;
    const ZERO: Self = DVec3::ZERO;
}

impl VectorSpace for DVec2 {
    type Scalar = f64;
    const ZERO: Self = DVec2::ZERO;
}

// Every scalar field is a 1-dimensional vector space over itself.
impl<T: ScalarField> VectorSpace for T {
    type Scalar = Self;
    const ZERO: Self = Self::ZERO;
}

/// A type that supports the operations of a scalar field. An implementation should support:
/// - Addition and subtraction
/// - Multiplication and division
/// - Negation
/// - Zero (additive identity)
/// - One (multiplicative identity)
///
/// Within the limitations of floating point arithmetic, all the following are required to hold:
/// - (Associativity of addition) For all `u, v, w: Self`, `(u + v) + w == u + (v + w)`.
/// - (Commutativity of addition) For all `u, v: Self`, `u + v == v + u`.
/// - (Additive identity) For all `v: Self`, `v + Self::ZERO == v`.
/// - (Additive inverse) For all `v: Self`, `v - v == v + (-v) == Self::ZERO`.
/// - (Associativity of multiplication) For all `u, v, w: Self`, `(u * v) * w == u * (v * w)`.
/// - (Commutativity of multiplication) For all `u, v: Self`, `u * v == v * u`.
/// - (Multiplicative identity) For all `v: Self`, `v * Self::ONE == v`.
/// - (Multiplicative inverse) For all `v: Self`, `v / v == v * v.inverse() == Self::ONE`.
/// - (Distributivity over addition) For all `a, b: Self`, `u, v: Self`, `(u + v) * a == u * a + v * a`.
pub trait ScalarField:
    Mul<Self, Output = Self>
    + Div<Self, Output = Self>
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Neg<Output = Self>
    + Default
    + Debug
    + Clone
    + Copy
{
    /// The additive identity.
    const ZERO: Self;
    /// The multiplicative identity.
    const ONE: Self;

    /// The multiplicative inverse of this element. This is equivalent to `1.0 / self`.
    fn recip(self) -> Self {
        Self::ONE / self
    }
}

impl ScalarField for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
}

impl ScalarField for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
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

impl<F: ScalarField, V, W> Mul<F> for Sum<V, W>
where
    V: VectorSpace<Scalar = F>,
    W: VectorSpace<Scalar = F>,
{
    type Output = Self;
    fn mul(self, rhs: F) -> Self::Output {
        Sum(self.0 * rhs, self.1 * rhs)
    }
}

impl<F: ScalarField, V, W> Div<F> for Sum<V, W>
where
    V: VectorSpace<Scalar = F>,
    W: VectorSpace<Scalar = F>,
{
    type Output = Self;
    fn div(self, rhs: F) -> Self::Output {
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

impl<F: ScalarField, V, W> VectorSpace for Sum<V, W>
where
    V: VectorSpace<Scalar = F>,
    W: VectorSpace<Scalar = F>,
{
    type Scalar = F;
    const ZERO: Self = Sum(V::ZERO, W::ZERO);
}

/// A type that supports the operations of a normed vector space; i.e. a norm operation in addition
/// to those of [`VectorSpace`]. Specifically, the implementor must guarantee that the following
/// relationships hold, within the limitations of floating point arithmetic:
/// - (Nonnegativity) For all `v: Self`, `v.norm() >= 0.0`.
/// - (Positive definiteness) For all `v: Self`, `v.norm() == 0.0` implies `v == Self::ZERO`.
/// - (Absolute homogeneity) For all `c: Self::Scalar`, `v: Self`, `(v * c).norm() == v.norm() * c.abs()`.
/// - (Triangle inequality) For all `v, w: Self`, `(v + w).norm() <= v.norm() + w.norm()`.
///
/// Note that, because implementing types use floating point arithmetic, they are not required to actually
/// implement `PartialEq` or `Eq`.
pub trait NormedVectorSpace: VectorSpace {
    /// The size of this element. The return value should always be nonnegative.
    fn norm(self) -> Self::Scalar;

    /// The squared norm of this element. Computing this is often faster than computing
    /// [`NormedVectorSpace::norm`].
    #[inline]
    fn norm_squared(self) -> Self::Scalar {
        self.norm() * self.norm()
    }

    /// The distance between this element and another, as determined by the norm.
    #[inline]
    fn distance(self, rhs: Self) -> Self::Scalar {
        (rhs - self).norm()
    }

    /// The squared distance between this element and another, as determined by the norm. Note that
    /// this is often faster to compute in practice than [`NormedVectorSpace::distance`].
    #[inline]
    fn distance_squared(self, rhs: Self) -> Self::Scalar {
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
}

impl NormedVectorSpace for DVec4 {
    #[inline]
    fn norm(self) -> f64 {
        self.length()
    }

    #[inline]
    fn norm_squared(self) -> f64 {
        self.length_squared()
    }
}

impl NormedVectorSpace for DVec3 {
    #[inline]
    fn norm(self) -> f64 {
        self.length()
    }

    #[inline]
    fn norm_squared(self) -> f64 {
        self.length_squared()
    }
}

impl NormedVectorSpace for DVec2 {
    #[inline]
    fn norm(self) -> f64 {
        self.length()
    }

    #[inline]
    fn norm_squared(self) -> f64 {
        self.length_squared()
    }
}

impl NormedVectorSpace for f64 {
    #[inline]
    #[cfg(feature = "std")]
    fn norm(self) -> f64 {
        f64::abs(self)
    }

    #[inline]
    #[cfg(all(any(feature = "libm", feature = "nostd-libm"), not(feature = "std")))]
    fn norm(self) -> f64 {
        libm::fabs(self)
    }
}

/// A type with a natural interpolation that provides strong subdivision guarantees.
///
/// Although the only required method is `interpolate_stable`, many things are expected of it:
///
/// 1. The notion of interpolation should follow naturally from the semantics of the type, so
///    that inferring the interpolation mode from the type alone is sensible.
///
/// 2. The interpolation recovers something equivalent to the starting value at `t = 0.0`
///    and likewise with the ending value at `t = 1.0`. They do not have to be data-identical, but
///    they should be semantically identical. For example, [`Quat::slerp`] doesn't always yield its
///    second rotation input exactly at `t = 1.0`, but it always returns an equivalent rotation.
///
/// 3. Importantly, the interpolation must be *subdivision-stable*: for any interpolation curve
///    between two (unnamed) values and any parameter-value pairs `(t0, p)` and `(t1, q)`, the
///    interpolation curve between `p` and `q` must be the *linear* reparameterization of the original
///    interpolation curve restricted to the interval `[t0, t1]`.
///
/// The last of these conditions is very strong and indicates something like constant speed. It
/// is called "subdivision stability" because it guarantees that breaking up the interpolation
/// into segments and joining them back together has no effect.
///
/// Here is a diagram depicting it:
/// ```text
/// top curve = u.interpolate_stable(v, t)
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
/// bottom curve = p.interpolate_stable(q, s)
/// ```
///
/// Note that some common forms of interpolation do not satisfy this criterion. For example,
/// [`Quat::lerp`] and [`Rot2::nlerp`] are not subdivision-stable.
///
/// Furthermore, this is not to be used as a general trait for abstract interpolation.
/// Consumers rely on the strong guarantees in order for behavior based on this trait to be
/// well-behaved.
///
/// [`Quat::slerp`]: crate::Quat::slerp
/// [`Quat::lerp`]: crate::Quat::lerp
/// [`Rot2::nlerp`]: crate::Rot2::nlerp
pub trait StableInterpolate: Clone {
    /// Interpolate between this value and the `other` given value using the parameter `t`. At
    /// `t = 0.0`, a value equivalent to `self` is recovered, while `t = 1.0` recovers a value
    /// equivalent to `other`, with intermediate values interpolating between the two.
    /// See the [trait-level documentation] for details.
    ///
    /// [trait-level documentation]: StableInterpolate
    fn interpolate_stable(&self, other: &Self, t: f32) -> Self;

    /// A version of [`interpolate_stable`] that assigns the result to `self` for convenience.
    ///
    /// [`interpolate_stable`]: StableInterpolate::interpolate_stable
    fn interpolate_stable_assign(&mut self, other: &Self, t: f32) {
        *self = self.interpolate_stable(other, t);
    }

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
    /// # use bevy_math::{Vec3, StableInterpolate};
    /// # let delta_time: f32 = 1.0 / 60.0;
    /// let mut object_position: Vec3 = Vec3::ZERO;
    /// let target_position: Vec3 = Vec3::new(2.0, 3.0, 5.0);
    /// // Decay rate of ln(10) => after 1 second, remaining distance is 1/10th
    /// let decay_rate = f32::ln(10.0);
    /// // Calling this repeatedly will move `object_position` towards `target_position`:
    /// object_position.smooth_nudge(&target_position, decay_rate, delta_time);
    /// ```
    fn smooth_nudge(&mut self, target: &Self, decay_rate: f32, delta: f32) {
        self.interpolate_stable_assign(target, 1.0 - ops::exp(-decay_rate * delta));
    }
}

// Conservatively, we presently only apply this for normed vector spaces, where the notion
// of being constant-speed is literally true. The technical axioms are satisfied for any
// VectorSpace type, but the "natural from the semantics" part is less clear in general.
impl<V> StableInterpolate for V
where
    V: NormedVectorSpace<Scalar = f32>,
{
    #[inline]
    fn interpolate_stable(&self, other: &Self, t: f32) -> Self {
        self.lerp(*other, t)
    }
}

impl StableInterpolate for Rot2 {
    #[inline]
    fn interpolate_stable(&self, other: &Self, t: f32) -> Self {
        self.slerp(*other, t)
    }
}

impl StableInterpolate for Quat {
    #[inline]
    fn interpolate_stable(&self, other: &Self, t: f32) -> Self {
        self.slerp(*other, t)
    }
}

impl StableInterpolate for Dir2 {
    #[inline]
    fn interpolate_stable(&self, other: &Self, t: f32) -> Self {
        self.slerp(*other, t)
    }
}

impl StableInterpolate for Dir3 {
    #[inline]
    fn interpolate_stable(&self, other: &Self, t: f32) -> Self {
        self.slerp(*other, t)
    }
}

impl StableInterpolate for Dir3A {
    #[inline]
    fn interpolate_stable(&self, other: &Self, t: f32) -> Self {
        self.slerp(*other, t)
    }
}

macro_rules! impl_stable_interpolate_tuple {
    ($(#[$meta:meta])* $(($n:tt, $T:ident)),*) => {
        $(#[$meta])*
        impl<$($T: StableInterpolate),*> StableInterpolate for ($($T,)*) {
            fn interpolate_stable(&self, other: &Self, t: f32) -> Self {
                (
                    $(
                        <$T as StableInterpolate>::interpolate_stable(&self.$n, &other.$n, t),
                    )*
                )
            }
        }
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

impl<F, U, V, M, N> HasTangent for (M, N)
where
    F: ScalarField,
    U: VectorSpace<Scalar = F>,
    V: VectorSpace<Scalar = F>,
    M: HasTangent<Tangent = U>,
    N: HasTangent<Tangent = V>,
{
    type Tangent = Sum<M::Tangent, N::Tangent>;
}
