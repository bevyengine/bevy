use glam::{Vec2, Vec3, Vec3A};

use std::{
    fmt::Debug,
    iter::Sum,
    ops::{Add, Mul, Sub},
};

/// A point in space of any dimension that supports addition and multiplication.
pub trait Point:
    Mul<f32, Output = Self>
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Sum
    + Default
    + Debug
    + Clone
    + PartialEq
    + Copy
{
}
impl Point for Vec3 {} // 3D
impl Point for Vec3A {} // 3D
impl Point for Vec2 {} // 2D
impl Point for f32 {} // 1D

/// A cubic Bezier curve in 2D space
pub type CubicBezier2d = Bezier<Vec2, 4>;
/// A cubic Bezier curve in 3D space
pub type CubicBezier3d = Bezier<Vec3A, 4>;
/// A quadratic Bezier curve in 2D space
pub type QuadraticBezier2d = Bezier<Vec2, 3>;
/// A quadratic Bezier curve in 3D space
pub type QuadraticBezier3d = Bezier<Vec3A, 3>;

/// A Bezier curve with `N` control points, and dimension defined by `P`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bezier<P: Point, const N: usize>(pub [P; N]);

impl<P: Point, const N: usize> Default for Bezier<P, N> {
    fn default() -> Self {
        Bezier([P::default(); N])
    }
}

impl<P: Point, const N: usize> Bezier<P, N> {
    /// Construct a new Bezier curve
    pub fn new(control_points: [P; N]) -> Self {
        Self(control_points)
    }

    /// Compute the [`Vec3`] position along the Bezier curve at the supplied parametric value `t`.
    pub fn position(&self, t: f32) -> P {
        bezier_impl::position(self.0, t)
    }

    /// Compute the first derivative B'(t) of this cubic bezier at `t` with respect to t. This is
    /// the instantaneous velocity of a point tracing the Bezier curve from t = 0 to 1.
    pub fn velocity(&self, t: f32) -> P {
        bezier_impl::velocity(self.0, t)
    }

    /// Compute the second derivative B''(t) of this cubic bezier at `t` with respect to t.This is
    /// the instantaneous acceleration of a point tracing the Bezier curve from t = 0 to 1.
    pub fn acceleration(&self, t: f32) -> P {
        bezier_impl::acceleration(self.0, t)
    }

    /// Split the cubic Bezier curve of degree `N-1` into `subdivisions` evenly spaced `t` values
    /// across the length of the curve from t = `0..=1`, and sample with the supplied
    /// `sample_function`.
    #[inline]
    pub fn sample(&self, subdivisions: i32, sample_function: fn(&Self, f32) -> P) -> Vec<P> {
        (0..=subdivisions)
            .map(|i| {
                let t = i as f32 / subdivisions as f32;
                sample_function(self, t)
            })
            .collect()
    }

    /// Split the Bezier curve into `subdivisions` evenly spaced `t` values across the length of the
    /// curve from t = `0..=1`. sampling the position at each step.
    pub fn to_positions(&self, subdivisions: i32) -> Vec<P> {
        self.sample(subdivisions, Self::position)
    }

    /// Split the Bezier curve into `subdivisions` evenly spaced `t` values across the length of the
    /// curve from t = `0..=1`. sampling the velocity at each step.
    pub fn to_velocities(&self, subdivisions: i32) -> Vec<P> {
        self.sample(subdivisions, Self::velocity)
    }

    /// Split the Bezier curve into `subdivisions` evenly spaced `t` values across the length of the
    /// curve from t = `0..=1` . sampling the acceleration at each step.
    pub fn to_accelerations(&self, subdivisions: i32) -> Vec<P> {
        self.sample(subdivisions, Self::acceleration)
    }
}

/// A 2-dimensional Bezier curve used for easing in animation.
///
/// A cubic Bezier easing curve has control point `p0` at (0, 0) and `p3` at (1, 1), leaving only
/// `p1` and `p2` as the remaining degrees of freedom. The first and last control points are fixed
/// to ensure the animation begins at 0, and ends at 1.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct CubicBezierEasing {
    /// Control point P1 of the 2D cubic Bezier curve. Controls the start of the animation.
    p1: Vec2,
    /// Control point P2 of the 2D cubic Bezier curve. Controls the end of the animation.
    p2: Vec2,
}

impl CubicBezierEasing {
    /// Construct a cubic bezier curve for animation easing, with control points `p1` and `p2`.
    pub fn new(p1: Vec2, p2: Vec2) -> Self {
        Self { p1, p2 }
    }

    /// Maximum allowable error for iterative bezier solve
    const MAX_ERROR: f32 = 1e-7;

    /// Maximum number of iterations during bezier solve
    const MAX_ITERS: u8 = 8;

