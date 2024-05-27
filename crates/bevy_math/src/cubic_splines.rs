//! Provides types for building cubic splines for rendering curves and use with animation easing.

use std::{fmt::Debug, iter::once};

use crate::{Vec2, VectorSpace};

use thiserror::Error;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// A spline composed of a single cubic Bezier curve.
///
/// Useful for user-drawn curves with local control, or animation easing. See
/// [`CubicSegment::new_bezier`] for use in easing.
///
/// ### Interpolation
/// The curve only passes through the first and last control point in each set of four points. The curve
/// is divided into "segments" by every fourth control point.
///
/// ### Tangency
/// Tangents are manually defined by the two intermediate control points within each set of four points.
/// You can think of the control points the curve passes through as "anchors", and as the intermediate
/// control points as the anchors displaced along their tangent vectors
///
/// ### Continuity
/// A Bezier curve is at minimum C0 continuous, meaning it has no holes or jumps. Each curve segment is
/// C2, meaning the tangent vector changes smoothly between each set of four control points, but this
/// doesn't hold at the control points between segments. Making the whole curve C1 or C2 requires moving
/// the intermediate control points to align the tangent vectors between segments, and can result in a
/// loss of local control.
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
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct CubicBezier<P: VectorSpace> {
    /// The control points of the Bezier curve
    pub control_points: Vec<[P; 4]>,
}

