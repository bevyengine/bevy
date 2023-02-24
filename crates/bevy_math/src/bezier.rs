use glam::{Vec2, Vec3, Vec3A};

use std::{
    fmt::Debug,
    iter::Sum,
    ops::{Add, Mul, Sub},
};

/// A point in space of any dimension that supports the mathematical operations needed by
/// [`Bezier`].
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

/// A generic Bezier curve with `N` control points, and dimension defined by `P`.
///
/// Consider the following type aliases for most common uses:
/// - [`CubicBezier2d`]
/// - [`CubicBezier3d`]
/// - [`QuadraticBezier2d`]
/// - [`QuadraticBezier3d`]
///
/// The Bezier degree is equal to `N - 1`. For example, a cubic Bezier has 4 control points, and a
/// degree of 3. The time-complexity of evaluating a Bezier increases superlinearly with the number
/// of control points. As such, it is recommended to instead use a chain of quadratic or cubic
/// `Beziers` instead of a high-degree Bezier.
///
/// ### About Bezier curves
///
/// `Bezier` curves are parametric implicit functions; all that means is they take a parameter `t`,
/// and output a point in space, like:
///
/// > B(t) = (x, y, z)
///
/// So, all that is needed to find a point in space along a Bezier curve is the parameter `t`.
/// Additionally, the values of `t` are straightforward: `t` is 0 at the start of the curve (first
/// control point) and 1 at the end (last control point).
///
/// ```
/// # use bevy_math::{Bezier, Vec2, vec2};
/// let p0 = vec2(0.25, 0.1);
/// let p1 = vec2(0.25, 1.0);
/// let bezier = Bezier::<Vec2, 2>::new([p0, p1]);
/// assert_eq!(bezier.position(0.0), p0);
/// assert_eq!(bezier.position(1.0), p1);
/// ```
///
/// ### Plotting
///
/// To plot a Bezier curve, simply plug in a series of values of `t` from zero to one. The functions
/// to do this are [`Bezier::position`] to sample the curve at a value of `t`, and
/// [`Bezier::iter_positions`] to iterate over the curve with a number of subdivisions.
///
/// ### Velocity and Acceleration
///
/// In addition to the position of a point on the Bezier curve, it is also useful to get information
/// about the curvature. Methods are provided to help with this:
///
/// - [`Bezier::velocity`]: the instantaneous velocity vector with respect to `t`. This is a vector
///       that points in the direction a point is traveling when it is at point `t`. This vector is
///       tangent to the curve.
/// - [`Bezier::acceleration`]: the instantaneous acceleration vector with respect to `t`. This is a
///       vector that points in the direction a point is accelerating towards when it is at point
///       `t`. This vector will point to the inside of turns, the direction the point is being
///       pulled toward to change direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bezier<P: Point, const N: usize>(pub [P; N]);

impl<P: Point, const N: usize> Default for Bezier<P, N> {
    fn default() -> Self {
        Bezier([P::default(); N])
    }
}

impl<P: Point, const N: usize> Bezier<P, N> {
    /// Construct a new Bezier curve.
    pub fn new(control_points: [impl Into<P>; N]) -> Self {
        let control_points = control_points.map(|v| v.into());
        Self(control_points)
    }

    /// Compute the position of a point along the Bezier curve at the supplied parametric value `t`.
    pub fn position(&self, t: f32) -> P {
        generic::position(self.0, t)
    }

    /// Compute the first derivative B'(t) of this Bezier at `t` with respect to t. This is the
    /// instantaneous velocity of a point on the Bezier curve at `t`.
    pub fn velocity(&self, t: f32) -> P {
        generic::velocity(self.0, t)
    }

    /// Compute the second derivative B''(t) of this Bezier at `t` with respect to t. This is the
    /// instantaneous acceleration of a point on the Bezier curve at `t`.
    pub fn acceleration(&self, t: f32) -> P {
        generic::acceleration(self.0, t)
    }

