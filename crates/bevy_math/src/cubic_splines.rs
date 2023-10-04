//! Provides types for building cubic splines for rendering curves and use with animation easing.

use glam::{Vec2, Vec3, Vec3A};

use std::{
    fmt::Debug,
    iter::Sum,
    ops::{Add, Mul, Sub},
};

/// A point in space of any dimension that supports the math ops needed for cubic spline
/// interpolation.
pub trait Point:
    Mul<f32, Output = Self>
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Add<f32, Output = Self>
    + Sum
    + Default
    + Debug
    + Clone
    + PartialEq
    + Copy
{
}
impl Point for Vec3 {}
impl Point for Vec3A {}
impl Point for Vec2 {}
impl Point for f32 {}

/// A spline composed of a single cubic Bezier curve.
///
/// Useful for user-drawn curves with local control, or animation easing. See
/// [`CubicSegment::new_bezier`] for use in easing.
///
/// ### Interpolation
/// The curve only passes through the first and last control point in each set of four points.
///
/// ### Tangency
/// Manually defined by the two intermediate control points within each set of four points.
///
/// ### Continuity
/// At minimum C0 continuous, up to C2. Continuity greater than C0 can result in a loss of local
/// control over the spline due to the curvature constraints.
///
/// ### Usage
///
/// ```
/// # use bevy_math::{*, prelude::*};
/// let points = [[
///     vec2(-1.0, -20.0),
///     vec2(3.0, 2.0),
///     vec2(5.0, 3.0),
///     vec2(9.0, 8.0),
/// ]];
/// let bezier = CubicBezier::new(points).to_curve();
/// let positions: Vec<_> = bezier.iter_positions(100).collect();
/// ```
pub struct CubicBezier<P: Point> {
    control_points: Vec<[P; 4]>,
}

impl<P: Point> CubicBezier<P> {
    /// Create a new cubic Bezier curve from sets of control points.
    pub fn new(control_points: impl Into<Vec<[P; 4]>>) -> Self {
        Self {
            control_points: control_points.into(),
        }
    }
}
impl<P: Point> CubicGenerator<P> for CubicBezier<P> {
    #[inline]
    fn to_curve(&self) -> CubicCurve<P> {
        let char_matrix = [
            [1., 0., 0., 0.],
            [-3., 3., 0., 0.],
            [3., -6., 3., 0.],
            [-1., 3., -3., 1.],
        ];

        let segments = self
            .control_points
            .iter()
            .map(|p| CubicCurve::coefficients(*p, 1.0, char_matrix))
            .collect();

        CubicCurve { segments }
    }
}

/// A spline interpolated continuously between the nearest two control points, with the position and
/// velocity of the curve specified at both control points. This curve passes through all control
/// points, with the specified velocity which includes direction and parametric speed.
///
/// Useful for smooth interpolation when you know the position and velocity at two points in time,
/// such as network prediction.
///
/// ### Interpolation
/// The curve passes through every control point.
///
/// ### Tangency
/// Explicitly defined at each control point.
///
/// ### Continuity
/// At minimum C0 continuous, up to C1.
///
/// ### Usage
///
/// ```
/// # use bevy_math::{*, prelude::*};
/// let points = [
///     vec2(-1.0, -20.0),
///     vec2(3.0, 2.0),
///     vec2(5.0, 3.0),
///     vec2(9.0, 8.0),
/// ];
/// let tangents = [
///     vec2(0.0, 1.0),
///     vec2(0.0, 1.0),
///     vec2(0.0, 1.0),
///     vec2(0.0, 1.0),
/// ];
/// let hermite = Hermite::new(points, tangents).to_curve();
/// let positions: Vec<_> = hermite.iter_positions(100).collect();
/// ```
pub struct Hermite<P: Point> {
    control_points: Vec<(P, P)>,
}
impl<P: Point> Hermite<P> {
    /// Create a new Hermite curve from sets of control points.
    pub fn new(
        control_points: impl IntoIterator<Item = P>,
        tangents: impl IntoIterator<Item = P>,
    ) -> Self {
        Self {
            control_points: control_points.into_iter().zip(tangents).collect(),
        }
    }
}
impl<P: Point> CubicGenerator<P> for Hermite<P> {
    #[inline]
    fn to_curve(&self) -> CubicCurve<P> {
        let char_matrix = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [-3., -2., 3., -1.],
            [2., 1., -2., 1.],
        ];

