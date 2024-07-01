//! The [`ShapeSample`] trait, allowing random sampling from geometric shapes.
//!
//! At the most basic level, this allows sampling random points from the interior and boundary of
//! geometric primitives. For example:
//! ```
//! # use bevy_math::primitives::*;
//! # use bevy_math::ShapeSample;
//! # use rand::SeedableRng;
//! # use rand::rngs::StdRng;
//! // Get some `Rng`:
//! let rng = &mut StdRng::from_entropy();
//! // Make a circle of radius 2:
//! let circle = Circle::new(2.0);
//! // Get a point inside this circle uniformly at random:
//! let interior_pt = circle.sample_interior(rng);
//! // Get a point on the circle's boundary uniformly at random:
//! let boundary_pt = circle.sample_boundary(rng);
//! ```
//!
//! For repeated sampling, `ShapeSample` also includes methods for accessing a [`Distribution`]:
//! ```
//! # use bevy_math::primitives::*;
//! # use bevy_math::{Vec2, ShapeSample};
//! # use rand::SeedableRng;
//! # use rand::rngs::StdRng;
//! # use rand::distributions::Distribution;
//! # let rng1 = StdRng::from_entropy();
//! # let rng2 = StdRng::from_entropy();
//! // Use a rectangle this time:
//! let rectangle = Rectangle::new(1.0, 2.0);
//! // Get an iterator that spits out random interior points:
//! let interior_iter = rectangle.interior_dist().sample_iter(rng1);
//! // Collect random interior points from the iterator:
//! let interior_pts: Vec<Vec2> = interior_iter.take(1000).collect();
//! // Similarly, get an iterator over many random boundary points and collect them:
//! let boundary_pts: Vec<Vec2> = rectangle.boundary_dist().sample_iter(rng2).take(1000).collect();
//! ```
//!
//! In any case, the [`Rng`] used as the source of randomness must be provided explicitly.

use std::f32::consts::{PI, TAU};

use crate::{primitives::*, NormedVectorSpace, Vec2, Vec3};
use rand::{
    distributions::{Distribution, WeightedIndex},
    Rng,
};

/// Exposes methods to uniformly sample a variety of primitive shapes.
pub trait ShapeSample {
    /// The type of vector returned by the sample methods, [`Vec2`] for 2D shapes and [`Vec3`] for 3D shapes.
    type Output;

    /// Uniformly sample a point from inside the area/volume of this shape, centered on 0.
    ///
    /// Shapes like [`Cylinder`], [`Capsule2d`] and [`Capsule3d`] are oriented along the y-axis.
    ///
    /// # Example
    /// ```
    /// # use bevy_math::prelude::*;
    /// let square = Rectangle::new(2.0, 2.0);
    ///
    /// // Returns a Vec2 with both x and y between -1 and 1.
    /// println!("{:?}", square.sample_interior(&mut rand::thread_rng()));
    /// ```
    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output;

    /// Uniformly sample a point from the surface of this shape, centered on 0.
    ///
    /// Shapes like [`Cylinder`], [`Capsule2d`] and [`Capsule3d`] are oriented along the y-axis.
    ///
    /// # Example
    /// ```
    /// # use bevy_math::prelude::*;
    /// let square = Rectangle::new(2.0, 2.0);
    ///
    /// // Returns a Vec2 where one of the coordinates is at ±1,
    /// //  and the other is somewhere between -1 and 1.
    /// println!("{:?}", square.sample_boundary(&mut rand::thread_rng()));
    /// ```
    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output;

    /// Extract a [`Distribution`] whose samples are points of this shape's interior, taken uniformly.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// # use rand::distributions::Distribution;
    /// let square = Rectangle::new(2.0, 2.0);
    /// let rng = rand::thread_rng();
    ///
    /// // Iterate over points randomly drawn from `square`'s interior:
    /// for random_val in square.interior_dist().sample_iter(rng).take(5) {
    ///     println!("{:?}", random_val);
    /// }
    /// ```
    fn interior_dist(self) -> impl Distribution<Self::Output>
    where
        Self: Sized,
    {
        InteriorOf(self)
    }

