//! Geometric measurement traits for primitive shapes.
//!
//! This module provides traits for computing common geometric measurements of
//! 2D and 3D primitives.
//!
//! - [`Measured2d`] provides methods for computing a shape's [perimeter](`Measured2d::perimeter`)
//!   and [area](`Measured2d::area`).
//! - [`Measured3d`] provides methods for computing a shape's [surface area](`Measured3d::area`) and
//!   [volume](`Measured3d::volume`).
//!
//! Implementations are provided for the standard primitives in [`bevy_shape`]
//! using exact formulas where practical, with documented approximations for
//! measurements that have no closed-form elementary solution.

use std::f32::consts::{FRAC_PI_3, PI};

use bevy_math::{ops, FloatPow};
use bevy_shape::{
    Annulus, Capsule2d, Capsule3d, Circle, CircularSector, CircularSegment, Cone, ConicalFrustum,
    Cuboid, Cylinder, Ellipse, Extrusion, Plane3d, Primitive2d, Rectangle, RegularPolygon, Rhombus,
    Sphere, Tetrahedron, Torus, Triangle2d, Triangle3d,
};

/// A trait for getting measurements of 2D shapes
pub trait Measured2d {
    /// Get the perimeter of the shape
    fn perimeter(&self) -> f32;

    /// Get the area of the shape
    fn area(&self) -> f32;
}

/// A trait for getting measurements of 3D shapes
pub trait Measured3d {
    /// Get the surface area of the shape
    fn area(&self) -> f32;

    /// Get the volume of the shape
    fn volume(&self) -> f32;
}

impl Measured2d for Circle {
    /// Get the area of the circle
    #[inline]
    fn area(&self) -> f32 {
        PI * self.radius.squared()
    }

    /// Get the perimeter or circumference of the circle
    #[inline]
    #[doc(alias = "circumference")]
    fn perimeter(&self) -> f32 {
        2.0 * PI * self.radius
    }
}

impl Measured2d for CircularSector {
    #[inline]
    fn area(&self) -> f32 {
        self.arc.radius.squared() * self.arc.half_angle
    }

    #[inline]
    fn perimeter(&self) -> f32 {
        if self.half_angle() >= PI {
            self.arc.radius * 2.0 * PI
        } else {
            2.0 * self.radius() + self.arc_length()
        }
    }
}

impl Measured2d for CircularSegment {
    #[inline]
    fn area(&self) -> f32 {
        0.5 * self.arc.radius.squared() * (self.arc.angle() - ops::sin(self.arc.angle()))
    }

    #[inline]
    fn perimeter(&self) -> f32 {
        self.chord_length() + self.arc_length()
    }
}

impl Measured2d for Ellipse {
    /// Get the area of the ellipse
    #[inline]
    fn area(&self) -> f32 {
        PI * self.half_size.x * self.half_size.y
    }

    #[inline]
    /// Get an approximation for the perimeter or circumference of the ellipse.
    ///
    /// The approximation is reasonably precise with a relative error less than 0.007%, getting more precise as the eccentricity of the ellipse decreases.
    fn perimeter(&self) -> f32 {
        let a = self.semi_major();
        let b = self.semi_minor();

        // In the case that `a == b`, the ellipse is a circle
        if a / b - 1. < 1e-5 {
            return PI * (a + b);
        };

        // In the case that `a` is much larger than `b`, the ellipse is a line
        if a / b > 1e4 {
            return 4. * a;
        };

        // These values are  the result of (0.5 choose n)^2 where n is the index in the array
        // They could be calculated on the fly but hardcoding them yields more accurate and faster results
        // because the actual calculation for these values involves factorials and numbers > 10^23
        const BINOMIAL_COEFFICIENTS: [f32; 21] = [
            1.,
            0.25,
            0.015625,
            0.00390625,
            0.0015258789,
            0.00074768066,
            0.00042057037,
            0.00025963783,
            0.00017140154,
            0.000119028846,
            0.00008599834,
            0.00006414339,
            0.000049109784,
            0.000038430585,
            0.000030636627,
            0.000024815668,
            0.000020380836,
            0.000016942893,
            0.000014236736,
            0.000012077564,
            0.000010333865,
        ];

        // The algorithm used here is the Gauss-Kummer infinite series expansion of the elliptic integral expression for the perimeter of ellipses
        // For more information see https://www.wolframalpha.com/input/?i=gauss-kummer+series
        // We only use the terms up to `i == 20` for this approximation
        let h = ((a - b) / (a + b)).squared();

        PI * (a + b)
            * (0..=20)
                .map(|i| BINOMIAL_COEFFICIENTS[i] * ops::powf(h, i as f32))
                .sum::<f32>()
    }
}

