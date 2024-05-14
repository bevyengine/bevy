use glam::{Vec2, Vec3, Vec3A, Vec4};
use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Neg, Sub};

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
pub trait VectorSpace:
    Mul<f32, Output = Self>
    + Div<f32, Output = Self>
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Neg
    + Default
    + Debug
    + Clone
    + Copy
{
    /// The zero vector, which is the identity of addition for the vector space type.
    const ZERO: Self;

    /// Perform vector space linear interpolation between this element and another, based
    /// on the parameter `t`. When `t` is `0`, `self` is recovered. When `t` is `1`, `rhs`
    /// is recovered.
    ///
    /// Note that the value of `t` is not clamped by this function, so interpolating outside
    /// of the interval `[0,1]` is allowed.
    #[inline]
    fn lerp(&self, rhs: Self, t: f32) -> Self {
        *self * (1. - t) + rhs * t
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

/// A type that supports the operations of a normed vector space; i.e. a norm operation in addition
/// to those of [`VectorSpace`]. Specifically, the implementor must guarantee that the following
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
        self.abs()
    }

    #[inline]
    fn norm_squared(self) -> f32 {
        self * self
    }
}