    /// Extract a [`Distribution`] whose samples are points of this shape's boundary, taken uniformly.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// # use rand::distributions::Distribution;
    /// let square = Rectangle::new(2.0, 2.0);
    /// let rng = rand::thread_rng();
    ///
    /// // Iterate over points randomly drawn from `square`'s boundary:
    /// for random_val in square.boundary_dist().sample_iter(rng).take(5) {
    ///     println!("{:?}", random_val);
    /// }
    /// ```
    fn boundary_dist(self) -> impl Distribution<Self::Output>
    where
        Self: Sized,
    {
        BoundaryOf(self)
    }
}

#[derive(Clone, Copy)]
/// A wrapper struct that allows interior sampling from a [`ShapeSample`] type directly as
/// a [`Distribution`].
pub struct InteriorOf<T: ShapeSample>(pub T);

#[derive(Clone, Copy)]
/// A wrapper struct that allows boundary sampling from a [`ShapeSample`] type directly as
/// a [`Distribution`].
pub struct BoundaryOf<T: ShapeSample>(pub T);

impl<T: ShapeSample> Distribution<<T as ShapeSample>::Output> for InteriorOf<T> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> <T as ShapeSample>::Output {
        self.0.sample_interior(rng)
    }
}

impl<T: ShapeSample> Distribution<<T as ShapeSample>::Output> for BoundaryOf<T> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> <T as ShapeSample>::Output {
        self.0.sample_boundary(rng)
    }
}

impl ShapeSample for Circle {
    type Output = Vec2;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec2 {
        // https://mathworld.wolfram.com/DiskPointPicking.html
        let theta = rng.gen_range(0.0..TAU);
        let r_squared = rng.gen_range(0.0..=(self.radius * self.radius));
        let r = r_squared.sqrt();
        Vec2::new(r * theta.cos(), r * theta.sin())
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec2 {
        let theta = rng.gen_range(0.0..TAU);
        Vec2::new(self.radius * theta.cos(), self.radius * theta.sin())
    }
}

impl ShapeSample for Sphere {
    type Output = Vec3;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        // https://mathworld.wolfram.com/SpherePointPicking.html
        let theta = rng.gen_range(0.0..TAU);
        let phi = rng.gen_range(-1.0_f32..1.0).acos();
        let r_cubed = rng.gen_range(0.0..=(self.radius * self.radius * self.radius));
        let r = r_cubed.cbrt();
        Vec3 {
            x: r * phi.sin() * theta.cos(),
            y: r * phi.sin() * theta.sin(),
            z: r * phi.cos(),
        }
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        let theta = rng.gen_range(0.0..TAU);
        let phi = rng.gen_range(-1.0_f32..1.0).acos();
        Vec3 {
            x: self.radius * phi.sin() * theta.cos(),
            y: self.radius * phi.sin() * theta.sin(),
            z: self.radius * phi.cos(),
        }
    }
}

impl ShapeSample for Annulus {
    type Output = Vec2;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        let inner_radius = self.inner_circle.radius;
        let outer_radius = self.outer_circle.radius;

        // Like random sampling for a circle, radius is weighted by the square.
        let r_squared = rng.gen_range((inner_radius * inner_radius)..(outer_radius * outer_radius));
        let r = r_squared.sqrt();
        let theta = rng.gen_range(0.0..TAU);

        Vec2::new(r * theta.cos(), r * theta.sin())
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        let total_perimeter = self.inner_circle.perimeter() + self.outer_circle.perimeter();
        let inner_prob = (self.inner_circle.perimeter() / total_perimeter) as f64;

        // Sample from boundary circles, choosing which one by weighting by perimeter:
        let inner = rng.gen_bool(inner_prob);
        if inner {
            self.inner_circle.sample_boundary(rng)
        } else {
            self.outer_circle.sample_boundary(rng)
        }
    }
}