impl Measured2d for Annulus {
    /// Get the area of the annulus
    #[inline]
    fn area(&self) -> f32 {
        PI * (self.outer_circle.radius.squared() - self.inner_circle.radius.squared())
    }

    /// Get the perimeter or circumference of the annulus,
    /// which is the sum of the perimeters of the inner and outer circles.
    #[inline]
    #[doc(alias = "circumference")]
    fn perimeter(&self) -> f32 {
        2.0 * PI * (self.outer_circle.radius + self.inner_circle.radius)
    }
}

impl Measured2d for Rhombus {
    /// Get the area of the rhombus
    #[inline]
    fn area(&self) -> f32 {
        2.0 * self.half_diagonals.x * self.half_diagonals.y
    }

    /// Get the perimeter of the rhombus
    #[inline]
    fn perimeter(&self) -> f32 {
        4.0 * self.side()
    }
}

impl Measured2d for Triangle2d {
    /// Get the area of the triangle
    #[inline]
    fn area(&self) -> f32 {
        let [a, b, c] = self.vertices;
        ops::abs(a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)) / 2.0
    }

    /// Get the perimeter of the triangle
    #[inline]
    fn perimeter(&self) -> f32 {
        let [a, b, c] = self.vertices;

        let ab = a.distance(b);
        let bc = b.distance(c);
        let ca = c.distance(a);

        ab + bc + ca
    }
}

impl Measured2d for Rectangle {
    /// Get the area of the rectangle
    #[inline]
    fn area(&self) -> f32 {
        4.0 * self.half_size.x * self.half_size.y
    }

    /// Get the perimeter of the rectangle
    #[inline]
    fn perimeter(&self) -> f32 {
        4.0 * (self.half_size.x + self.half_size.y)
    }
}

impl Measured2d for RegularPolygon {
    /// Get the area of the regular polygon
    #[inline]
    fn area(&self) -> f32 {
        let angle: f32 = 2.0 * PI / (self.sides as f32);
        (self.sides as f32) * self.circumradius().squared() * ops::sin(angle) / 2.0
    }

    /// Get the perimeter of the regular polygon.
    /// This is the sum of its sides
    #[inline]
    fn perimeter(&self) -> f32 {
        self.sides as f32 * self.side_length()
    }
}

impl Measured2d for Capsule2d {
    /// Get the area of the capsule
    #[inline]
    fn area(&self) -> f32 {
        // pi*r^2 + (2r)*l
        PI * self.radius.squared() + self.to_inner_rectangle().area()
    }

    /// Get the perimeter of the capsule
    #[inline]
    fn perimeter(&self) -> f32 {
        // 2pi*r + 2l
        2.0 * PI * self.radius + 4.0 * self.half_length
    }
}

impl Measured3d for Sphere {
    /// Get the surface area of the sphere
    #[inline]
    fn area(&self) -> f32 {
        4.0 * PI * self.radius.squared()
    }

    /// Get the volume of the sphere
    #[inline]
    fn volume(&self) -> f32 {
        4.0 * FRAC_PI_3 * self.radius.cubed()
    }
}

impl Measured2d for Plane3d {
    #[inline]
    fn area(&self) -> f32 {
        self.half_size.element_product() * 4.0
    }

