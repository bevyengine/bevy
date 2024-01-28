//! Provides types for building cubic splines for rendering curves and use with animation easing.

use std::{
    fmt::Debug,
    iter::Sum,
    ops::{Add, Mul, Sub},
};

use bevy_utils::{thiserror, thiserror::Error};
use glam::Vec2;

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

impl<T> Point for T where
    T: Mul<f32, Output = Self>
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
            .map(|p| CubicSegment::coefficients(*p, char_matrix))
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
/// let hermite = CubicHermite::new(points, tangents).to_curve();
/// let positions: Vec<_> = hermite.iter_positions(100).collect();
/// ```
pub struct CubicHermite<P: Point> {
    control_points: Vec<(P, P)>,
}
impl<P: Point> CubicHermite<P> {
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
impl<P: Point> CubicGenerator<P> for CubicHermite<P> {
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
                CubicSegment::coefficients([p0, v0, p1, v1], char_matrix)
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
/// let cardinal = CubicCardinalSpline::new(0.3, points).to_curve();
/// let positions: Vec<_> = cardinal.iter_positions(100).collect();
/// ```
pub struct CubicCardinalSpline<P: Point> {
    tension: f32,
    control_points: Vec<P>,
}

impl<P: Point> CubicCardinalSpline<P> {
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
impl<P: Point> CubicGenerator<P> for CubicCardinalSpline<P> {
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
            .map(|p| CubicSegment::coefficients([p[0], p[1], p[2], p[3]], char_matrix))
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
/// let b_spline = CubicBSpline::new(points).to_curve();
/// let positions: Vec<_> = b_spline.iter_positions(100).collect();
/// ```
pub struct CubicBSpline<P: Point> {
    control_points: Vec<P>,
}
impl<P: Point> CubicBSpline<P> {
    /// Build a new B-Spline.
    pub fn new(control_points: impl Into<Vec<P>>) -> Self {
        Self {
            control_points: control_points.into(),
        }
    }
}
impl<P: Point> CubicGenerator<P> for CubicBSpline<P> {
    #[inline]
    fn to_curve(&self) -> CubicCurve<P> {
        let char_matrix = [
            [1.0 / 6.0, 4.0 / 6.0, 1.0 / 6.0, 0.0 / 6.0],
            [-3.0 / 6.0, 0.0 / 6.0, 3.0 / 6.0, 0.0 / 6.0],
            [3.0 / 6.0, -6.0 / 6.0, 3.0 / 6.0, 0.0 / 6.0],
            [-1.0 / 6.0, 3.0 / 6.0, -3.0 / 6.0, 1.0 / 6.0],
        ];

        let segments = self
            .control_points
            .windows(4)
            .map(|p| CubicSegment::coefficients([p[0], p[1], p[2], p[3]], char_matrix))
            .collect();

        CubicCurve { segments }
    }
}

/// Error during construction of [`CubicNurbs`]
#[derive(Debug, Error)]
pub enum CubicNurbsError {
    /// Provided knot vector had an invalid length.
    #[error("Invalid knot vector length: expected {expected}, provided {provided}")]
    InvalidKnotVectorLength {
        /// Expected knot vector length
        expected: usize,
        /// Provided knot vector length
        provided: usize,
    },
    /// Knot vector has invalid values. Values of a knot vector must be nondescending, meaning the
    /// next element must be greater than or equal to the previous one.
    #[error("Invalid knot vector values: elements are not nondescending")]
    InvalidKnotVectorValues,
    /// Provided weights vector didn't have the same amount of values as the control points vector.
    #[error("Invalid weights vector length: expected {expected}, provided {provided}")]
    WeightsVectorMismatch {
        /// Expected weights vector length
        expected: usize,
        /// Provided weights vector length
        provided: usize,
    },
    /// The amount of control points provided is less than 4.
    #[error("Not enough control points, at least 4 are required, {provided} were provided")]
    NotEnoughControlPoints {
        /// The amount of control points provided
        provided: usize,
    },
}

/// A cubic non-uniform rational B-spline (NURBS). Generates a smooth curve from a
/// sequence of control points by interpolating between four points at a time.
///
/// ### Interpolation
/// The knot vector is a non-decreasing sequence that controls which four
/// control points are assigned to each segment of the curve. It can be used to make
/// sharp corners. The curve will not pass through the control points unless the
/// knot vector has the same value four times in a row.
///
/// ### Curvature
/// The tangents automatically calculated based on the position of the control
/// points. The curve is C2 continuous (meaning both the velocity and
/// acceleration are smooth), making it useful for camera paths and moving objects.
/// The continuity reduces if the curve's knot vector has repeating values, which is called knot
/// multiplicity. Knot multiplicity of 2 would reduce the continuity to C1, multiplicity of 3 would
/// reduce the continuity to C0.
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
/// let weights = [1.0, 1.0, 2.0, 1.0];
/// let knot_vector = [0.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 5.0];
/// let nurbs = CubicNurbs::new(points, Some(weights), Some(knot_vector))
///     .expect("NURBS construction failed!")
///     .to_curve();
/// let positions: Vec<_> = nurbs.iter_positions(100).collect();
/// ```
pub struct CubicNurbs<P: Point> {
    control_points: Vec<P>,
    knot_vector: Vec<f32>,
}
impl<P: Point> CubicNurbs<P> {
    /// Build a Non-Uniform Rational B-Spline.
    ///
    /// If provided, weights vector must have the same amount of items as the control points
    /// vector. Defaults to equal weights.
    ///
    /// If provided, the knot vector must have n + 4 elements, where n is the amount of control
    /// points. Defaults to open uniform knot vector: [`Self::open_uniform_knot_vector`].
    ///
    /// At least 4 points must be provided, otherwise an error will be returned.
    pub fn new(
        control_points: impl Into<Vec<P>>,
        weights: Option<impl Into<Vec<f32>>>,
        knot_vector: Option<impl Into<Vec<f32>>>,
    ) -> Result<Self, CubicNurbsError> {
        let mut control_points: Vec<P> = control_points.into();
        let control_points_len = control_points.len();

        if control_points_len < 4 {
            return Err(CubicNurbsError::NotEnoughControlPoints {
                provided: control_points_len,
            });
        }

        let mut weights = weights
            .map(Into::into)
            .unwrap_or_else(|| vec![1.0; control_points_len]);

        let knot_vector: Vec<f32> = knot_vector.map(Into::into).unwrap_or_else(|| {
            Self::open_uniform_knot_vector(control_points_len)
                .expect("The amount of control points was checked")
        });

        let knot_vector_expected_length = Self::knot_vector_length(control_points_len);

        // Check the knot vector length
        if knot_vector.len() != knot_vector_expected_length {
            return Err(CubicNurbsError::InvalidKnotVectorLength {
                expected: knot_vector_expected_length,
                provided: knot_vector.len(),
            });
        }

        // Check the knot vector for being nondescending (previous elements is less than or equal
        // to the next)
        if knot_vector.windows(2).any(|win| win[0] > win[1]) {
            return Err(CubicNurbsError::InvalidKnotVectorValues);
        }

        // Check the weights vector length
        if weights.len() != control_points_len {
            return Err(CubicNurbsError::WeightsVectorMismatch {
                expected: control_points_len,
                provided: weights.len(),
            });
        }

        weights = Self::normalize_weights(weights);

        control_points
            .iter_mut()
            .zip(weights)
            .for_each(|(p, w)| *p = *p * w);

        Ok(Self {
            control_points,
            knot_vector,
        })
    }