impl ShapeSample for Rectangle {
    type Output = Vec2;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec2 {
        let x = rng.gen_range(-self.half_size.x..=self.half_size.x);
        let y = rng.gen_range(-self.half_size.y..=self.half_size.y);
        Vec2::new(x, y)
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec2 {
        let primary_side = rng.gen_range(-1.0..1.0);
        let other_side = if rng.gen() { -1.0 } else { 1.0 };

        if self.half_size.x + self.half_size.y > 0.0 {
            if rng.gen_bool((self.half_size.x / (self.half_size.x + self.half_size.y)) as f64) {
                Vec2::new(primary_side, other_side) * self.half_size
            } else {
                Vec2::new(other_side, primary_side) * self.half_size
            }
        } else {
            Vec2::ZERO
        }
    }
}

impl ShapeSample for Cuboid {
    type Output = Vec3;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        let x = rng.gen_range(-self.half_size.x..=self.half_size.x);
        let y = rng.gen_range(-self.half_size.y..=self.half_size.y);
        let z = rng.gen_range(-self.half_size.z..=self.half_size.z);
        Vec3::new(x, y, z)
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        let primary_side1 = rng.gen_range(-1.0..1.0);
        let primary_side2 = rng.gen_range(-1.0..1.0);
        let other_side = if rng.gen() { -1.0 } else { 1.0 };

        if let Ok(dist) = WeightedIndex::new([
            self.half_size.y * self.half_size.z,
            self.half_size.x * self.half_size.z,
            self.half_size.x * self.half_size.y,
        ]) {
            match dist.sample(rng) {
                0 => Vec3::new(other_side, primary_side1, primary_side2) * self.half_size,
                1 => Vec3::new(primary_side1, other_side, primary_side2) * self.half_size,
                2 => Vec3::new(primary_side1, primary_side2, other_side) * self.half_size,
                _ => unreachable!(),
            }
        } else {
            Vec3::ZERO
        }
    }
}

/// Interior sampling for triangles which doesn't depend on the ambient dimension.
fn sample_triangle_interior<P: NormedVectorSpace, R: Rng + ?Sized>(
    vertices: [P; 3],
    rng: &mut R,
) -> P {
    let [a, b, c] = vertices;
    let ab = b - a;
    let ac = c - a;

    // Generate random points on a parallelipiped and reflect so that
    // we can use the points that lie outside the triangle
    let u = rng.gen_range(0.0..=1.0);
    let v = rng.gen_range(0.0..=1.0);

    if u + v > 1. {
        let u1 = 1. - v;
        let v1 = 1. - u;
        a + (ab * u1 + ac * v1)
    } else {
        a + (ab * u + ac * v)
    }
}

/// Boundary sampling for triangles which doesn't depend on the ambient dimension.
fn sample_triangle_boundary<P: NormedVectorSpace, R: Rng + ?Sized>(
    vertices: [P; 3],
    rng: &mut R,
) -> P {
    let [a, b, c] = vertices;
    let ab = b - a;
    let ac = c - a;
    let bc = c - b;

    let t = rng.gen_range(0.0..=1.0);

    if let Ok(dist) = WeightedIndex::new([ab.norm(), ac.norm(), bc.norm()]) {
        match dist.sample(rng) {
            0 => a.lerp(b, t),
            1 => a.lerp(c, t),
            2 => b.lerp(c, t),
            _ => unreachable!(),
        }
    } else {
        // This should only occur when the triangle is 0-dimensional degenerate
        // so this is actually the correct result.
        a
    }
}

impl ShapeSample for Triangle2d {
    type Output = Vec2;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        sample_triangle_interior(self.vertices, rng)
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        sample_triangle_boundary(self.vertices, rng)
    }
}

impl ShapeSample for Triangle3d {
    type Output = Vec3;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        sample_triangle_interior(self.vertices, rng)
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        sample_triangle_boundary(self.vertices, rng)
    }
}

impl ShapeSample for Tetrahedron {
    type Output = Vec3;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        let [v0, v1, v2, v3] = self.vertices;

        // Generate a random point in a cube:
        let mut coords: [f32; 3] = [
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
        ];

        // The cube is broken into six tetrahedra of the form 0 <= c_0 <= c_1 <= c_2 <= 1,
        // where c_i are the three euclidean coordinates in some permutation. (Since 3! = 6,
        // there are six of them). Sorting the coordinates folds these six tetrahedra into the
        // tetrahedron 0 <= x <= y <= z <= 1 (i.e. a fundamental domain of the permutation action).
        coords.sort_by(|x, y| x.partial_cmp(y).unwrap());