    #[inline]
    fn perimeter(&self) -> f32 {
        self.half_size.element_sum() * 4.0
    }
}

impl Measured3d for Cuboid {
    /// Get the surface area of the cuboid
    #[inline]
    fn area(&self) -> f32 {
        8.0 * (self.half_size.x * self.half_size.y
            + self.half_size.y * self.half_size.z
            + self.half_size.x * self.half_size.z)
    }

    /// Get the volume of the cuboid
    #[inline]
    fn volume(&self) -> f32 {
        8.0 * self.half_size.x * self.half_size.y * self.half_size.z
    }
}

impl Measured3d for Cylinder {
    /// Get the total surface area of the cylinder
    #[inline]
    fn area(&self) -> f32 {
        2.0 * PI * self.radius * (self.radius + 2.0 * self.half_height)
    }

    /// Get the volume of the cylinder
    #[inline]
    fn volume(&self) -> f32 {
        self.base_area() * 2.0 * self.half_height
    }
}

impl Measured3d for Capsule3d {
    /// Get the surface area of the capsule
    #[inline]
    fn area(&self) -> f32 {
        // Modified version of 2pi * r * (2r + h)
        4.0 * PI * self.radius * (self.radius + self.half_length)
    }

    /// Get the volume of the capsule
    #[inline]
    fn volume(&self) -> f32 {
        // Modified version of pi * r^2 * (4/3 * r + a)
        let diameter = self.radius * 2.0;
        PI * self.radius * diameter * (diameter / 3.0 + self.half_length)
    }
}

impl Measured3d for Cone {
    /// Get the total surface area of the cone
    #[inline]
    fn area(&self) -> f32 {
        self.base_area() + self.lateral_area()
    }

    /// Get the volume of the cone
    #[inline]
    fn volume(&self) -> f32 {
        (self.base_area() * self.height) / 3.0
    }
}

impl Measured3d for ConicalFrustum {
    #[inline]
    fn volume(&self) -> f32 {
        FRAC_PI_3
            * self.height
            * (self.radius_bottom * self.radius_bottom
                + self.radius_top * self.radius_top
                + self.radius_top * self.radius_bottom)
    }
    #[inline]
    fn area(&self) -> f32 {
        self.bottom_base_area() + self.top_base_area() + self.lateral_area()
    }
}

impl Measured3d for Torus {
    /// Get the surface area of the torus. Note that this only produces
    /// the expected result when the torus has a ring and isn't self-intersecting
    #[inline]
    fn area(&self) -> f32 {
        4.0 * PI.squared() * self.major_radius * self.minor_radius
    }

    /// Get the volume of the torus. Note that this only produces
    /// the expected result when the torus has a ring and isn't self-intersecting
    #[inline]
    fn volume(&self) -> f32 {
        2.0 * PI.squared() * self.major_radius * self.minor_radius.squared()
    }
}

impl Measured2d for Triangle3d {
    /// Get the area of the triangle.
    #[inline]
    fn area(&self) -> f32 {
        let [a, b, c] = self.vertices;
        let ab = b - a;
        let ac = c - a;
        ab.cross(ac).length() / 2.0
    }

    /// Get the perimeter of the triangle.
    #[inline]
    fn perimeter(&self) -> f32 {
        let [a, b, c] = self.vertices;
        a.distance(b) + b.distance(c) + c.distance(a)
    }
}

impl Measured3d for Tetrahedron {
    /// Get the surface area of the tetrahedron.
    #[inline]
    fn area(&self) -> f32 {
        let [a, b, c, d] = self.vertices;
        let ab = b - a;
        let ac = c - a;
        let ad = d - a;
        let bc = c - b;
        let bd = d - b;
        (ab.cross(ac).length()
            + ab.cross(ad).length()
            + ac.cross(ad).length()
            + bc.cross(bd).length())
            / 2.0
    }

    /// Get the volume of the tetrahedron.
    #[inline]
    fn volume(&self) -> f32 {
        ops::abs(self.signed_volume())
    }
}