    /// Generates a uniform knot vector that will generate the same curve as [`CubicBSpline`].
    ///
    /// "Uniform" means that the difference between two knot values next to each other is the same
    /// through the entire knot vector.
    ///
    /// Will return `None` if there are less than 4 control points
    pub fn uniform_knot_vector(control_points: usize) -> Option<Vec<f32>> {
        if control_points < 4 {
            return None;
        }
        Some(
            (0..Self::knot_vector_length(control_points))
                .map(|v| v as f32)
                .collect(),
        )
    }

    /// Generates an open uniform knot vector, which makes the ends of the curve pass through the
    /// start and end points.
    ///
    /// The knot vector will have a knot with multiplicity of 4 at the end and start and elements
    /// in the middle will have a difference of 1. "Multiplicity" means that there are N
    /// consecutive elements that have the same value.
    ///
    /// Will return `None` if there are less than 4 control points
    pub fn open_uniform_knot_vector(control_points: usize) -> Option<Vec<f32>> {
        if control_points < 4 {
            return None;
        }
        let last_knots_value = control_points - 3;
        Some(
            std::iter::repeat(0.0)
                .take(4)
                .chain((1..last_knots_value).map(|v| v as f32))
                .chain(std::iter::repeat(last_knots_value as f32).take(4))
                .collect(),
        )
    }

    #[inline(always)]
    const fn knot_vector_length(control_points_len: usize) -> usize {
        control_points_len + 4
    }

