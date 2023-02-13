use glam::{Vec2, Vec3};

/// A 2-dimensional Bezier curve, defined by its 4 [`Vec2`] control points.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier2d(pub [Vec2; 4]);

impl CubicBezier2d {
    /// Returns the [`Vec2`] position along the Bezier curve at the supplied parametric value `t`.
    pub fn evaluate_at(&self, t: f32) -> Vec2 {
        bezier_impl::evaluate_cubic_bezier(self.0, t)
    }

    /// Split the Bezier curve into `subdivisions` across the length of the curve from t = `0..=1`.
    /// evaluating the [`Vec2`] position at each step.
    pub fn to_points(&self, subdivisions: i32) -> Vec<Vec2> {
        bezier_impl::cubic_bezier_to_points(self.0, subdivisions)
    }
}

/// A 3-dimensional Bezier curve, defined by its 4 [`Vec3`] control points.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier3d(pub [Vec3; 4]);

impl CubicBezier3d {
    /// Returns the [`Vec3`] position along the Bezier curve at the supplied parametric value `t`.
    pub fn evaluate_at(&self, t: f32) -> Vec3 {
        bezier_impl::evaluate_cubic_bezier(self.0, t)
    }

    /// Split the Bezier curve into `subdivisions` across the length of the curve from t = `0..=1`.
    /// evaluating the [`Vec3`] position at each step.
    pub fn to_points(&self, subdivisions: i32) -> Vec<Vec3> {
        bezier_impl::cubic_bezier_to_points(self.0, subdivisions)
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
    /// Construct a cubic bezier curve for animation easing, with control points `p1` and `p2`.
    pub fn new(p1: Vec2, p2: Vec2) -> Self {
        Self { p1, p2 }
    }

    /// Maximum allowable error for iterative bezier solve
    const MAX_ERROR: f32 = 1e-7;

    /// Maximum number of iterations during bezier solve
    const MAX_ITERS: u8 = 8;

    /// Returns the x-coordinate of the point along the Bezier curve at the supplied parametric
    /// value `t`.
    pub fn evaluate_x_at(&self, t: f32) -> f32 {
        bezier_impl::evaluate_cubic_bezier([0.0, self.p1.x, self.p2.x, 1.0], t)
    }

    /// Returns the y-coordinate of the point along the Bezier curve at the supplied parametric
    /// value `t`.
    pub fn evaluate_y_at(&self, t: f32) -> f32 {
        bezier_impl::evaluate_cubic_bezier([0.0, self.p1.y, self.p2.y, 1.0], t)
    }

    /// Given a `time` within `0..=1`, remaps to a new value using the cubic Bezier curve as a
    /// shaping function, where `x = time`, and `y = animation progress`. This will return `0` when
    /// `t = 0`, and `1` when `t = 1`.
    pub fn remap(&self, time: f32) -> f32 {
        let x = time.clamp(0.0, 1.0);
        let t = self.find_t_given_x(x);
        self.evaluate_y_at(t)
    }

    /// Compute the slope `dx/dt` of a cubic bezier easing curve at the given parametric `t`.
    pub fn dx_dt(&self, t: f32) -> f32 {
        let p0x = 0.0;
        let p1x = self.p1.x;
        let p2x = self.p2.x;
        let p3x = 1.0;
        3. * (1. - t).powi(2) * (p1x - p0x)
            + 6. * (1. - t) * t * (p2x - p1x)
            + 3. * t.powi(2) * (p3x - p2x)
    }

    /// Solve for the parametric value `t` corresponding to the given value of `x`.
    ///
    /// This will return `x` if the solve fails to converge, which corresponds to a simple linear
    /// interpolation.
    pub fn find_t_given_x(&self, x: f32) -> f32 {
        // We will use the desired value x as our initial guess for t. This is a good estimate,
        // as cubic bezier curves for animation are usually near the line where x = t.
        let mut t_guess = x;
        let mut error = f32::MAX;
        for _ in 0..Self::MAX_ITERS {
            let x_guess = self.evaluate_x_at(t_guess);
            error = x_guess - x;
            if error.abs() <= Self::MAX_ERROR {
                return t_guess;
            }
            let slope = self.dx_dt(t_guess);
            t_guess -= error / slope;
        }
        if error.abs() <= Self::MAX_ERROR {
            t_guess
        } else {
            x // fallback to linear interpolation if the solve fails
        }
    }
}

/// The bezier implementation is wrapped inside a private module to keep the public interface
/// simple. This allows us to reuse the generic code across various cubic Bezier types, without
/// exposing users to any unwieldy traits or generics in the IDE or documentation.
#[doc(hidden)]
mod bezier_impl {
    use glam::{Vec2, Vec3};
    use std::ops::{Add, Mul};

    /// A point in space of any dimension that supports addition and multiplication.
    pub trait Point: Copy + Mul<f32, Output = Self> + Add<Self, Output = Self> {}
    impl Point for Vec3 {}
    impl Point for Vec2 {}
    impl Point for f32 {}

    /// Evaluate the cubic Bezier curve at the parametric value `t`.
    pub fn evaluate_cubic_bezier<P: Point>(control_points: [P; 4], t: f32) -> P {
        let p = control_points;
        p[0] * (1. - t).powi(3)
            + p[1] * t * 3.0 * (1.0 - t).powi(2)
            + p[2] * 3.0 * (1.0 - t) * t.powi(2)
            + p[3] * t.powi(3)
    }

    /// Split the Bezier curve into `subdivisions`, and sample the position at each [`Point`] `P`.
    pub fn cubic_bezier_to_points<P: Point>(control_points: [P; 4], subdivisions: i32) -> Vec<P> {
        (0..=subdivisions)
            .map(|i| {
                let t = i as f32 / subdivisions as f32;
                evaluate_cubic_bezier(control_points, t)
            })
            .collect()
    }
}