impl<T: Primitive2d + Measured2d> Measured3d for Extrusion<T> {
    /// Get the surface area of the extrusion
    fn area(&self) -> f32 {
        2. * (self.base_shape.area() + self.half_depth * self.base_shape.perimeter())
    }

    /// Get the volume of the extrusion
    fn volume(&self) -> f32 {
        2. * self.base_shape.area() * self.half_depth
    }
}

#[cfg(test)]
mod tests {
    // Reference values were computed by hand and/or with external tools

    use super::*;
    use approx::assert_relative_eq;
    use bevy_math::{Vec2, Vec3};

    #[test]
    fn circle_math() {
        let circle = Circle { radius: 3.0 };
        assert_eq!(circle.area(), 28.274334, "incorrect area");
        assert_eq!(circle.perimeter(), 18.849556, "incorrect perimeter");
    }

    #[test]
    fn capsule2d_math() {
        let capsule = Capsule2d::new(2.0, 9.0);
        assert_eq!(capsule.area(), 48.566371, "incorrect area");
        assert_eq!(capsule.perimeter(), 30.566371, "incorrect perimeter");
    }

    #[test]
    fn annulus_math() {
        let annulus = Annulus::new(2.5, 3.5);
        assert_eq!(annulus.area(), 18.849556, "incorrect area");
        assert_eq!(annulus.perimeter(), 37.699112, "incorrect perimeter");
    }

    #[test]
    fn rhombus_math() {
        let rhombus = Rhombus::new(3.0, 4.0);
        assert_eq!(rhombus.area(), 6.0, "incorrect area");
        assert_eq!(rhombus.perimeter(), 10.0, "incorrect perimeter");
        let rhombus = Rhombus::new(0.0, 0.0);
        assert_eq!(rhombus.area(), 0.0, "incorrect area");
        assert_eq!(rhombus.perimeter(), 0.0, "incorrect perimeter");
    }

    #[test]
    fn ellipse_math() {
        let ellipse = Ellipse::new(3.0, 1.0);
        assert_eq!(ellipse.area(), 9.424778, "incorrect area");
    }

    #[test]
    fn ellipse_perimeter() {
        let circle = Ellipse::new(1., 1.);
        assert_relative_eq!(circle.perimeter(), 6.2831855);

        let line = Ellipse::new(75_000., 0.5);
        assert_relative_eq!(line.perimeter(), 300_000.);

        let ellipse = Ellipse::new(0.5, 2.);
        assert_relative_eq!(ellipse.perimeter(), 8.578423);

        let ellipse = Ellipse::new(5., 3.);
        assert_relative_eq!(ellipse.perimeter(), 25.526999);
    }

    #[test]
    fn triangle2d_math() {
        let triangle = Triangle2d::new(
            Vec2::new(-2.0, -1.0),
            Vec2::new(1.0, 4.0),
            Vec2::new(7.0, 0.0),
        );
        assert_eq!(triangle.area(), 21.0, "incorrect area");
        assert_eq!(triangle.perimeter(), 22.097439, "incorrect perimeter");
    }

    #[test]
    fn rectangle_math() {
        let rectangle = Rectangle::new(3.0, 7.0);
        assert_eq!(rectangle.area(), 21.0, "incorrect area");
        assert_eq!(rectangle.perimeter(), 20.0, "incorrect perimeter");
    }

    #[test]
    fn regular_polygon_math() {
        let polygon = RegularPolygon::new(3.0, 6);
        assert_relative_eq!(polygon.area(), 23.38268, epsilon = 0.00001);
        assert_eq!(polygon.perimeter(), 18.0, "incorrect perimeter");
    }

    #[test]
    fn sphere_math() {
        let sphere = Sphere { radius: 4.0 };
        assert_eq!(sphere.area(), 201.06193, "incorrect area");
        assert_eq!(sphere.volume(), 268.08257, "incorrect volume");
    }