        let segments = self
            .control_points
            .windows(2)
            .map(|p| {
                let (p0, v0, p1, v1) = (p[0].0, p[0].1, p[1].0, p[1].1);
                CubicCurve::coefficients([p0, v0, p1, v1], 1.0, char_matrix)
            })
            .collect();

        CubicCurve { segments }
    }
}

/// A spline interpolated continuously across the nearest four control points, with the position of
/// the curve specified at every control point and the tangents computed automatically.
///
/// **Note** the Catmull-Rom spline is a special case of Cardinal spline where the tension is 0.5.
///
/// ### Interpolation
/// The curve passes through every control point.
///
/// ### Tangency
/// Automatically defined at each control point.
///
/// ### Continuity
/// C1 continuous.
///
/// ### Usage
///
/// ```
/// # use bevy_math::{*, prelude::*};
/// let points = [
///     vec2(-1.0, -20.0),
///     vec2(3.0, 2.0),
///     vec2(5.0, 3.0),
///     vec2(9.0, 8.0),
/// ];
/// let cardinal = CardinalSpline::new(0.3, points).to_curve();
/// let positions: Vec<_> = cardinal.iter_positions(100).collect();
/// ```
pub struct CardinalSpline<P: Point> {
    tension: f32,
    control_points: Vec<P>,
}

impl<P: Point> CardinalSpline<P> {
    /// Build a new Cardinal spline.
    pub fn new(tension: f32, control_points: impl Into<Vec<P>>) -> Self {
        Self {
            tension,
            control_points: control_points.into(),
        }
    }

    /// Build a new Catmull-Rom spline, the special case of a Cardinal spline where tension = 1/2.
    pub fn new_catmull_rom(control_points: impl Into<Vec<P>>) -> Self {
        Self {
            tension: 0.5,
            control_points: control_points.into(),
        }
    }
}
impl<P: Point> CubicGenerator<P> for CardinalSpline<P> {
    #[inline]
    fn to_curve(&self) -> CubicCurve<P> {
        let s = self.tension;
        let char_matrix = [
            [0., 1., 0., 0.],
            [-s, 0., s, 0.],
            [2. * s, s - 3., 3. - 2. * s, -s],
            [-s, 2. - s, s - 2., s],
        ];

        let segments = self
            .control_points
            .windows(4)
            .map(|p| CubicCurve::coefficients([p[0], p[1], p[2], p[3]], 1.0, char_matrix))
            .collect();

        CubicCurve { segments }
    }
}

/// A spline interpolated continuously across the nearest four control points. The curve does not
/// pass through any of the control points.
///
/// ### Interpolation
/// The curve does not pass through control points.
///
/// ### Tangency
/// Automatically computed based on the position of control points.
///
/// ### Continuity
/// C2 continuous! The acceleration continuity of this spline makes it useful for camera paths.
///
/// ### Usage
///
/// ```
/// # use bevy_math::{*, prelude::*};
/// let points = [
///     vec2(-1.0, -20.0),
///     vec2(3.0, 2.0),
///     vec2(5.0, 3.0),
///     vec2(9.0, 8.0),
/// ];
/// let b_spline = BSpline::new(points).to_curve();
/// let positions: Vec<_> = b_spline.iter_positions(100).collect();
/// ```
pub struct BSpline<P: Point> {
    control_points: Vec<P>,
}
impl<P: Point> BSpline<P> {
    /// Build a new Cardinal spline.
    pub fn new(control_points: impl Into<Vec<P>>) -> Self {
        Self {
            control_points: control_points.into(),
        }
    }
}
impl<P: Point> CubicGenerator<P> for BSpline<P> {
    #[inline]
    fn to_curve(&self) -> CubicCurve<P> {
        let char_matrix = [
            [1., 4., 1., 0.],
            [-3., 0., 3., 0.],
            [3., -6., 3., 0.],
            [-1., 3., -3., 1.],
        ];

        let segments = self
            .control_points
            .windows(4)
            .map(|p| CubicCurve::coefficients([p[0], p[1], p[2], p[3]], 1.0 / 6.0, char_matrix))
            .collect();

        CubicCurve { segments }
    }
}