    /// A flexible iterator used to sample [`Bezier`] curves with arbitrary functions.
    ///
    /// This splits the Bezier into `subdivisions` of evenly spaced `t` values across the length of
    /// the curve from start (t = 0) to end (t = 1), returning an iterator that evaluates the curve
    /// with the supplied `sample_function` at each `t`.
    ///
    /// Given `subdivisions = 2`, this will split the curve into two lines, or three points, and
    /// return an iterator over those three points, one at the start, middle, and end.
    #[inline]
    pub fn iter_samples(
        &self,
        subdivisions: usize,
        sample_function: fn(&Self, f32) -> P,
    ) -> impl Iterator<Item = P> + '_ {
        (0..=subdivisions).map(move |i| {
            let t = i as f32 / subdivisions as f32;
            sample_function(self, t)
        })
    }

    /// Iterate over the curve split into `subdivisions`, sampling the position at each step.
    pub fn iter_positions(&self, subdivisions: usize) -> impl Iterator<Item = P> + '_ {
        self.iter_samples(subdivisions, Self::position)
    }

    /// Iterate over the curve split into `subdivisions`, sampling the velocity at each step.
    pub fn iter_velocities(&self, subdivisions: usize) -> impl Iterator<Item = P> + '_ {
        self.iter_samples(subdivisions, Self::velocity)
    }

    /// Iterate over the curve split into `subdivisions`, sampling the acceleration at each step.
    pub fn iter_accelerations(&self, subdivisions: usize) -> impl Iterator<Item = P> + '_ {
        self.iter_samples(subdivisions, Self::acceleration)
    }
}

impl<T: Into<P>, P: Point, const N: usize> From<[T; N]> for Bezier<P, N> {
    fn from(control_points: [T; N]) -> Self {
        Bezier::new(control_points)
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
    pub p1: Vec2,
    /// Control point P2 of the 2D cubic Bezier curve. Controls the end of the animation.
    pub p2: Vec2,
}

impl CubicBezierEasing {
    /// Construct a cubic Bezier curve for animation easing, with control points `p1` and `p2`.
    /// These correspond to the two free "handles" of the Bezier curve.
    ///
    /// This is a very common tool for animations that accelerate and decelerate smoothly. For
    /// example, the ubiquitous "ease-in-out" is defined as `(0.25, 0.1), (0.25, 1.0)`.
    pub fn new(p1: impl Into<Vec2>, p2: impl Into<Vec2>) -> Self {
        Self {
            p1: p1.into(),
            p2: p2.into(),
        }
    }

    /// Maximum allowable error for iterative Bezier solve
    const MAX_ERROR: f32 = 1e-5;

    /// Maximum number of iterations during Bezier solve
    const MAX_ITERS: u8 = 8;