impl<P: VectorSpace> CubicBezier<P> {
    /// Create a new cubic Bezier curve from sets of control points.
    pub fn new(control_points: impl Into<Vec<[P; 4]>>) -> Self {
        Self {
            control_points: control_points.into(),
        }
    }
}
impl<P: VectorSpace> CubicGenerator<P> for CubicBezier<P> {
    #[inline]
    fn to_curve(&self) -> CubicCurve<P> {
        // A derivation for this matrix can be found in "General Matrix Representations for B-splines" by Kaihuai Qin.
        // <https://xiaoxingchen.github.io/2020/03/02/bspline_in_so3/general_matrix_representation_for_bsplines.pdf>
        // See section 4.2 and equation 11.
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
/// Tangents are explicitly defined at each control point.
///
/// ### Continuity
/// The curve is at minimum C0 continuous, meaning it has no holes or jumps. It is also C1, meaning the
/// tangent vector has no sudden jumps.
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
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct CubicHermite<P: VectorSpace> {
    /// The control points of the Hermite curve
    pub control_points: Vec<(P, P)>,
}
impl<P: VectorSpace> CubicHermite<P> {
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
impl<P: VectorSpace> CubicGenerator<P> for CubicHermite<P> {
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
/// the curve specified at every control point and the tangents computed automatically. The associated [`CubicCurve`]
/// has one segment between each pair of adjacent control points.
///
/// **Note** the Catmull-Rom spline is a special case of Cardinal spline where the tension is 0.5.
///
/// ### Interpolation
/// The curve passes through every control point.
///
/// ### Tangency
/// Tangents are automatically computed based on the positions of control points.
///
/// ### Continuity
/// The curve is at minimum C1, meaning that it is continuous (it has no holes or jumps), and its tangent
/// vector is also well-defined everywhere, without sudden jumps.
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
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct CubicCardinalSpline<P: VectorSpace> {
    /// Tension
    pub tension: f32,
    /// The control points of the Cardinal spline
    pub control_points: Vec<P>,
}

impl<P: VectorSpace> CubicCardinalSpline<P> {
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
impl<P: VectorSpace> CubicGenerator<P> for CubicCardinalSpline<P> {
    #[inline]
    fn to_curve(&self) -> CubicCurve<P> {
        let s = self.tension;
        let char_matrix = [
            [0., 1., 0., 0.],
            [-s, 0., s, 0.],
            [2. * s, s - 3., 3. - 2. * s, -s],
            [-s, 2. - s, s - 2., s],
        ];

        let length = self.control_points.len();

        // Early return to avoid accessing an invalid index
        if length < 2 {
            return CubicCurve { segments: vec![] };
        }

        // Extend the list of control points by mirroring the last second-to-last control points on each end;
        // this allows tangents for the endpoints to be provided, and the overall effect is that the tangent
        // at an endpoint is proportional to twice the vector between it and its adjacent control point.
        //
        // The expression used here is P_{-1} := P_0 - (P_1 - P_0) = 2P_0 - P_1. (Analogously at the other end.)
        let mirrored_first = self.control_points[0] * 2. - self.control_points[1];
        let mirrored_last = self.control_points[length - 1] * 2. - self.control_points[length - 2];
        let extended_control_points = once(&mirrored_first)
            .chain(self.control_points.iter())
            .chain(once(&mirrored_last))
            .collect::<Vec<_>>();

        let segments = extended_control_points
            .windows(4)
            .map(|p| CubicSegment::coefficients([*p[0], *p[1], *p[2], *p[3]], char_matrix))
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
/// Tangents are automatically computed based on the position of control points.
///
/// ### Continuity
/// The curve is C2 continuous, meaning it has no holes or jumps, and the tangent vector changes smoothly along
/// the entire curve length. The acceleration continuity of this spline makes it useful for camera paths.
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
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct CubicBSpline<P: VectorSpace> {
    /// The control points of the spline
    pub control_points: Vec<P>,
}
impl<P: VectorSpace> CubicBSpline<P> {
    /// Build a new B-Spline.
    pub fn new(control_points: impl Into<Vec<P>>) -> Self {
        Self {
            control_points: control_points.into(),
        }
    }
}
impl<P: VectorSpace> CubicGenerator<P> for CubicBSpline<P> {
    #[inline]
    fn to_curve(&self) -> CubicCurve<P> {
        // A derivation for this matrix can be found in "General Matrix Representations for B-splines" by Kaihuai Qin.
        // <https://xiaoxingchen.github.io/2020/03/02/bspline_in_so3/general_matrix_representation_for_bsplines.pdf>
        // See section 4.1 and equations 7 and 8.
        let mut char_matrix = [
            [1.0, 4.0, 1.0, 0.0],
            [-3.0, 0.0, 3.0, 0.0],
            [3.0, -6.0, 3.0, 0.0],
            [-1.0, 3.0, -3.0, 1.0],
        ];

        char_matrix
            .iter_mut()
            .for_each(|r| r.iter_mut().for_each(|c| *c /= 6.0));

        let segments = self
            .control_points
            .windows(4)
            .map(|p| CubicSegment::coefficients([p[0], p[1], p[2], p[3]], char_matrix))
            .collect();

        CubicCurve { segments }
    }
}

/// Error during construction of [`CubicNurbs`]
#[derive(Clone, Debug, Error)]
pub enum CubicNurbsError {
    /// Provided the wrong number of knots.
    #[error("Wrong number of knots: expected {expected}, provided {provided}")]
    KnotsNumberMismatch {
        /// Expected number of knots
        expected: usize,
        /// Provided number of knots
        provided: usize,
    },
    /// The provided knots had a descending knot pair. Subsequent knots must
    /// either increase or stay the same.
    #[error("Invalid knots: contains descending knot pair")]
    DescendingKnots,
    /// The provided knots were all equal. Knots must contain at least one increasing pair.
    #[error("Invalid knots: all knots are equal")]
    ConstantKnots,
    /// Provided a different number of weights and control points.
    #[error("Incorrect number of weights: expected {expected}, provided {provided}")]
    WeightsNumberMismatch {
        /// Expected number of weights
        expected: usize,
        /// Provided number of weights
        provided: usize,
    },
    /// The number of control points provided is less than 4.
    #[error("Not enough control points, at least 4 are required, {provided} were provided")]
    NotEnoughControlPoints {
        /// The number of control points provided
        provided: usize,
    },
}

/// Non-uniform Rational B-Splines (NURBS) are a powerful generalization of the [`CubicBSpline`] which can
/// represent a much more diverse class of curves (like perfect circles and ellipses).
///
/// ### Non-uniformity
/// The 'NU' part of NURBS stands for "Non-Uniform". This has to do with a parameter called 'knots'.
/// The knots are a non-decreasing sequence of floating point numbers. The first and last three pairs of
/// knots control the behavior of the curve as it approaches its endpoints. The intermediate pairs
/// each control the length of one segment of the curve. Multiple repeated knot values are called
/// "knot multiplicity". Knot multiplicity in the intermediate knots causes a "zero-length" segment,
/// and can create sharp corners.
///
/// ### Rationality
/// The 'R' part of NURBS stands for "Rational". This has to do with NURBS allowing each control point to
/// be assigned a weighting, which controls how much it affects the curve compared to the other points.
///
/// ### Interpolation
/// The curve will not pass through the control points except where a knot has multiplicity four.
///
/// ### Tangency
/// Tangents are automatically computed based on the position of control points.
///
/// ### Continuity
/// When there is no knot multiplicity, the curve is C2 continuous, meaning it has no holes or jumps and the
/// tangent vector changes smoothly along the entire curve length. Like the [`CubicBSpline`], the acceleration
/// continuity makes it useful for camera paths. Knot multiplicity of 2 in intermediate knots reduces the
/// continuity to C2, and knot multiplicity of 3 reduces the continuity to C0. The curve is always at least
/// C0, meaning it has no jumps or holes.
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
/// let knots = [0.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 5.0];
/// let nurbs = CubicNurbs::new(points, Some(weights), Some(knots))
///     .expect("NURBS construction failed!")
///     .to_curve();
/// let positions: Vec<_> = nurbs.iter_positions(100).collect();
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct CubicNurbs<P: VectorSpace> {
    /// The control points of the NURBS
    pub control_points: Vec<P>,
    /// Weights
    pub weights: Vec<f32>,
    /// Knots
    pub knots: Vec<f32>,
}
impl<P: VectorSpace> CubicNurbs<P> {
    /// Build a Non-Uniform Rational B-Spline.
    ///
    /// If provided, weights must be the same length as the control points. Defaults to equal weights.
    ///
    /// If provided, the number of knots must be n + 4 elements, where n is the amount of control
    /// points. Defaults to open uniform knots: [`Self::open_uniform_knots`]. Knots cannot
    /// all be equal.
    ///
    /// At least 4 points must be provided, otherwise an error will be returned.
    pub fn new(
        control_points: impl Into<Vec<P>>,
        weights: Option<impl Into<Vec<f32>>>,
        knots: Option<impl Into<Vec<f32>>>,
    ) -> Result<Self, CubicNurbsError> {
        let mut control_points: Vec<P> = control_points.into();
        let control_points_len = control_points.len();

        if control_points_len < 4 {
            return Err(CubicNurbsError::NotEnoughControlPoints {
                provided: control_points_len,
            });
        }

        let weights = weights
            .map(Into::into)
            .unwrap_or_else(|| vec![1.0; control_points_len]);

        let mut knots: Vec<f32> = knots.map(Into::into).unwrap_or_else(|| {
            Self::open_uniform_knots(control_points_len)
                .expect("The amount of control points was checked")
        });

        let expected_knots_len = Self::knots_len(control_points_len);

        // Check the number of knots is correct
        if knots.len() != expected_knots_len {
            return Err(CubicNurbsError::KnotsNumberMismatch {
                expected: expected_knots_len,
                provided: knots.len(),
            });
        }

        // Ensure the knots are non-descending (previous element is less than or equal
        // to the next)
        if knots.windows(2).any(|win| win[0] > win[1]) {
            return Err(CubicNurbsError::DescendingKnots);
        }

        // Ensure the knots are non-constant
        if knots.windows(2).all(|win| win[0] == win[1]) {
            return Err(CubicNurbsError::ConstantKnots);
        }

        // Check that the number of weights equals the number of control points
        if weights.len() != control_points_len {
            return Err(CubicNurbsError::WeightsNumberMismatch {
                expected: control_points_len,
                provided: weights.len(),
            });
        }

        // To align the evaluation behavior of nurbs with the other splines,
        // make the intervals between knots form an exact cover of [0, N], where N is
        // the number of segments of the final curve.
        let curve_length = (control_points.len() - 3) as f32;
        let min = *knots.first().unwrap();
        let max = *knots.last().unwrap();
        let knot_delta = max - min;
        knots = knots
            .into_iter()
            .map(|k| k - min)
            .map(|k| k * curve_length / knot_delta)
            .collect();

        control_points
            .iter_mut()
            .zip(weights.iter())
            .for_each(|(p, w)| *p = *p * *w);

        Ok(Self {
            control_points,
            weights,
            knots,
        })
    }

    /// Generates uniform knots that will generate the same curve as [`CubicBSpline`].
    ///
    /// "Uniform" means that the difference between two subsequent knots is the same.
    ///
    /// Will return `None` if there are less than 4 control points.
    pub fn uniform_knots(control_points: usize) -> Option<Vec<f32>> {
        if control_points < 4 {
            return None;
        }
        Some(
            (0..Self::knots_len(control_points))
                .map(|v| v as f32)
                .collect(),
        )
    }

    /// Generates open uniform knots, which makes the ends of the curve pass through the
    /// start and end points.
    ///
    /// The start and end knots have multiplicity 4, and intermediate knots have multiplicity 0 and
    /// difference of 1.
    ///
    /// Will return `None` if there are less than 4 control points.
    pub fn open_uniform_knots(control_points: usize) -> Option<Vec<f32>> {
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
    const fn knots_len(control_points_len: usize) -> usize {
        control_points_len + 4
    }

    /// Generates a non-uniform B-spline characteristic matrix from a sequence of six knots. Each six
    /// knots describe the relationship between four successive control points. For padding reasons,
    /// this takes a vector of 8 knots, but only six are actually used.
    fn generate_matrix(knots: &[f32; 8]) -> [[f32; 4]; 4] {
        // A derivation for this matrix can be found in "General Matrix Representations for B-splines" by Kaihuai Qin.
        // <https://xiaoxingchen.github.io/2020/03/02/bspline_in_so3/general_matrix_representation_for_bsplines.pdf>
        // See section 3.1.

        let t = knots;
        // In the notation of the paper:
        // t[1] := t_i-2
        // t[2] := t_i-1
        // t[3] := t_i   (the lower extent of the current knot span)
        // t[4] := t_i+1 (the upper extent of the current knot span)
        // t[5] := t_i+2
        // t[6] := t_i+3

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
}
impl<P: VectorSpace> RationalGenerator<P> for CubicNurbs<P> {
    #[inline]
    fn to_curve(&self) -> RationalCurve<P> {
        let segments = self
            .control_points
            .windows(4)
            .zip(self.weights.windows(4))
            .zip(self.knots.windows(8))
            .filter(|(_, knots)| knots[4] - knots[3] > 0.0)
            .map(|((points, weights), knots)| {
                // This is curve segment i. It uses control points P_i, P_i+2, P_i+2 and P_i+3,
                // It is associated with knot span i+3 (which is the interval between knots i+3
                // and i+4) and its characteristic matrix uses knots i+1 through i+6 (because
                // those define the two knot spans on either side).
                let span = knots[4] - knots[3];
                let coefficient_knots = knots.try_into().expect("Knot windows are of length 6");
                let matrix = Self::generate_matrix(coefficient_knots);
                RationalSegment::coefficients(
                    points.try_into().expect("Point windows are of length 4"),
                    weights.try_into().expect("Weight windows are of length 4"),
                    span,
                    matrix,
                )
            })
            .collect();
        RationalCurve { segments }
    }
}

/// A spline interpolated linearly between the nearest 2 points.
///
/// ### Interpolation
/// The curve passes through every control point.
///
/// ### Tangency
/// The curve is not generally differentiable at control points.
///
/// ### Continuity
/// The curve is C0 continuous, meaning it has no holes or jumps.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct LinearSpline<P: VectorSpace> {
    /// The control points of the NURBS
    pub points: Vec<P>,
}
impl<P: VectorSpace> LinearSpline<P> {
    /// Create a new linear spline
    pub fn new(points: impl Into<Vec<P>>) -> Self {
        Self {
            points: points.into(),
        }
    }
}
impl<P: VectorSpace> CubicGenerator<P> for LinearSpline<P> {
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

/// Implement this on cubic splines that can generate a cubic curve from their spline parameters.
pub trait CubicGenerator<P: VectorSpace> {
    /// Build a [`CubicCurve`] by computing the interpolation coefficients for each curve segment.
    fn to_curve(&self) -> CubicCurve<P>;
}

/// A segment of a cubic curve, used to hold precomputed coefficients for fast interpolation.
/// Can be evaluated as a parametric curve over the domain `[0, 1)`.
///
/// Segments can be chained together to form a longer compound curve.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Default))]
pub struct CubicSegment<P: VectorSpace> {
    /// Coefficients of the segment
    pub coeff: [P; 4],
}

impl<P: VectorSpace> CubicSegment<P> {
    /// Instantaneous position of a point at parametric value `t`.
    #[inline]
    pub fn position(&self, t: f32) -> P {
        let [a, b, c, d] = self.coeff;
        // Evaluate `a + bt + ct^2 + dt^3`, avoiding exponentiation
        a + (b + (c + d * t) * t) * t
    }

    /// Instantaneous velocity of a point at parametric value `t`.
    #[inline]
    pub fn velocity(&self, t: f32) -> P {
        let [_, b, c, d] = self.coeff;
        // Evaluate the derivative, which is `b + 2ct + 3dt^2`, avoiding exponentiation
        b + (c * 2.0 + d * 3.0 * t) * t
    }

    /// Instantaneous acceleration of a point at parametric value `t`.
    #[inline]
    pub fn acceleration(&self, t: f32) -> P {
        let [_, _, c, d] = self.coeff;
        // Evaluate the second derivative, which is `2c + 6dt`
        c * 2.0 + d * 6.0 * t
    }

    /// Calculate polynomial coefficients for the cubic curve using a characteristic matrix.
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
        bezier.segments[0]
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

/// A collection of [`CubicSegment`]s chained into a single parametric curve. Has domain `[0, N)`
/// where `N` is the number of attached segments.
///
/// Use any struct that implements the [`CubicGenerator`] trait to create a new curve, such as
/// [`CubicBezier`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct CubicCurve<P: VectorSpace> {
    /// Segments of the curve
    pub segments: Vec<CubicSegment<P>>,
}

impl<P: VectorSpace> CubicCurve<P> {
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

impl<P: VectorSpace> Extend<CubicSegment<P>> for CubicCurve<P> {
    fn extend<T: IntoIterator<Item = CubicSegment<P>>>(&mut self, iter: T) {
        self.segments.extend(iter);
    }
}

impl<P: VectorSpace> IntoIterator for CubicCurve<P> {
    type IntoIter = <Vec<CubicSegment<P>> as IntoIterator>::IntoIter;

    type Item = CubicSegment<P>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
    }
}

/// Implement this on cubic splines that can generate a rational cubic curve from their spline parameters.
pub trait RationalGenerator<P: VectorSpace> {
    /// Build a [`RationalCurve`] by computing the interpolation coefficients for each curve segment.
    fn to_curve(&self) -> RationalCurve<P>;
}

/// A segment of a rational cubic curve, used to hold precomputed coefficients for fast interpolation.
/// Can be evaluated as a parametric curve over the domain `[0, knot_span)`.
///
/// Segments can be chained together to form a longer compound curve.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Default))]
pub struct RationalSegment<P: VectorSpace> {
    /// The coefficients matrix of the cubic curve.
    pub coeff: [P; 4],
    /// The homogeneous weight coefficients.
    pub weight_coeff: [f32; 4],
    /// The width of the domain of this segment.
    pub knot_span: f32,
}

impl<P: VectorSpace> RationalSegment<P> {
    /// Instantaneous position of a point at parametric value `t` in `[0, knot_span)`.
    #[inline]
    pub fn position(&self, t: f32) -> P {
        let [a, b, c, d] = self.coeff;
        let [x, y, z, w] = self.weight_coeff;
        // Compute a cubic polynomial for the control points
        let numerator = a + (b + (c + d * t) * t) * t;
        // Compute a cubic polynomial for the weights
        let denominator = x + (y + (z + w * t) * t) * t;
        numerator / denominator
    }

    /// Instantaneous velocity of a point at parametric value `t` in `[0, knot_span)`.
    #[inline]
    pub fn velocity(&self, t: f32) -> P {
        // A derivation for the following equations can be found in "Matrix representation for NURBS
        // curves and surfaces" by Choi et al. See equation 19.

        let [a, b, c, d] = self.coeff;
        let [x, y, z, w] = self.weight_coeff;
        // Compute a cubic polynomial for the control points
        let numerator = a + (b + (c + d * t) * t) * t;
        // Compute a cubic polynomial for the weights
        let denominator = x + (y + (z + w * t) * t) * t;

        // Compute the derivative of the control point polynomial
        let numerator_derivative = b + (c * 2.0 + d * 3.0 * t) * t;
        // Compute the derivative of the weight polynomial
        let denominator_derivative = y + (z * 2.0 + w * 3.0 * t) * t;

        // Velocity is the first derivative (wrt to the parameter `t`)
        // Position = N/D therefore
        // Velocity = (N/D)' = N'/D - N * D'/D^2 = (N' * D - N * D')/D^2
        numerator_derivative / denominator
            - numerator * (denominator_derivative / denominator.powi(2))
    }

    /// Instantaneous acceleration of a point at parametric value `t` in `[0, knot_span)`.
    #[inline]
    pub fn acceleration(&self, t: f32) -> P {
        // A derivation for the following equations can be found in "Matrix representation for NURBS
        // curves and surfaces" by Choi et al. See equation 20. Note: In come copies of this paper, equation 20
        // is printed with the following two errors:
        // + The first term has incorrect sign.
        // + The second term uses R when it should use the first derivative.

        let [a, b, c, d] = self.coeff;
        let [x, y, z, w] = self.weight_coeff;
        // Compute a cubic polynomial for the control points
        let numerator = a + (b + (c + d * t) * t) * t;
        // Compute a cubic polynomial for the weights
        let denominator = x + (y + (z + w * t) * t) * t;

        // Compute the derivative of the control point polynomial
        let numerator_derivative = b + (c * 2.0 + d * 3.0 * t) * t;
        // Compute the derivative of the weight polynomial
        let denominator_derivative = y + (z * 2.0 + w * 3.0 * t) * t;

        // Compute the second derivative of the control point polynomial
        let numerator_second_derivative = c * 2.0 + d * 6.0 * t;
        // Compute the second derivative of the weight polynomial
        let denominator_second_derivative = z * 2.0 + w * 6.0 * t;

        // Velocity is the first derivative (wrt to the parameter `t`)
        // Position = N/D therefore
        // Velocity = (N/D)' = N'/D - N * D'/D^2 = (N' * D - N * D')/D^2
        // Acceleration = (N/D)'' = ((N' * D - N * D')/D^2)' = N''/D + N' * (-2D'/D^2) + N * (-D''/D^2 + 2D'^2/D^3)
        numerator_second_derivative / denominator
            + numerator_derivative * (-2.0 * denominator_derivative / denominator.powi(2))
            + numerator
                * (-denominator_second_derivative / denominator.powi(2)
                    + 2.0 * denominator_derivative.powi(2) / denominator.powi(3))
    }

    /// Calculate polynomial coefficients for the cubic polynomials using a characteristic matrix.
    #[inline]
    fn coefficients(
        control_points: [P; 4],
        weights: [f32; 4],
        knot_span: f32,
        char_matrix: [[f32; 4]; 4],
    ) -> Self {
        // An explanation of this use can be found in "Matrix representation for NURBS curves and surfaces"
        // by Choi et al. See section "Evaluation of NURB Curves and Surfaces", and equation 16.

        let [c0, c1, c2, c3] = char_matrix;
        let p = control_points;
        let w = weights;
        // These are the control point polynomial coefficients, computed by multiplying the characteristic
        // matrix by the point matrix.
        let coeff = [
            p[0] * c0[0] + p[1] * c0[1] + p[2] * c0[2] + p[3] * c0[3],
            p[0] * c1[0] + p[1] * c1[1] + p[2] * c1[2] + p[3] * c1[3],
            p[0] * c2[0] + p[1] * c2[1] + p[2] * c2[2] + p[3] * c2[3],
            p[0] * c3[0] + p[1] * c3[1] + p[2] * c3[2] + p[3] * c3[3],
        ];
        // These are the weight polynomial coefficients, computed by multiplying the characteristic
        // matrix by the weight matrix.
        let weight_coeff = [
            w[0] * c0[0] + w[1] * c0[1] + w[2] * c0[2] + w[3] * c0[3],
            w[0] * c1[0] + w[1] * c1[1] + w[2] * c1[2] + w[3] * c1[3],
            w[0] * c2[0] + w[1] * c2[1] + w[2] * c2[2] + w[3] * c2[3],
            w[0] * c3[0] + w[1] * c3[1] + w[2] * c3[2] + w[3] * c3[3],
        ];
        Self {
            coeff,
            weight_coeff,
            knot_span,
        }
    }
}

/// A collection of [`RationalSegment`]s chained into a single parametric curve.
///
/// Use any struct that implements the [`RationalGenerator`] trait to create a new curve, such as
/// [`CubicNurbs`], or convert [`CubicCurve`] using `into/from`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct RationalCurve<P: VectorSpace> {
    /// The segments in the curve
    pub segments: Vec<RationalSegment<P>>,
}

impl<P: VectorSpace> RationalCurve<P> {
    /// Compute the position of a point on the curve at the parametric value `t`.
    ///
    /// Note that `t` varies from `0..=(n_points - 3)`.
    #[inline]
    pub fn position(&self, t: f32) -> P {
        let (segment, t) = self.segment(t);
        segment.position(t)
    }

    /// Compute the first derivative with respect to t at `t`. This is the instantaneous velocity of
    /// a point on the curve at `t`.
    ///
    /// Note that `t` varies from `0..=(n_points - 3)`.
    #[inline]
    pub fn velocity(&self, t: f32) -> P {
        let (segment, t) = self.segment(t);
        segment.velocity(t)
    }

    /// Compute the second derivative with respect to t at `t`. This is the instantaneous
    /// acceleration of a point on the curve at `t`.
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
        let domain = self.domain();
        let step = domain / subdivisions as f32;
        (0..=subdivisions).map(move |i| i as f32 * step)
    }

    /// The list of segments contained in this `RationalCurve`.
    ///
    /// This spline's global `t` value is equal to how many segments it has.
    ///
    /// All method accepting `t` on `RationalCurve` depends on the global `t`.
    /// When sampling over the entire curve, you should either use one of the
    /// `iter_*` methods or account for the segment count using `curve.segments().len()`.
    #[inline]
    pub fn segments(&self) -> &[RationalSegment<P>] {
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

    /// Adds a segment to the curve.
    #[inline]
    pub fn push_segment(&mut self, segment: RationalSegment<P>) {
        self.segments.push(segment);
    }

    /// Returns the [`RationalSegment`] and local `t` value given a spline's global `t` value.
    /// Input `t` will be clamped to the domain of the curve. Returned value will be in `[0, 1]`.
    #[inline]
    fn segment(&self, mut t: f32) -> (&RationalSegment<P>, f32) {
        if t <= 0.0 {
            (&self.segments[0], 0.0)
        } else if self.segments.len() == 1 {
            (&self.segments[0], t / self.segments[0].knot_span)
        } else {
            // Try to fit t into each segment domain
            for segment in self.segments.iter() {
                if t < segment.knot_span {
                    // The division here makes t a normalized parameter in [0, 1] that can be properly
                    // evaluated against a cubic curve segment. See equations 6 & 16 from "Matrix representation
                    // of NURBS curves and surfaces" by Choi et al. or equation 3 from "General Matrix
                    // Representations for B-Splines" by Qin.
                    return (segment, t / segment.knot_span);
                }
                t -= segment.knot_span;
            }
            return (self.segments.last().unwrap(), 1.0);
        }
    }

    /// Returns the length of of the domain of the parametric curve.
    #[inline]
    pub fn domain(&self) -> f32 {
        self.segments.iter().map(|segment| segment.knot_span).sum()
    }
}

impl<P: VectorSpace> Extend<RationalSegment<P>> for RationalCurve<P> {
    fn extend<T: IntoIterator<Item = RationalSegment<P>>>(&mut self, iter: T) {
        self.segments.extend(iter);
    }
}

impl<P: VectorSpace> IntoIterator for RationalCurve<P> {
    type IntoIter = <Vec<RationalSegment<P>> as IntoIterator>::IntoIter;

    type Item = RationalSegment<P>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
    }
}

impl<P: VectorSpace> From<CubicSegment<P>> for RationalSegment<P> {
    fn from(value: CubicSegment<P>) -> Self {
        Self {
            coeff: value.coeff,
            weight_coeff: [1.0, 0.0, 0.0, 0.0],
            knot_span: 1.0, // Cubic curves are uniform, so every segment has domain [0, 1).
        }
    }
}

impl<P: VectorSpace> From<CubicCurve<P>> for RationalCurve<P> {
    fn from(value: CubicCurve<P>) -> Self {
        Self {
            segments: value.segments.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::{vec2, Vec2};

    use crate::cubic_splines::{
        CubicBSpline, CubicBezier, CubicGenerator, CubicNurbs, CubicSegment, RationalCurve,
        RationalGenerator,
    };

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

    /// Test that a simple cardinal spline passes through all of its control points with
    /// the correct tangents.
    #[test]
    fn cardinal_control_pts() {
        use super::CubicCardinalSpline;

        let tension = 0.2;
        let [p0, p1, p2, p3] = [vec2(-1., -2.), vec2(0., 1.), vec2(1., 2.), vec2(-2., 1.)];
        let curve = CubicCardinalSpline::new(tension, [p0, p1, p2, p3]).to_curve();

        // Positions at segment endpoints
        assert!(curve.position(0.).abs_diff_eq(p0, FLOAT_EQ));
        assert!(curve.position(1.).abs_diff_eq(p1, FLOAT_EQ));
        assert!(curve.position(2.).abs_diff_eq(p2, FLOAT_EQ));
        assert!(curve.position(3.).abs_diff_eq(p3, FLOAT_EQ));

        // Tangents at segment endpoints
        assert!(curve
            .velocity(0.)
            .abs_diff_eq((p1 - p0) * tension * 2., FLOAT_EQ));
        assert!(curve
            .velocity(1.)
            .abs_diff_eq((p2 - p0) * tension, FLOAT_EQ));
        assert!(curve
            .velocity(2.)
            .abs_diff_eq((p3 - p1) * tension, FLOAT_EQ));
        assert!(curve
            .velocity(3.)
            .abs_diff_eq((p3 - p2) * tension * 2., FLOAT_EQ));
    }

    /// Test that [`RationalCurve`] properly generalizes [`CubicCurve`]. A Cubic upgraded to a rational
    /// should produce pretty much the same output.
    #[test]
    fn cubic_to_rational() {
        const EPSILON: f32 = 0.00001;

        let points = [
            vec2(0.0, 0.0),
            vec2(1.0, 1.0),
            vec2(1.0, 1.0),
            vec2(2.0, -1.0),
            vec2(3.0, 1.0),
            vec2(0.0, 0.0),
        ];

        let b_spline = CubicBSpline::new(points).to_curve();
        let rational_b_spline = RationalCurve::from(b_spline.clone());

        /// Tests if two vectors of points are approximately the same
        fn compare_vectors(cubic_curve: Vec<Vec2>, rational_curve: Vec<Vec2>, name: &str) {
            assert_eq!(
                cubic_curve.len(),
                rational_curve.len(),
                "{name} vector lengths mismatch"
            );
            for (i, (a, b)) in cubic_curve.iter().zip(rational_curve.iter()).enumerate() {
                assert!(
                    a.distance(*b) < EPSILON,
                    "Mismatch at {name} value {i}. CubicCurve: {} Converted RationalCurve: {}",
                    a,
                    b
                );
            }
        }

        // Both curves should yield the same values
        let cubic_positions: Vec<_> = b_spline.iter_positions(10).collect();
        let rational_positions: Vec<_> = rational_b_spline.iter_positions(10).collect();
        compare_vectors(cubic_positions, rational_positions, "position");

        let cubic_velocities: Vec<_> = b_spline.iter_velocities(10).collect();
        let rational_velocities: Vec<_> = rational_b_spline.iter_velocities(10).collect();
        compare_vectors(cubic_velocities, rational_velocities, "velocity");

        let cubic_accelerations: Vec<_> = b_spline.iter_accelerations(10).collect();
        let rational_accelerations: Vec<_> = rational_b_spline.iter_accelerations(10).collect();
        compare_vectors(cubic_accelerations, rational_accelerations, "acceleration");
    }

    /// Test that a nurbs curve can approximate a portion of a circle.
    #[test]
    fn nurbs_circular_arc() {
        use std::f32::consts::FRAC_PI_2;
        const EPSILON: f32 = 0.0000001;

        // The following NURBS parameters were determined by constraining the first two
        // points to the line y=1, the second two points to the line x=1, and the distance
        // between each pair of points to be equal. One can solve the weights by assuming the
        // first and last weights to be one, the intermediate weights to be equal, and
        // subjecting ones self to a lot of tedious matrix algebra.

        let alpha = FRAC_PI_2;
        let leg = 2.0 * f32::sin(alpha / 2.0) / (1.0 + 2.0 * f32::cos(alpha / 2.0));
        let weight = (1.0 + 2.0 * f32::cos(alpha / 2.0)) / 3.0;
        let points = [
            vec2(1.0, 0.0),
            vec2(1.0, leg),
            vec2(leg, 1.0),
            vec2(0.0, 1.0),
        ];
        let weights = [1.0, weight, weight, 1.0];
        let knots = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
        let spline = CubicNurbs::new(points, Some(weights), Some(knots)).unwrap();
        let curve = spline.to_curve();
        for (i, point) in curve.iter_positions(10).enumerate() {
            assert!(
                f32::abs(point.length() - 1.0) < EPSILON,
                "Point {i} is not on the unit circle: {point:?} has length {}",
                point.length()
            );
        }
    }
}