/// Implement this on cubic splines that can generate a curve from their spline parameters.
pub trait CubicGenerator<P: Point> {
    /// Build a [`CubicCurve`] by computing the interpolation coefficients for each curve segment.
    fn to_curve(&self) -> CubicCurve<P>;
}

/// A segment of a cubic curve, used to hold precomputed coefficients for fast interpolation.
///
/// Segments can be chained together to form a longer compound curve.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CubicSegment<P: Point> {
    coeff: [P; 4],
}

impl<P: Point> CubicSegment<P> {
    /// Instantaneous position of a point at parametric value `t`.
    #[inline]
    pub fn position(&self, t: f32) -> P {
        let [a, b, c, d] = self.coeff;
        a + b * t + c * t.powi(2) + d * t.powi(3)
    }

    /// Instantaneous velocity of a point at parametric value `t`.
    #[inline]
    pub fn velocity(&self, t: f32) -> P {
        let [_, b, c, d] = self.coeff;
        b + c * 2.0 * t + d * 3.0 * t.powi(2)
    }

    /// Instantaneous acceleration of a point at parametric value `t`.
    #[inline]
    pub fn acceleration(&self, t: f32) -> P {
        let [_, _, c, d] = self.coeff;
        c * 2.0 + d * 6.0 * t
    }
}

/// The `CubicSegment<Vec2>` can be used as a 2-dimensional easing curve for animation.
///
/// The x-axis of the curve is time, and the y-axis is the output value. This struct provides
/// methods for extremely fast solves for y given x.
impl CubicSegment<Vec2> {
    /// Construct a cubic Bezier curve for animation easing, with control points `p1` and `p2`. A
    /// cubic Bezier easing curve has control point `p0` at (0, 0) and `p3` at (1, 1), leaving only
    /// `p1` and `p2` as the remaining degrees of freedom. The first and last control points are
    /// fixed to ensure the animation begins at 0, and ends at 1.
    ///
    /// This is a very common tool for UI animations that accelerate and decelerate smoothly. For
    /// example, the ubiquitous "ease-in-out" is defined as `(0.25, 0.1), (0.25, 1.0)`.
    pub fn new_bezier(p1: impl Into<Vec2>, p2: impl Into<Vec2>) -> Self {
        let (p0, p3) = (Vec2::ZERO, Vec2::ONE);
        let bezier = CubicBezier::new([[p0, p1.into(), p2.into(), p3]]).to_curve();
        bezier.segments[0].clone()
    }

    /// Maximum allowable error for iterative Bezier solve
    const MAX_ERROR: f32 = 1e-5;

    /// Maximum number of iterations during Bezier solve
    const MAX_ITERS: u8 = 8;

    /// Given a `time` within `0..=1`, returns an eased value that follows the cubic curve instead
    /// of a straight line. This eased result may be outside the range `0..=1`, however it will
    /// always start at 0 and end at 1: `ease(0) = 0` and `ease(1) = 1`.
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// let cubic_bezier = CubicSegment::new_bezier((0.25, 0.1), (0.25, 1.0));
    /// assert_eq!(cubic_bezier.ease(0.0), 0.0);
    /// assert_eq!(cubic_bezier.ease(1.0), 1.0);
    /// ```
    ///
    /// # How cubic easing works
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
    /// To accomplish this, we need to be able to find the position `y` on a curve, given the `x`
    /// value. Cubic curves are implicit parametric functions like B(t) = (x,y). To find `y`, we
    /// first solve for `t` that corresponds to the given `x` (`time`). We use the Newton-Raphson
    /// root-finding method to quickly find a value of `t` that is very near the desired value of
    /// `x`. Once we have this we can easily plug that `t` into our curve's `position` function, to
    /// find the `y` component, which is how far along our animation should be. In other words:
    ///
    /// > Given `time` in `0..=1`
    ///
    /// > Use Newton's method to find a value of `t` that results in B(t) = (x,y) where `x == time`
    ///
    /// > Once a solution is found, use the resulting `y` value as the final result
    #[inline]
    pub fn ease(&self, time: f32) -> f32 {
        let x = time.clamp(0.0, 1.0);
        self.find_y_given_x(x)
    }

