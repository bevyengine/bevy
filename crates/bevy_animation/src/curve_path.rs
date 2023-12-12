use bevy_math::cubic_splines::{CubicCurve, Point};

/// A curve that has been divided into straight line segments.
#[derive(Clone, Debug)]
pub struct CurvePath<P: Point> {
    curve: CubicCurve<P>,
    arc_lengths: Vec<f32>,
}

impl<P: Point> CurvePath<P> {
    /// Splits the curve into subdivisions of straight line segments that can be used for computing
    /// spatial length.
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

    /// Total length of the curve.
    pub fn length(&self) -> f32 {
        self.arc_lengths[self.arc_lengths.len() - 1]
    }

    /// Returns a 't' value between 0..=1 based on the distance along the curve.
    fn sample(&self, distance: f32) -> f32 {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_math::cubic_splines::{CubicBezier, CubicGenerator};

    use crate::curve_path::CurvePath;

    #[test]
    fn sample() {
        const SUBDIVISIONS: usize = 1000;
        let points = [[0.0, 0.2, 0.8, 1.0]];
        let bezier = CubicBezier::new(points).to_curve();
        let curve_path = CurvePath::from_cubic_curve(bezier, SUBDIVISIONS);

        assert_eq!(curve_path.length(), 1.0);

        // Check with exact point.
        assert_eq!(curve_path.sample(0.5), 0.5);

        // Check with value between two points.
        let t = curve_path.sample(0.25);
        let position = curve_path.curve.position(t);
        assert!(0.24998 < position && position < 0.25001);
    }
}
