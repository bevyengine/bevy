use std::f32::consts::TAU;
use bevy_math::Vec2;


#[derive(Debug, Clone, Copy)]
pub(crate) struct CircleIterator {
    count: usize,
    theta: f32,
    wrap: bool
}

impl CircleIterator {
    pub(crate) fn new(count: usize, wrap: bool) -> CircleIterator {
        if wrap  {
            Self {count: count+1, theta: -TAU/(count as f32), wrap }
        } else {
            Self {count: count, theta: -TAU/(count as f32), wrap }
        }

    }
}
impl Iterator for CircleIterator {
    type Item = Vec2;
    fn next(&mut self) -> Option<Self::Item> {


        if self.count != 0 {
            if self.wrap {
                self.count -= 1;
                Some(Vec2::new(
                    (self.theta*self.count as f32).cos(),
                    (self.theta*self.count as f32).sin()
                ))
            } else {
                let res = Some(Vec2::new(
                    (self.theta*self.count as f32).cos(),
                    (self.theta*self.count as f32).sin()
                ));
                self.count -= 1;
                res
            }

        } else {
            None
        }
    }
}

mod tests {
    #[test]
    fn circle_iterator_has_correct_length() {
        let vertices: Vec<Vec2> = CircleIterator::new(6, false).collect();
        assert_eq!(vertices.len(), 6);
        let vertices: Vec<Vec2> = CircleIterator::new(6, true).collect();
        assert_eq!(vertices.len(), 7);
    }

    #[test]
    fn circle_iterator_vertices_are_equidistant() {
        let epsilon = 0.00001;
        let mut vertices : Vec<Vec2> = CircleIterator::new(6, true).collect();
        let center = Vec2::new(0.0, 0.0);
        let distances_center: Vec<f32> = vertices.iter().map(|x| center.distance(*x)).collect();
        assert!(distances_center.windows(2).all(|w| (w[0] - w[1]).abs() < epsilon));
        let distances_neighbors: Vec<f32> = vertices.windows(2).map(|w| w[0].distance(w[1])).collect();
        assert!(distances_neighbors.windows(2).all(|w| (w[0] - w[1]).abs() < epsilon));
    }
}