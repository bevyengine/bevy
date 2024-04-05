use bevy_math::DMat2;
use bevy_math::DVec2;
use bevy_math::Vec2;
use std::f64::consts::TAU;

//iterator for generating a set of points around a unit circle, beginning at 0 radians
#[derive(Debug, Clone)]
pub(crate) struct CircleIterator {
    count: usize,
    rot: DMat2,
    pos: DVec2,
}

impl CircleIterator {
    //produces an iterator over (count) equidistant points that starts at (1.0, 0.0) on the unit circle
    pub(crate) fn new(count: usize) -> CircleIterator {
        Self {
            count,
            rot: DMat2::from_angle(TAU / (count as f64)),
            pos: DVec2::new(1.0, 0.0),
        }
    }

    //produces an iterator over (count) equidistant points that starts at (1.0, 0.0) on the unit circle, with an additional end point of (1.0, 0.0)
    pub(crate) fn wrapping(count: usize) -> impl Iterator<Item = Vec2> {
        Self::new(count).chain(std::iter::once(Vec2::new(1.0, 0.0)))
    }

    //semicircle with points ranging from 0 radians to pi radians, with (count) regions between points.
    pub(crate) fn semicircle(count: usize) -> impl Iterator<Item = Vec2> {
        Self::new(count * 2)
            .take(count)
            .chain(std::iter::once(Vec2::new(-1.0, 0.0)))
    }

    //quarter circle with points ranging from 0 radians to pi/2 radians, with (count) regions between points.
    pub(crate) fn quarter_circle(count: usize) -> impl Iterator<Item = Vec2> {
        Self::new(count * 4)
            .take(count)
            .chain(std::iter::once(Vec2::new(0.0, 1.0)))
    }
}
impl Iterator for CircleIterator {
    type Item = Vec2;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count != 0 {
            let prev = self.pos.as_vec2();
            self.pos = self.rot * self.pos;
            self.count -= 1;
            Some(prev)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use super::CircleIterator;
    use bevy_math::Vec2;
    #[test]
    fn circle_iterator_has_correct_length() {
        let vertices: Vec<Vec2> = CircleIterator::new(6).collect();
        assert_eq!(vertices.len(), 6);
    }

    #[test]
    fn circle_iterator_vertices_are_equidistant() {
        let epsilon = 0.00001;
        let vertices: Vec<Vec2> = CircleIterator::new(6).collect();
        let center = Vec2::new(0.0, 0.0);
        let distances_center: Vec<f32> = vertices.iter().map(|x| center.distance(*x)).collect();
        assert!(distances_center
            .windows(2)
            .all(|w| (w[0] - w[1]).abs() < epsilon));
        let distances_neighbors: Vec<f32> =
            vertices.windows(2).map(|w| w[0].distance(w[1])).collect();
        assert!(distances_neighbors
            .windows(2)
            .all(|w| (w[0] - w[1]).abs() < epsilon));
    }
}