    /// Given a `time` within `0..=1`, remaps to a new value using the cubic Bezier curve as a
    /// shaping function, for which when plotted `x = time` and `y = animation progress`. This will
    /// return `0` when `t = 0`, and `1` when `t = 1`.
    pub fn ease(&self, time: f32) -> f32 {
        let x = time.clamp(0.0, 1.0);
        let t = self.find_t_given_x(x);
        self.evaluate_y_at(t)
    }

    /// Compute the x-coordinate of the point along the Bezier curve at `t`.
    #[inline]
    pub fn evaluate_x_at(&self, t: f32) -> f32 {
        bezier_impl::position([0.0, self.p1.x, self.p2.x, 1.0], t)
    }

    /// Compute the y-coordinate of the point along the Bezier curve at `t`.
    #[inline]
    pub fn evaluate_y_at(&self, t: f32) -> f32 {
        bezier_impl::position([0.0, self.p1.y, self.p2.y, 1.0], t)
    }

    /// Compute the slope of the line at the given parametric value `t` with respect to t.
    #[inline]
    pub fn dx_dt(&self, t: f32) -> f32 {
        bezier_impl::velocity([0.0, self.p1.x, self.p2.x, 1.0], t)
    }

    /// Solve for the parametric value `t` that corresponds to the given value of `x` using the
    /// Newton-Raphson method.
    #[inline]
    pub fn find_t_given_x(&self, x: f32) -> f32 {
        let mut t_guess = x;
        (0..Self::MAX_ITERS).any(|_| {
            let x_guess = self.evaluate_x_at(t_guess);
            let error = x_guess - x;
            if error.abs() <= Self::MAX_ERROR {
                true
            } else {
                // Using Newton's method, use the tangent line to estimate a better guess value.
                let slope = self.dx_dt(t_guess);
                t_guess -= error / slope;
                false
            }
        });
        t_guess.clamp(0.0, 1.0)
    }
}

/// Generic implementations for sampling cubic Bezier curves. Consider using the methods on
/// [`Bezier`] for more ergonomic use.
pub mod bezier_impl {
    use super::Point;

    /// Compute the Bernstein basis polynomial for iteration `i`, for a Bezier curve with with
    /// degree `n`, at `t`.
    #[inline]
    pub fn bernstein_basis(n: usize, i: usize, t: f32) -> f32 {
        (1. - t).powi((n - i) as i32) * t.powi(i as i32)
    }

    /// Efficiently compute the binomial coefficient
    #[inline]
    const fn binomial_coeff(n: usize, k: usize) -> usize {
        let mut i = 0;
        let mut result = 1;
        let k = match k > n - k {
            true => n - k,
            false => k,
        };
        while i < k {
            result *= n - i;
            result /= i + 1;
            i += 1;
        }
        result
    }

    /// Evaluate the Bezier curve B(t) of degree `N-1` at the parametric value `t`.
    #[inline]
    pub fn position<P: Point, const N: usize>(control_points: [P; N], t: f32) -> P {
        let p = control_points;
        let degree = N - 1;
        (0..=degree)
            .map(|i| p[i] * binomial_coeff(degree, i) as f32 * bernstein_basis(degree, i, t))
            .sum()
    }

    /// Compute the first derivative B'(t) of Bezier curve B(t) of degree `N-1` at the given
    /// parametric value `t` with respect to t.
    #[inline]
    pub fn velocity<P: Point, const N: usize>(control_points: [P; N], t: f32) -> P {
        if N <= 1 {
            return P::default(); // Zero for numeric types
        }

        let p = control_points;
        let degree = N - 1;
        let degree_vel = N - 2; // the velocity Bezier is one degree lower than the position Bezier
        (0..=degree_vel)
            .map(|i| {
                // Point on the velocity Bezier curve:
                let p = (p[i + 1] - p[i]) * degree as f32;
                p * binomial_coeff(degree_vel, i) as f32 * bernstein_basis(degree_vel, i, t)
            })
            .sum()
    }

    /// Compute the second derivative B''(t) of Bezier curve B(t) of degree `N-1` at the given
    /// parametric value `t` with respect to t.
    #[inline]
    pub fn acceleration<P: Point, const N: usize>(control_points: [P; N], t: f32) -> P {
        if N <= 2 {
            return P::default(); // Zero for numeric types
        }
        let p = control_points;
        let degree = N - 1;
        let degree_vel = N - 2; // the velocity Bezier is one degree lower than the position Bezier
        let degree_accel = N - 3; // the accel Bezier is one degree lower than the velocity Bezier
        (0..degree_vel)
            .map(|i| {
                // Points on the velocity Bezier curve:
                let p0 = (p[i + 1] - p[i]) * degree as f32;
                let p1 = (p[i + 2] - p[i + 1]) * degree as f32;
                // Point on the acceleration Bezier curve:
                let p = (p1 - p0) * (degree_vel) as f32;
                p * binomial_coeff(degree_accel, i) as f32 * bernstein_basis(degree_accel, i, t)
            })
            .sum()
    }
}