    /// Find the `y` value of the curve at the given `x` value using the Newton-Raphson method.
    #[inline]
    fn find_y_given_x(&self, x: f32) -> f32 {
        let mut t_guess = x;
        let mut pos_guess = Vec2::ZERO;
        for _ in 0..Self::MAX_ITERS {
            pos_guess = self.position(t_guess);
            let error = pos_guess.x - x;
            if error.abs() <= Self::MAX_ERROR {
                break;
            }
            // Using Newton's method, use the tangent line to estimate a better guess value.
            let slope = self.velocity(t_guess).x; // dx/dt
            t_guess -= error / slope;
        }
        pos_guess.y
    }
}

/// A collection of [`CubicSegment`]s chained into a curve.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CubicCurve<P: Point> {
    segments: Vec<CubicSegment<P>>,
}

impl<P: Point> CubicCurve<P> {
    /// Compute the position of a point on the cubic curve at the parametric value `t`.
    ///
    /// Note that `t` varies from `0..=(n_points - 3)`.
    #[inline]
    pub fn position(&self, t: f32) -> P {
        let (segment, t) = self.segment(t);
        segment.position(t)
    }

    /// Compute the first derivative with respect to t at `t`. This is the instantaneous velocity of
    /// a point on the cubic curve at `t`.
    ///
    /// Note that `t` varies from `0..=(n_points - 3)`.
    #[inline]
    pub fn velocity(&self, t: f32) -> P {
        let (segment, t) = self.segment(t);
        segment.velocity(t)
    }

    /// Compute the second derivative with respect to t at `t`. This is the instantaneous
    /// acceleration of a point on the cubic curve at `t`.
    ///
    /// Note that `t` varies from `0..=(n_points - 3)`.
    #[inline]
    pub fn acceleration(&self, t: f32) -> P {
        let (segment, t) = self.segment(t);
        segment.acceleration(t)
    }