        // Now, convert a point from the fundamental tetrahedron into barycentric coordinates by
        // taking the four successive differences of coordinates; note that these telescope to sum
        // to 1, and this transformation is linear, hence preserves the probability density, since
        // the latter comes from the Lebesgue measure.
        //
        // (See https://en.wikipedia.org/wiki/Lebesgue_measure#Properties — specifically, that
        // Lebesgue measure of a linearly transformed set is its original measure times the
        // determinant.)
        let (a, b, c, d) = (
            coords[0],
            coords[1] - coords[0],
            coords[2] - coords[1],
            1. - coords[2],
        );

        // This is also a linear mapping, so probability density is still preserved.
        v0 * a + v1 * b + v2 * c + v3 * d
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        let triangles = self.faces();
        let areas = triangles.iter().map(|t| t.area());

        if areas.clone().sum::<f32>() > 0.0 {
            // There is at least one triangle with nonzero area, so this unwrap succeeds.
            let dist = WeightedIndex::new(areas).unwrap();

            // Get a random index, then sample the interior of the associated triangle.
            let idx = dist.sample(rng);
            triangles[idx].sample_interior(rng)
        } else {
            // In this branch the tetrahedron has zero surface area; just return a point that's on
            // the tetrahedron.
            self.vertices[0]
        }
    }
}

impl ShapeSample for Cylinder {
    type Output = Vec3;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        let Vec2 { x, y: z } = self.base().sample_interior(rng);
        let y = rng.gen_range(-self.half_height..=self.half_height);
        Vec3::new(x, y, z)
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        // This uses the area of the ends divided by the overall surface area (optimised)
        // [2 (\pi r^2)]/[2 (\pi r^2) + 2 \pi r h] = r/(r + h)
        if self.radius + 2.0 * self.half_height > 0.0 {
            if rng.gen_bool((self.radius / (self.radius + 2.0 * self.half_height)) as f64) {
                let Vec2 { x, y: z } = self.base().sample_interior(rng);
                if rng.gen() {
                    Vec3::new(x, self.half_height, z)
                } else {
                    Vec3::new(x, -self.half_height, z)
                }
            } else {
                let Vec2 { x, y: z } = self.base().sample_boundary(rng);
                let y = rng.gen_range(-self.half_height..=self.half_height);
                Vec3::new(x, y, z)
            }
        } else {
            Vec3::ZERO
        }
    }
}

impl ShapeSample for Capsule2d {
    type Output = Vec2;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec2 {
        let rectangle_area = self.half_length * self.radius * 4.0;
        let capsule_area = rectangle_area + PI * self.radius * self.radius;
        if capsule_area > 0.0 {
            // Check if the random point should be inside the rectangle
            if rng.gen_bool((rectangle_area / capsule_area) as f64) {
                let rectangle = Rectangle::new(self.radius, self.half_length * 2.0);
                rectangle.sample_interior(rng)
            } else {
                let circle = Circle::new(self.radius);
                let point = circle.sample_interior(rng);
                // Add half length if it is the top semi-circle, otherwise subtract half
                if point.y > 0.0 {
                    point + Vec2::Y * self.half_length
                } else {
                    point - Vec2::Y * self.half_length
                }
            }
        } else {
            Vec2::ZERO
        }
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec2 {
        let rectangle_surface = 4.0 * self.half_length;
        let capsule_surface = rectangle_surface + TAU * self.radius;
        if capsule_surface > 0.0 {
            if rng.gen_bool((rectangle_surface / capsule_surface) as f64) {
                let side_distance =
                    rng.gen_range((-2.0 * self.half_length)..=(2.0 * self.half_length));
                if side_distance < 0.0 {
                    Vec2::new(self.radius, side_distance + self.half_length)
                } else {
                    Vec2::new(-self.radius, side_distance - self.half_length)
                }
            } else {
                let circle = Circle::new(self.radius);
                let point = circle.sample_boundary(rng);
                // Add half length if it is the top semi-circle, otherwise subtract half
                if point.y > 0.0 {
                    point + Vec2::Y * self.half_length
                } else {
                    point - Vec2::Y * self.half_length
                }
            }
        } else {
            Vec2::ZERO
        }
    }
}

impl ShapeSample for Capsule3d {
    type Output = Vec3;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        let cylinder_vol = PI * self.radius * self.radius * 2.0 * self.half_length;
        // Add 4/3 pi r^3
        let capsule_vol = cylinder_vol + 4.0 / 3.0 * PI * self.radius * self.radius * self.radius;
        if capsule_vol > 0.0 {
            // Check if the random point should be inside the cylinder
            if rng.gen_bool((cylinder_vol / capsule_vol) as f64) {
                self.to_cylinder().sample_interior(rng)
            } else {
                let sphere = Sphere::new(self.radius);
                let point = sphere.sample_interior(rng);
                // Add half length if it is the top semi-sphere, otherwise subtract half
                if point.y > 0.0 {
                    point + Vec3::Y * self.half_length
                } else {
                    point - Vec3::Y * self.half_length
                }
            }
        } else {
            Vec3::ZERO
        }
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        let cylinder_surface = TAU * self.radius * 2.0 * self.half_length;
        let capsule_surface = cylinder_surface + 4.0 * PI * self.radius * self.radius;
        if capsule_surface > 0.0 {
            if rng.gen_bool((cylinder_surface / capsule_surface) as f64) {
                let Vec2 { x, y: z } = Circle::new(self.radius).sample_boundary(rng);
                let y = rng.gen_range(-self.half_length..=self.half_length);
                Vec3::new(x, y, z)
            } else {
                let sphere = Sphere::new(self.radius);
                let point = sphere.sample_boundary(rng);
                // Add half length if it is the top semi-sphere, otherwise subtract half
                if point.y > 0.0 {
                    point + Vec3::Y * self.half_length
                } else {
                    point - Vec3::Y * self.half_length
                }
            }
        } else {
            Vec3::ZERO
        }
    }
}