    /// Given a `time` within `0..=1`, returns an eased value that follows the cubic Bezier curve
    /// instead of a straight line. This eased result may be outside the range `0..=1`, however it
    /// will always start at 0 and end at 1: `ease(0) = 0` and `ease(1) = 1`.
    ///
    /// ```
    /// # use bevy_math::CubicBezierEasing;
    /// let cubic_bezier = CubicBezierEasing::new((0.25, 0.1), (0.25, 1.0));
    /// assert_eq!(cubic_bezier.ease(0.0), 0.0);
    /// assert_eq!(cubic_bezier.ease(1.0), 1.0);
    /// ```
    ///
    /// # How cubic Bezier easing works
    ///
    /// Easing is generally accomplished with the help of "shaping functions". These are curves that
    /// start at (0,0) and end at (1,1). The x-axis of this plot is the current `time` of the
    /// animation, from 0 to 1. The y-axis is how far along the animation is, also from 0 to 1. You
    /// can imagine that if the shaping function is a straight line, there is a 1:1 mapping between
    /// the `time` and how far along your animation is. If the `time` = 0.5, the animation is
    /// halfway through. This is known as linear interpolation, and results in objects animating
    /// with a constant velocity, and no smooth acceleration or deceleration at the start or end.
    ///
    /// ```text
    /// y
    /// │         ●
    /// │       ⬈
    /// │     ⬈    
    /// │   ⬈
    /// │ ⬈
    /// ●─────────── x (time)
    /// ```
    ///
    /// Using cubic Beziers, we have a curve that starts at (0,0), ends at (1,1), and follows a path
    /// determined by the two remaining control points (handles). These handles allow us to define a
    /// smooth curve. As `time` (x-axis) progresses, we now follow the curve, and use the `y` value
    /// to determine how far along the animation is.
    ///
    /// ```text
    /// y
    ///          ⬈➔●
    /// │      ⬈   
    /// │     ↑      
    /// │     ↑
    /// │    ⬈
    /// ●➔⬈───────── x (time)
    /// ```
    ///
    /// To accomplish this, we need to be able to find the position `y` on a Bezier curve, given the
    /// `x` value. As discussed in the [`Bezier`] documentation, a Bezier curve is an implicit
    /// parametric function like B(t) = (x,y). To find `y`, we first solve for `t` that corresponds
    /// to the given `x` (`time`). We use the Newton-Raphson root-finding method to quickly find a
    /// value of `t` that matches `x`. Once we have this we can easily plug that `t` into our
    /// Bezier's `position` function, to find the `y` component, which is how far along our
    /// animation should be. In other words:
    ///
    /// > Given `time` in `0..=1`
    ///
    /// > Use Newton's method to find a value of `t` that results in B(t) = (x,y) where `x == time`
    ///
    /// > Once a solution is found, use the resulting `y` value as the final result
    ///
    /// # Performance
    ///
    /// This operation can be used frequently without fear of performance issues. Benchmarks show
    /// this operation taking on the order of 50 nanoseconds.
    pub fn ease(&self, time: f32) -> f32 {
        let x = time.clamp(0.0, 1.0);
        let t = self.find_t_given_x(x);
        self.evaluate_y_at(t)
    }

    /// Compute the x-coordinate of the point along the Bezier curve at `t`.
    #[inline]
    fn evaluate_x_at(&self, t: f32) -> f32 {
        generic::position([0.0, self.p1.x, self.p2.x, 1.0], t)
    }

    /// Compute the y-coordinate of the point along the Bezier curve at `t`.
    #[inline]
    fn evaluate_y_at(&self, t: f32) -> f32 {
        generic::position([0.0, self.p1.y, self.p2.y, 1.0], t)
    }

    /// Compute the slope of the line at the given parametric value `t` with respect to t.
    #[inline]
    fn dx_dt(&self, t: f32) -> f32 {
        generic::velocity([0.0, self.p1.x, self.p2.x, 1.0], t)
    }

    /// Solve for the parametric value `t` that corresponds to the given value of `x` using the
    /// Newton-Raphson method. See documentation on [`Self::ease`] for more details.
    #[inline]
    fn find_t_given_x(&self, x: f32) -> f32 {
        // PERFORMANCE NOTE:
        //
        // I tried pre-solving and caching values along the curve at struct instantiation to give
        // the solver a better starting guess. This ended up being slightly slower, possibly due to
        // the increased size of the type. Another option would be to store the last `t`, and use
        // that, however it's possible this could end up in a bad state where t is very far from the
        // naive but generally safe guess of x, e.g. after an animation resets.
        //
        // Further optimization might not be needed however - benchmarks are showing it takes about
        // 50 nanoseconds for an ease operation on my modern laptop, which seems sufficiently fast.
        let mut t_guess = x;
        for _ in 0..Self::MAX_ITERS {
            let x_guess = self.evaluate_x_at(t_guess);
            let error = x_guess - x;
            if error.abs() <= Self::MAX_ERROR {
                break;
            }
            // Using Newton's method, use the tangent line to estimate a better guess value.
            let slope = self.dx_dt(t_guess);
            t_guess -= error / slope;
        }
        t_guess.clamp(0.0, 1.0)
    }
}

impl<P: Into<Vec2>> From<[P; 2]> for CubicBezierEasing {
    fn from(points: [P; 2]) -> Self {
        let [p0, p1] = points;
        CubicBezierEasing::new(p0, p1)
    }
}

/// Generic implementations for sampling Bezier curves. Consider using the methods on
/// [`Bezier`](crate::Bezier) for more ergonomic use.
pub mod generic {
    use super::Point;

