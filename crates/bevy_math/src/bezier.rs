use std::ops::{Add, Mul};

use glam::{Vec2, Vec3};

/// Provides methods for sampling Cubic Bezier curves.
pub trait CubicBezier {
    /// Represents a coordinate in space, this can be 3d, 2d, or even 1d.
    type Coord: Copy + Mul<f32, Output = Self::Coord> + Add<Self::Coord, Output = Self::Coord>;

    /// Returns the four control points of the cubic Bezier curve.
    fn control_points(&self) -> [Self::Coord; 4];

    /// Returns the point along the Bezier curve at the supplied parametric value `t`.
    fn evaluate_at(&self, t: f32) -> Self::Coord {
        let p = self.control_points();
        p[0] * (1. - t).powi(3)
            + p[1] * t * 3.0 * (1.0 - t).powi(2)
            + p[2] * 3.0 * (1.0 - t) * t.powi(2)
            + p[3] * t.powi(3)
    }

    /// Iterate over points in the bezier curve from `t = 0` to `t = 1`.
    fn into_points(&self, subdivisions: i32) -> Vec<Self::Coord> {
        (0..=subdivisions)
            .map(|i| {
                let t = i as f32 / subdivisions as f32;
                self.evaluate_at(t)
            })
            .collect()
    }
}

/// A 2-dimensional Bezier curve.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier2d(pub [Vec2; 4]);

impl CubicBezier for CubicBezier2d {
    type Coord = Vec2;

    fn control_points(&self) -> [Self::Coord; 4] {
        self.0
    }
}

/// A 3-dimensional Bezier curve.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier3d(pub [Vec3; 4]);

impl CubicBezier for CubicBezier3d {
    type Coord = Vec3;

    fn control_points(&self) -> [Self::Coord; 4] {
        self.0
    }
}

pub mod easing {
    use super::CubicBezier;

    /// Used to optimize cubic bezier easing, which only does operations in one dimension at a time,
    /// and whose first and last control points are constrained to 0 and 1 respectively.
    #[derive(Default, Clone, Copy, Debug, PartialEq)]
    struct CubicBezier1d(f32, f32);

    impl CubicBezier for CubicBezier1d {
        type Coord = f32;

        fn control_points(&self) -> [Self::Coord; 4] {
            [0.0, self.0, self.1, 1.0]
        }
    }

    /// A 2-dimensional Bezier curve used for easing in animation; the first and last control points
    /// are constrained to (0, 0) and (1, 1) respectively.
    #[derive(Default, Clone, Copy, Debug, PartialEq)]
    pub struct CubicBezierEasing {
        x: CubicBezier1d,
        y: CubicBezier1d,
    }

    impl CubicBezierEasing {
        /// Construct a cubic bezier curve for animation easing, with control points P1 and P2.
        ///
        /// This is equivalent to the syntax used to define cubic bezier easing functions in, say, CSS:
        /// `ease-in-out = cubic-bezier(0.42, 0.0, 0.58, 1.0)`.
        pub fn new(p1_x: f32, p1_y: f32, p2_x: f32, p2_y: f32) -> Self {
            Self {
                x: CubicBezier1d(p1_x, p2_x),
                y: CubicBezier1d(p1_y, p2_y),
            }
        }

        /// Maximum allowable error for iterative bezier solve
        const MAX_ERROR: f32 = 1e-7;

        /// Maximum number of iterations during bezier solve
        const MAX_ITERS: u8 = 8;

        /// Given a `time` within `0..=1`, remaps this using a shaping function to a new value. The
        pub fn remap(&self, time: f32) -> f32 {
            let x = time.clamp(0.0, 1.0);
            let t = self.find_t_given_x(x);
            self.y.evaluate_at(t)
        }

        /// Compute the slope `dx/dt` of a cubic bezier easing curve at the given parametric `t`.
        pub fn dx_dt(&self, t: f32) -> f32 {
            let p0x = 0.0;
            let p1x = self.x.0;
            let p2x = self.x.1;
            let p3x = 1.0;
            3. * (1. - t).powi(2) * (p1x - p0x)
                + 6. * (1. - t) * t * (p2x - p1x)
                + 3. * t.powi(2) * (p3x - p2x)
        }

        /// Solve for the parametric value `t` corresponding to the given value of `x`.
        pub fn find_t_given_x(&self, x: f32) -> f32 {
            // We will use the desired value x as our initial guess for t. This is a good estimate,
            // as cubic bezier curves for animation are usually near the line where x = t.
            let mut t_guess = x;
            let mut error = f32::MAX;
            for _ in 0..Self::MAX_ITERS {
                let x_guess = self.x.evaluate_at(t_guess);
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
}