impl<P: Primitive2d + Measured2d + ShapeSample<Output = Vec2>> ShapeSample for Extrusion<P> {
    type Output = Vec3;

    fn sample_interior<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        let base_point = self.base_shape.sample_interior(rng);
        let depth = rng.gen_range(-self.half_depth..self.half_depth);
        base_point.extend(depth)
    }

    fn sample_boundary<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        let base_area = self.base_shape.area();
        let total_area = self.area();

        let random = rng.gen_range(0.0..total_area);
        match random {
            x if x < base_area => self.base_shape.sample_interior(rng).extend(self.half_depth),
            x if x < 2. * base_area => self
                .base_shape
                .sample_interior(rng)
                .extend(-self.half_depth),
            _ => self
                .base_shape
                .sample_boundary(rng)
                .extend(rng.gen_range(-self.half_depth..self.half_depth)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn circle_interior_sampling() {
        let mut rng = ChaCha8Rng::from_seed(Default::default());
        let circle = Circle::new(8.0);

        let boxes = [
            (-3.0, 3.0),
            (1.0, 2.0),
            (-1.0, -2.0),
            (3.0, -2.0),
            (1.0, -6.0),
            (-3.0, -7.0),
            (-7.0, -3.0),
            (-6.0, 1.0),
        ];
        let mut box_hits = [0; 8];

        // Checks which boxes (if any) the sampled points are in
        for _ in 0..5000 {
            let point = circle.sample_interior(&mut rng);

            for (i, box_) in boxes.iter().enumerate() {
                if (point.x > box_.0 && point.x < box_.0 + 4.0)
                    && (point.y > box_.1 && point.y < box_.1 + 4.0)
                {
                    box_hits[i] += 1;
                }
            }
        }

        assert_eq!(
            box_hits,
            [396, 377, 415, 404, 366, 408, 408, 430],
            "samples will occur across all array items at statistically equal chance"
        );
    }

    #[test]
    fn circle_boundary_sampling() {
        let mut rng = ChaCha8Rng::from_seed(Default::default());
        let circle = Circle::new(1.0);

        let mut wedge_hits = [0; 8];

        // Checks in which eighth of the circle each sampled point is in
        for _ in 0..5000 {
            let point = circle.sample_boundary(&mut rng);

            let angle = f32::atan(point.y / point.x) + PI / 2.0;
            let wedge = (angle * 8.0 / PI).floor() as usize;
            wedge_hits[wedge] += 1;
        }

        assert_eq!(
            wedge_hits,
            [636, 608, 639, 603, 614, 650, 640, 610],
            "samples will occur across all array items at statistically equal chance"
        );
    }
}