    /// A flexible iterator used to sample curves with arbitrary functions.
    ///
    /// This splits the curve into `subdivisions` of evenly spaced `t` values across the
    /// length of the curve from start (t = 0) to end (t = n), where `n = self.segment_count()`,
    /// returning an iterator evaluating the curve with the supplied `sample_function` at each `t`.
    ///
    /// For `subdivisions = 2`, this will split the curve into two lines, or three points, and
    /// return an iterator with 3 items, the three points, one at the start, middle, and end.
    #[inline]
    pub fn iter_samples<'a, 'b: 'a>(
        &'b self,
        subdivisions: usize,
        mut sample_function: impl FnMut(&Self, f32) -> P + 'a,
    ) -> impl Iterator<Item = P> + 'a {
        self.iter_uniformly(subdivisions)
            .map(move |t| sample_function(self, t))
    }

    /// An iterator that returns values of `t` uniformly spaced over `0..=subdivisions`.
    #[inline]
    fn iter_uniformly(&self, subdivisions: usize) -> impl Iterator<Item = f32> {
        let segments = self.segments.len() as f32;
        let step = segments / subdivisions as f32;
        (0..=subdivisions).map(move |i| i as f32 * step)
    }

    /// The list of segments contained in this `CubicCurve`.
    ///
    /// This spline's global `t` value is equal to how many segments it has.
    ///
    /// All method accepting `t` on `CubicCurve` depends on the global `t`.
    /// When sampling over the entire curve, you should either use one of the
    /// `iter_*` methods or account for the segment count using `curve.segments().len()`.
    #[inline]
    pub fn segments(&self) -> &[CubicSegment<P>] {
        &self.segments
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

    /// Returns the [`CubicSegment`] and local `t` value given a spline's global `t` value.
    #[inline]
    fn segment(&self, t: f32) -> (&CubicSegment<P>, f32) {
        if self.segments.len() == 1 {
            (&self.segments[0], t)
        } else {
            let i = (t.floor() as usize).clamp(0, self.segments.len() - 1);
            (&self.segments[i], t - i as f32)
        }
    }

    #[inline]
    fn coefficients(p: [P; 4], multiplier: f32, char_matrix: [[f32; 4]; 4]) -> CubicSegment<P> {
        let [c0, c1, c2, c3] = char_matrix;
        // These are the polynomial coefficients, computed by multiplying the characteristic
        // matrix by the point matrix.
        let mut coeff = [
            p[0] * c0[0] + p[1] * c0[1] + p[2] * c0[2] + p[3] * c0[3],
            p[0] * c1[0] + p[1] * c1[1] + p[2] * c1[2] + p[3] * c1[3],
            p[0] * c2[0] + p[1] * c2[1] + p[2] * c2[2] + p[3] * c2[3],
            p[0] * c3[0] + p[1] * c3[1] + p[2] * c3[2] + p[3] * c3[3],
        ];
        coeff.iter_mut().for_each(|c| *c = *c * multiplier);
        CubicSegment { coeff }
    }
}

#[cfg(test)]
mod tests {
    use glam::{vec2, Vec2};

    use crate::cubic_splines::{CubicBezier, CubicGenerator, CubicSegment};

    /// How close two floats can be and still be considered equal
    const FLOAT_EQ: f32 = 1e-5;

    /// Sweep along the full length of a 3D cubic Bezier, and manually check the position.
    #[test]
    fn cubic() {
        const N_SAMPLES: usize = 1000;
        let points = [[
            vec2(-1.0, -20.0),
            vec2(3.0, 2.0),
            vec2(5.0, 3.0),
            vec2(9.0, 8.0),
        ]];
        let bezier = CubicBezier::new(points).to_curve();
        for i in 0..=N_SAMPLES {
            let t = i as f32 / N_SAMPLES as f32; // Check along entire length
            assert!(bezier.position(t).distance(cubic_manual(t, points[0])) <= FLOAT_EQ);
        }
    }

    /// Manual, hardcoded function for computing the position along a cubic bezier.
    fn cubic_manual(t: f32, points: [Vec2; 4]) -> Vec2 {
        let p = points;
        p[0] * (1.0 - t).powi(3)
            + 3.0 * p[1] * t * (1.0 - t).powi(2)
            + 3.0 * p[2] * t.powi(2) * (1.0 - t)
            + p[3] * t.powi(3)
    }

    /// Basic cubic Bezier easing test to verify the shape of the curve.
    #[test]
    fn easing_simple() {
        // A curve similar to ease-in-out, but symmetric
        let bezier = CubicSegment::new_bezier([1.0, 0.0], [0.0, 1.0]);
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
        let bezier = CubicSegment::new_bezier([0.0, 2.0], [1.0, 2.0]);
        assert_eq!(bezier.ease(0.0), 0.0);
        assert!(bezier.ease(0.5) > 1.5);
        assert_eq!(bezier.ease(1.0), 1.0);
    }

    /// A curve that forms a "U", that should extend below 0.0. Useful for animations that go beyond
    /// the start and end positions, e.g. bouncing.
    #[test]
    fn easing_undershoot() {
        let bezier = CubicSegment::new_bezier([0.0, -2.0], [1.0, -2.0]);
        assert_eq!(bezier.ease(0.0), 0.0);
        assert!(bezier.ease(0.5) < -0.5);
        assert_eq!(bezier.ease(1.0), 1.0);
    }
}
