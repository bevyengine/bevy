//! A curve through space.

use bevy_math::cubic_splines::{CubicCurve, Point};

/// A curve representing a path through space. Unlike [`CubicCurve`], it is sampled based on distance
/// along the path. Can be used for linearly animating along a curve.
///
/// Note: Values are approximated using a series of line segments. Accuracy is based on the
/// subdivisions specified when creating the path.
#[derive(Clone, Debug)]
pub struct CurvePath<P: Point> {
    curve: CubicCurve<P>,
    arc_lengths: Vec<f32>,
}

impl<P: Point> CurvePath<P> {
    /// Creates a new [`CurvePath`] from a curve. Subdivisions will determine how many line segments
    /// are used, and therefore the accuracy.
    pub fn from_cubic_curve(curve: CubicCurve<P>, subdivisions: usize) -> CurvePath<P> {
        let arc_lengths: Vec<f32> = curve
            .iter_positions(subdivisions)
            .scan((0.0, curve.position(0.0)), |state, x| {
                state.0 += x.distance(state.1);
                state.1 = x;

                Some(state.0)
            })
            .collect();

        CurvePath { curve, arc_lengths }
    }

    /// Returns the [`CubicCurve`] this path is based on.
    pub fn curve(&self) -> &CubicCurve<P> {
        &self.curve
    }

    /// Total length of the curve.
    pub fn length(&self) -> f32 {
        self.arc_lengths[self.arc_lengths.len() - 1]
    }

    /// Returns a 't' value corresponding to the `t` value on the underlying curve that matches the
    /// provided `distance`.
    pub fn t(&self, distance: f32) -> f32 {
        // Get index with greatest value that is less than or equal to target distance.
        let closest = self.arc_lengths.partition_point(|&x| x <= distance) - 1;

        // Check if index's distance perfectly matches target distance, otherwise lerp between the
        // index and its next neighbor.
        if self.arc_lengths[closest] == distance {
            (closest + 1) as f32 / (self.arc_lengths.len() + 1) as f32
        } else {
            let length_before = self.arc_lengths[closest];
            let length_after = self.arc_lengths[closest + 1];
            let segment_length = length_after - length_before;

            let segment_fraction = (distance - length_before) / segment_length;

            (closest as f32 + segment_fraction) / (self.arc_lengths.len() - 1) as f32
                * self.curve.segments().len() as f32
        }
    }

    /// Compute the position of a point on the curve at `distance`.
    pub fn position(&self, distance: f32) -> P {
        self.curve.position(self.t(distance))
    }

    /// Compute the first derivative at `distance`. This is the instantaneous velocity of a point on the cubic curve at `distance`.
    pub fn velocity(&self, distance: f32) -> P {
        self.curve.velocity(self.t(distance))
    }

    /// Compute the second derivative at `distance`. This is the instantaneous acceleration of a point on the cubic curve at `distance`.
    pub fn acceleration(&self, distance: f32) -> P {
        self.curve.acceleration(self.t(distance))
    }
}

#[cfg(test)]
mod tests {
    use bevy_math::{
        cubic_splines::{CubicBezier, CubicGenerator},
        vec2, Vec2,
    };

    use crate::curve_path::CurvePath;

    const SUBDIVISIONS: usize = 256;
    // The 0.551915 is taken from https://www.mechanicalexpressions.com/explore/geometric-modeling/circle-spline-approximation.pdf
    const CIRCLE_POINTS: [[Vec2; 4]; 4] = [
        [
            vec2(1., 0.),
            vec2(1., 0.551915),
            vec2(0.551915, 1.),
            vec2(0., 1.),
        ],
        [
            vec2(0., 1.),
            vec2(-0.551915, 1.),
            vec2(-1., 0.55228475),
            vec2(-1., 0.),
        ],
        [
            vec2(-1., 0.),
            vec2(-1., -0.551915),
            vec2(-0.551915, -1.),
            vec2(0., -1.),
        ],
        [
            vec2(0., -1.),
            vec2(0.551915, -1.),
            vec2(1., -0.551915),
            vec2(1., 0.),
        ],
    ];

    #[test]
    fn length() {
        let path =
            CurvePath::from_cubic_curve(CubicBezier::new(CIRCLE_POINTS).to_curve(), SUBDIVISIONS);

        let length = path.length();

        // Length should be 2 PI
        assert!(6.283 < length && length < 6.284);
    }

    #[test]
    fn sample() {
        let points = [[0.0, 0.2, 0.8, 1.0]];
        let bezier = CubicBezier::new(points).to_curve();
        let curve_path = CurvePath::from_cubic_curve(bezier, SUBDIVISIONS);

        assert_eq!(curve_path.length(), 1.0);

        // Check with exact point.
        assert_eq!(curve_path.t(0.5), 0.5);

        // Check with value between two points.
        let position = curve_path.position(0.25);
        assert!(0.24998 < position && position < 0.25001);
    }

    #[test]
    fn sample_segmented_curve() {
        let path =
            CurvePath::from_cubic_curve(CubicBezier::new(CIRCLE_POINTS).to_curve(), SUBDIVISIONS);

        // Circle starts at (1, 0) and goes counter-clockwise so PI should be (-1, 0)
        let position = path.position(std::f32::consts::PI);

        assert!(position.distance(vec2(-1., 0.)) < 0.001);
    }
}