    /// Compute the Bernstein basis polynomial `i` of degree `n`, at `t`.
    ///
    /// For more information, see <https://en.wikipedia.org/wiki/Bernstein_polynomial>.
    #[inline]
    pub fn bernstein_basis(n: usize, i: usize, t: f32) -> f32 {
        (1. - t).powi((n - i) as i32) * t.powi(i as i32)
    }

    /// Efficiently compute the binomial coefficient of `n` choose `k`.
    #[inline]
    fn binomial_coeff(n: usize, k: usize) -> usize {
        let k = usize::min(k, n - k);
        (0..k).fold(1, |val, i| val * (n - i) / (i + 1))
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
    /// parametric value `t` with respect to t. Note that the first derivative of a Bezier is also a
    /// Bezier, of degree `N-2`.
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
    /// parametric value `t` with respect to t. Note that the second derivative of a Bezier is also
    /// a Bezier, of degree `N-3`.
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

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::{CubicBezier2d, CubicBezierEasing};

    /// How close two floats can be and still be considered equal
    const FLOAT_EQ: f32 = 1e-5;

    /// Basic cubic Bezier easing test to verify the shape of the curve.
    #[test]
    fn easing_simple() {
        // A curve similar to ease-in-out, but symmetric
        let bezier = CubicBezierEasing::new([1.0, 0.0], [0.0, 1.0]);
        assert_eq!(bezier.ease(0.0), 0.0);
        assert!(bezier.ease(0.2) < 0.2); // tests curve
        assert_eq!(bezier.ease(0.5), 0.5); // true due to symmetry
        assert!(bezier.ease(0.8) > 0.8); // tests curve
        assert_eq!(bezier.ease(1.0), 1.0);
    }

    /// A curve that forms an upside-down "U", that should extend below 0.0. Useful for animations
    /// that go beyond the start and end positions, e.g. bouncing.
    #[test]
    fn easing_overshoot() {
        // A curve that forms an upside-down "U", that should extend above 1.0
        let bezier = CubicBezierEasing::new([0.0, 2.0], [1.0, 2.0]);
        assert_eq!(bezier.ease(0.0), 0.0);
        assert!(bezier.ease(0.5) > 1.5);
        assert_eq!(bezier.ease(1.0), 1.0);
    }

    /// A curve that forms a "U", that should extend below 0.0. Useful for animations that go beyond
    /// the start and end positions, e.g. bouncing.
    #[test]
    fn easing_undershoot() {
        let bezier = CubicBezierEasing::new([0.0, -2.0], [1.0, -2.0]);
        assert_eq!(bezier.ease(0.0), 0.0);
        assert!(bezier.ease(0.5) < -0.5);
        assert_eq!(bezier.ease(1.0), 1.0);
    }

    /// Sweep along the full length of a 3D cubic Bezier, and manually check the position.
    #[test]
    fn cubic() {
        const N_SAMPLES: usize = 1000;
        let bezier = CubicBezier2d::new([[-1.0, -20.0], [3.0, 2.0], [5.0, 3.0], [9.0, 8.0]]);
        assert_eq!(bezier.position(0.0), bezier.0[0]); // 0 == Start
        assert_eq!(bezier.position(1.0), bezier.0[3]); // 1 == End
        for i in 0..=N_SAMPLES {
            let t = i as f32 / N_SAMPLES as f32; // Check along entire length
            assert!(bezier.position(t).distance(cubic_manual(t, bezier)) <= FLOAT_EQ);
        }
    }

    /// Manual, hardcoded function for computing the position along a cubic bezier.
    fn cubic_manual(t: f32, bezier: CubicBezier2d) -> Vec2 {
        let [p0, p1, p2, p3] = bezier.0;
        p0 * (1.0 - t).powi(3)
            + 3.0 * p1 * t * (1.0 - t).powi(2)
            + 3.0 * p2 * t.powi(2) * (1.0 - t)
            + p3 * t.powi(3)
    }
}