    #[test]
    fn cuboid_math() {
        let cuboid = Cuboid::new(3.0, 7.0, 2.0);
        assert_eq!(cuboid.area(), 82.0, "incorrect area");
        assert_eq!(cuboid.volume(), 42.0, "incorrect volume");
    }

    #[test]
    fn cylinder_math() {
        let cylinder = Cylinder::new(2.0, 9.0);
        assert_relative_eq!(cylinder.area(), 138.23007);
        assert_eq!(cylinder.volume(), 113.097336, "incorrect volume");
    }

    #[test]
    fn capsule3d_math() {
        let capsule = Capsule3d::new(2.0, 9.0);
        assert_eq!(capsule.area(), 163.36282, "incorrect area");
        assert_relative_eq!(capsule.volume(), 146.60765);
    }

    #[test]
    fn cone_math() {
        let cone = Cone {
            radius: 2.0,
            height: 9.0,
        };
        assert_relative_eq!(cone.area(), 70.49447);
        assert_eq!(cone.volume(), 37.699111, "incorrect volume");
    }

    #[test]
    fn conical_frustum_math() {
        let frustum = ConicalFrustum {
            height: 9.0,
            radius_top: 1.0,
            radius_bottom: 2.0,
        };
        assert_eq!(frustum.area(), 101.05296, "incorrect surface area");
        assert_eq!(frustum.volume(), 65.97345, "incorrect volume");
    }

    #[test]
    fn torus_math() {
        let torus = Torus {
            minor_radius: 0.3,
            major_radius: 2.8,
        };
        assert_relative_eq!(torus.area(), 33.16187);
        assert_relative_eq!(torus.volume(), 4.97428, epsilon = 0.00001);
    }

    #[test]
    fn tetrahedron_math() {
        let tetrahedron = Tetrahedron {
            vertices: [
                Vec3::new(0.3, 1.0, 1.7),
                Vec3::new(-2.0, -1.0, 0.0),
                Vec3::new(1.8, 0.5, 1.0),
                Vec3::new(-1.0, -2.0, 3.5),
            ],
        };
        assert_eq!(tetrahedron.area(), 19.251068, "incorrect area");
        assert_eq!(tetrahedron.volume(), 3.2058334, "incorrect volume");

        assert_eq!(Tetrahedron::default().area(), 3.4641016, "incorrect area");
        assert_eq!(
            Tetrahedron::default().volume(),
            0.33333334,
            "incorrect volume"
        );
    }

    #[test]
    fn extrusion_math() {
        let circle = Circle::new(0.75);
        let cylinder = Extrusion::new(circle, 2.5);
        assert_eq!(cylinder.area(), 15.315264, "incorrect surface area");
        assert_eq!(cylinder.volume(), 4.417865, "incorrect volume");

        let annulus = Annulus::new(0.25, 1.375);
        let tube = Extrusion::new(annulus, 0.333);
        assert_eq!(tube.area(), 14.886437, "incorrect surface area");
        assert_eq!(tube.volume(), 1.9124937, "incorrect volume");

        let polygon = RegularPolygon::new(3.8, 7);
        let regular_prism = Extrusion::new(polygon, 1.25);
        assert_eq!(regular_prism.area(), 107.8808, "incorrect surface area");
        assert_eq!(regular_prism.volume(), 49.392204, "incorrect volume");
    }

    #[test]
    fn triangle3d_math() {
        // Default triangle tests
        let default_triangle = Triangle3d::default();
        assert_eq!(default_triangle.area(), 0.5, "incorrect area");
        assert_relative_eq!(
            default_triangle.perimeter(),
            1.0 + 2.0 * ops::sqrt(1.25_f32),
            epsilon = 10e-9
        );

        // Arbitrary triangle tests
        let [a, b, c] = [Vec3::ZERO, Vec3::new(1., 1., 0.5), Vec3::new(-3., 2.5, 1.)];
        let triangle = Triangle3d::new(a, b, c);

        assert_eq!(triangle.area(), 3.0233467, "incorrect area");
        assert_eq!(triangle.perimeter(), 9.832292, "incorrect perimeter");
    }
}