    /// Based on <https://xiaoxingchen.github.io/2020/03/02/bspline_in_so3/general_matrix_representation_for_bsplines.pdf>
    fn generate_matrix(knot_vector_segment: &[f32; 8]) -> [[f32; 4]; 4] {
        let t = knot_vector_segment;
        let m00 = (t[4] - t[3]).powi(2) / ((t[4] - t[2]) * (t[4] - t[1]));
        let m02 = (t[3] - t[2]).powi(2) / ((t[5] - t[2]) * (t[4] - t[2]));
        let m12 = (3.0 * (t[4] - t[3]) * (t[3] - t[2])) / ((t[5] - t[2]) * (t[4] - t[2]));
        let m22 = 3.0 * (t[4] - t[3]).powi(2) / ((t[5] - t[2]) * (t[4] - t[2]));
        let m33 = (t[4] - t[3]).powi(2) / ((t[6] - t[3]) * (t[5] - t[3]));
        let m32 = -m22 / 3.0 - m33 - (t[4] - t[3]).powi(2) / ((t[5] - t[3]) * (t[5] - t[2]));
        [
            [m00, 1.0 - m00 - m02, m02, 0.0],
            [-3.0 * m00, 3.0 * m00 - m12, m12, 0.0],
            [3.0 * m00, -3.0 * m00 - m22, m22, 0.0],
            [-m00, m00 - m32 - m33, m32, m33],
        ]
    }

    /// Normalizes weights vector using L0 norm.
    /// The resulting weight vector's values will add up to be equal the amount of values in the
    /// weights vector
    fn normalize_weights(weights: Vec<f32>) -> Vec<f32> {
        let g = weights.len() as f32;
        let weights_sum: f32 = weights.iter().sum();
        let mul = g / weights_sum;
        weights.into_iter().map(|w| w * mul).collect()
    }
}
impl<P: Point> CubicGenerator<P> for CubicNurbs<P> {
    #[inline]
    fn to_curve(&self) -> CubicCurve<P> {
        let segments = self
            .control_points
            .windows(4)
            .zip(self.knot_vector.windows(8))
            .map(|(points, knot_vector_segment)| {
                let knot_vector_segment = knot_vector_segment
                    .try_into()
                    .expect("Knot vector windows are of length 8");
                let matrix = Self::generate_matrix(knot_vector_segment);
                CubicSegment::coefficients(
                    points
                        .try_into()
                        .expect("Points vector windows are of length 4"),
                    matrix,
                )
            })
            .collect();
        CubicCurve { segments }
    }
}

/// A spline interpolated linearly across nearest 2 points.
pub struct LinearSpline<P: Point> {
    points: Vec<P>,
}
impl<P: Point> LinearSpline<P> {
    /// Create a new linear spline
    pub fn new(points: impl Into<Vec<P>>) -> Self {
        Self {
            points: points.into(),
        }
    }
}
impl<P: Point> CubicGenerator<P> for LinearSpline<P> {
    #[inline]
    fn to_curve(&self) -> CubicCurve<P> {
        let segments = self
            .points
            .windows(2)
            .map(|points| {
                let a = points[0];
                let b = points[1];
                CubicSegment {
                    coeff: [a, b - a, P::default(), P::default()],
                }
            })
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

    #[inline]
    fn coefficients(p: [P; 4], char_matrix: [[f32; 4]; 4]) -> Self {
        let [c0, c1, c2, c3] = char_matrix;
        // These are the polynomial coefficients, computed by multiplying the characteristic
        // matrix by the point matrix.
        let coeff = [
            p[0] * c0[0] + p[1] * c0[1] + p[2] * c0[2] + p[3] * c0[3],
            p[0] * c1[0] + p[1] * c1[1] + p[2] * c1[2] + p[3] * c1[3],
            p[0] * c2[0] + p[1] * c2[1] + p[2] * c2[2] + p[3] * c2[3],
            p[0] * c3[0] + p[1] * c3[1] + p[2] * c3[2] + p[3] * c3[3],
        ];
        Self { coeff }
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
///
/// Use any struct that implements the [`CubicGenerator`] trait to create a new curve, such as
/// [`CubicBezier`].
#[derive(Clone, Debug, PartialEq)]
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

    #[inline]
    /// Adds a segment to the curve
    pub fn push_segment(&mut self, segment: CubicSegment<P>) {
        self.segments.push(segment);
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
}

impl<P: Point> Extend<CubicSegment<P>> for CubicCurve<P> {
    fn extend<T: IntoIterator<Item = CubicSegment<P>>>(&mut self, iter: T) {
        self.segments.extend(iter);
    }
}

impl<P: Point> IntoIterator for CubicCurve<P> {
    type IntoIter = <Vec<CubicSegment<P>> as IntoIterator>::IntoIter;

    type Item = CubicSegment<P>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
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
