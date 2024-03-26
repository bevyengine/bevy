use std::f32::consts::{FRAC_PI_2, FRAC_PI_3, PI};

use super::{Primitive2d, WindingOrder};
use crate::{Dir2, Vec2};

/// A circle primitive
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Circle {
    /// The radius of the circle
    pub radius: f32,
}
impl Primitive2d for Circle {}

impl Default for Circle {
    /// Returns the default [`Circle`] with a radius of `0.5`.
    fn default() -> Self {
        Self { radius: 0.5 }
    }
}

impl Circle {
    /// Create a new [`Circle`] from a `radius`
    #[inline(always)]
    pub const fn new(radius: f32) -> Self {
        Self { radius }
    }

    /// Get the diameter of the circle
    #[inline(always)]
    pub fn diameter(&self) -> f32 {
        2.0 * self.radius
    }

    /// Get the area of the circle
    #[inline(always)]
    pub fn area(&self) -> f32 {
        PI * self.radius.powi(2)
    }

    /// Get the perimeter or circumference of the circle
    #[inline(always)]
    #[doc(alias = "circumference")]
    pub fn perimeter(&self) -> f32 {
        2.0 * PI * self.radius
    }

    /// Finds the point on the circle that is closest to the given `point`.
    ///
    /// If the point is outside the circle, the returned point will be on the perimeter of the circle.
    /// Otherwise, it will be inside the circle and returned as is.
    #[inline(always)]
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        let distance_squared = point.length_squared();

        if distance_squared <= self.radius.powi(2) {
            // The point is inside the circle.
            point
        } else {
            // The point is outside the circle.
            // Find the closest point on the perimeter of the circle.
            let dir_to_point = point / distance_squared.sqrt();
            self.radius * dir_to_point
        }
    }
}

/// A primitive representing an arc between two points on a circle.
///
/// An arc has no area.
/// If you want to include the portion of a circle's area swept out by the arc,
/// use the pie-shaped [`CircularSector`].
/// If you want to include only the space inside the convex hull of the arc,
/// use the bowl-shaped [`CircularSegment`].
///
/// The arc is drawn starting from [`Vec2::Y`], extending by `half_angle` radians on
/// either side. The center of the circle is the origin [`Vec2::ZERO`]. Note that this
/// means that the origin may not be within the `Arc2d`'s convex hull.
///
/// **Warning:** Arcs with negative angle or radius, or with angle greater than an entire circle, are not officially supported.
/// It is recommended to normalize arcs to have an angle in [0, 2π].
#[derive(Clone, Copy, Debug, PartialEq)]
#[doc(alias("CircularArc", "CircleArc"))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Arc2d {
    /// The radius of the circle
    pub radius: f32,
    /// Half the angle defining the arc
    pub half_angle: f32,
}
impl Primitive2d for Arc2d {}

impl Default for Arc2d {
    /// Returns the default [`Arc2d`] with radius `0.5`, covering one third of a circle
    fn default() -> Self {
        Self {
            radius: 0.5,
            half_angle: 2.0 * FRAC_PI_3,
        }
    }
}

impl Arc2d {
    /// Create a new [`Arc2d`] from a `radius` and a `half_angle`
    #[inline(always)]
    pub fn new(radius: f32, half_angle: f32) -> Self {
        Self { radius, half_angle }
    }

    /// Create a new [`Arc2d`] from a `radius` and an `angle` in radians
    #[inline(always)]
    pub fn from_radians(radius: f32, angle: f32) -> Self {
        Self {
            radius,
            half_angle: angle / 2.0,
        }
    }

    /// Create a new [`Arc2d`] from a `radius` and an angle in `degrees`.
    #[inline(always)]
    pub fn from_degrees(radius: f32, degrees: f32) -> Self {
        Self {
            radius,
            half_angle: degrees.to_radians() / 2.0,
        }
    }

    /// Create a new [`Arc2d`] from a `radius` and a `fraction` of a circle
    ///
    /// A `fraction` of 1.0 would be a whole circle; 0.5 would be a semicircle
    #[inline(always)]
    pub fn from_fraction(radius: f32, fraction: f32) -> Self {
        Self {
            radius,
            half_angle: fraction * PI,
        }
    }

    /// Get the angle of the arc
    #[inline(always)]
    pub fn angle(&self) -> f32 {
        self.half_angle * 2.0
    }

    /// Get the length of the arc
    #[inline(always)]
    pub fn length(&self) -> f32 {
        self.angle() * self.radius
    }

    /// Get the right-hand end point of the arc
    #[inline(always)]
    pub fn right_endpoint(&self) -> Vec2 {
        self.radius * Vec2::from_angle(FRAC_PI_2 - self.half_angle)
    }

    /// Get the left-hand end point of the arc
    #[inline(always)]
    pub fn left_endpoint(&self) -> Vec2 {
        self.radius * Vec2::from_angle(FRAC_PI_2 + self.half_angle)
    }

    /// Get the endpoints of the arc
    #[inline(always)]
    pub fn endpoints(&self) -> [Vec2; 2] {
        [self.left_endpoint(), self.right_endpoint()]
    }

    /// Get the midpoint of the arc
    #[inline]
    pub fn midpoint(&self) -> Vec2 {
        self.radius * Vec2::Y
    }

    /// Get half the distance between the endpoints (half the length of the chord)
    #[inline(always)]
    pub fn half_chord_length(&self) -> f32 {
        self.radius * f32::sin(self.half_angle)
    }

    /// Get the distance between the endpoints (the length of the chord)
    #[inline(always)]
    pub fn chord_length(&self) -> f32 {
        2.0 * self.half_chord_length()
    }

    /// Get the midpoint of the two endpoints (the midpoint of the chord)
    #[inline(always)]
    pub fn chord_midpoint(&self) -> Vec2 {
        self.apothem() * Vec2::Y
    }

    /// Get the length of the apothem of this arc, that is,
    /// the distance from the center of the circle to the midpoint of the chord, in the direction of the midpoint of the arc.
    /// Equivalently, the [`radius`](Self::radius) minus the [`sagitta`](Self::sagitta).
    ///
    /// Note that for a [`major`](Self::is_major) arc, the apothem will be negative.
    #[inline(always)]
    // Naming note: Various sources are inconsistent as to whether the apothem is the segment between the center and the
    // midpoint of a chord, or the length of that segment. Given this confusion, we've opted for the definition
    // used by Wolfram MathWorld, which is the distance rather than the segment.
    pub fn apothem(&self) -> f32 {
        let sign = if self.is_minor() { 1.0 } else { -1.0 };
        sign * f32::sqrt(self.radius.powi(2) - self.half_chord_length().powi(2))
    }

    /// Get the length of the sagitta of this arc, that is,
    /// the length of the line between the midpoints of the arc and its chord.
    /// Equivalently, the height of the triangle whose base is the chord and whose apex is the midpoint of the arc.
    ///
    /// The sagitta is also the sum of the [`radius`](Self::radius) and the [`apothem`](Self::apothem).
    pub fn sagitta(&self) -> f32 {
        self.radius - self.apothem()
    }

    /// Produces true if the arc is at most half a circle.
    ///
    /// **Note:** This is not the negation of [`is_major`](Self::is_major): an exact semicircle is both major and minor.
    #[inline(always)]
    pub fn is_minor(&self) -> bool {
        self.half_angle <= FRAC_PI_2
    }

    /// Produces true if the arc is at least half a circle.
    ///
    /// **Note:** This is not the negation of [`is_minor`](Self::is_minor): an exact semicircle is both major and minor.
    #[inline(always)]
    pub fn is_major(&self) -> bool {
        self.half_angle >= FRAC_PI_2
    }
}

/// A primitive representing a circular sector: a pie slice of a circle.
///
/// The segment is drawn starting from [`Vec2::Y`], extending equally on either side.
/// To orient the sector differently, apply a rotation.
/// The sector is drawn with the center of its circle at the origin [`Vec2::ZERO`].
///
/// **Warning:** Circular sectors with negative angle or radius, or with angle greater than an entire circle, are not officially supported.
/// We recommend normalizing circular sectors to have an angle in [0, 2π].
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct CircularSector {
    /// The arc defining the sector
    #[cfg_attr(feature = "serialize", serde(flatten))]
    pub arc: Arc2d,
}
impl Primitive2d for CircularSector {}

impl Default for CircularSector {
    /// Returns the default [`CircularSector`] with radius `0.5` and covering a third of a circle
    fn default() -> Self {
        Self::from(Arc2d::default())
    }
}

impl From<Arc2d> for CircularSector {
    fn from(arc: Arc2d) -> Self {
        Self { arc }
    }
}

impl CircularSector {
    /// Create a new [`CircularSector`] from a `radius` and an `angle`
    #[inline(always)]
    pub fn new(radius: f32, angle: f32) -> Self {
        Self::from(Arc2d::new(radius, angle))
    }

    /// Create a new [`CircularSector`] from a `radius` and an `angle` in radians.
    #[inline(always)]
    pub fn from_radians(radius: f32, angle: f32) -> Self {
        Self::from(Arc2d::from_radians(radius, angle))
    }

    /// Create a new [`CircularSector`] from a `radius` and an angle in `degrees`.
    #[inline(always)]
    pub fn from_degrees(radius: f32, degrees: f32) -> Self {
        Self::from(Arc2d::from_degrees(radius, degrees))
    }

    /// Create a new [`CircularSector`] from a `radius` and a `fraction` of a circle.
    ///
    /// A `fraction` of 1.0 would be a whole circle; 0.5 would be a semicircle.
    #[inline(always)]
    pub fn from_fraction(radius: f32, fraction: f32) -> Self {
        Self::from(Arc2d::from_fraction(radius, fraction))
    }

    /// Get half the angle of the sector
    #[inline(always)]
    pub fn half_angle(&self) -> f32 {
        self.arc.half_angle
    }

    /// Get the angle of the sector
    #[inline(always)]
    pub fn angle(&self) -> f32 {
        self.arc.angle()
    }

    /// Get the radius of the sector
    #[inline(always)]
    pub fn radius(&self) -> f32 {
        self.arc.radius
    }

    /// Get the length of the arc defining the sector
    #[inline(always)]
    pub fn arc_length(&self) -> f32 {
        self.arc.length()
    }

    /// Get half the length of the chord defined by the sector
    ///
    /// See [`Arc2d::half_chord_length`]
    #[inline(always)]
    pub fn half_chord_length(&self) -> f32 {
        self.arc.half_chord_length()
    }

    /// Get the length of the chord defined by the sector
    ///
    /// See [`Arc2d::chord_length`]
    #[inline(always)]
    pub fn chord_length(&self) -> f32 {
        self.arc.chord_length()
    }

    /// Get the midpoint of the chord defined by the sector
    ///
    /// See [`Arc2d::chord_midpoint`]
    #[inline(always)]
    pub fn chord_midpoint(&self) -> Vec2 {
        self.arc.chord_midpoint()
    }

    /// Get the length of the apothem of this sector
    ///
    /// See [`Arc2d::apothem`]
    #[inline(always)]
    pub fn apothem(&self) -> f32 {
        self.arc.apothem()
    }

    /// Get the length of the sagitta of this sector
    ///
    /// See [`Arc2d::sagitta`]
    #[inline(always)]
    pub fn sagitta(&self) -> f32 {
        self.arc.sagitta()
    }

    /// Returns the area of this sector
    #[inline(always)]
    pub fn area(&self) -> f32 {
        self.arc.radius.powi(2) * self.arc.half_angle
    }
}

/// A primitive representing a circular segment:
/// the area enclosed by the arc of a circle and its chord (the line between its endpoints).
///
/// The segment is drawn starting from [`Vec2::Y`], extending equally on either side.
/// To orient the segment differently, apply a rotation.
/// The segment is drawn with the center of its circle at the origin [`Vec2::ZERO`].
/// When positioning a segment, the [`apothem`](Self::apothem) function may be particularly useful.
///
/// **Warning:** Circular segments with negative angle or radius, or with angle greater than an entire circle, are not officially supported.
/// We recommend normalizing circular segments to have an angle in [0, 2π].
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct CircularSegment {
    /// The arc defining the segment
    #[cfg_attr(feature = "serialize", serde(flatten))]
    pub arc: Arc2d,
}
impl Primitive2d for CircularSegment {}

impl Default for CircularSegment {
    /// Returns the default [`CircularSegment`] with radius `0.5` and covering a third of a circle
    fn default() -> Self {
        Self::from(Arc2d::default())
    }
}

impl From<Arc2d> for CircularSegment {
    fn from(arc: Arc2d) -> Self {
        Self { arc }
    }
}

impl CircularSegment {
    /// Create a new [`CircularSegment`] from a `radius`, and an `angle`
    #[inline(always)]
    pub fn new(radius: f32, angle: f32) -> Self {
        Self::from(Arc2d::new(radius, angle))
    }

    /// Create a new [`CircularSegment`] from a `radius` and an `angle` in radians.
    #[inline(always)]
    pub fn from_radians(radius: f32, angle: f32) -> Self {
        Self::from(Arc2d::from_radians(radius, angle))
    }

    /// Create a new [`CircularSegment`] from a `radius` and an angle in `degrees`
    #[inline(always)]
    pub fn from_degrees(radius: f32, degrees: f32) -> Self {
        Self::from(Arc2d::from_degrees(radius, degrees))
    }

    /// Create a new [`CircularSegment`] from a `radius` and a `fraction` of a circle.
    ///
    /// A `fraction` of 1.0 would be a whole circle; 0.5 would be a semicircle.
    #[inline(always)]
    pub fn from_fraction(radius: f32, fraction: f32) -> Self {
        Self::from(Arc2d::from_fraction(radius, fraction))
    }

    /// Get the half-angle of the segment
    #[inline(always)]
    pub fn half_angle(&self) -> f32 {
        self.arc.half_angle
    }

    /// Get the angle of the segment
    #[inline(always)]
    pub fn angle(&self) -> f32 {
        self.arc.angle()
    }

    /// Get the radius of the segment
    #[inline(always)]
    pub fn radius(&self) -> f32 {
        self.arc.radius
    }

    /// Get the length of the arc defining the segment
    #[inline(always)]
    pub fn arc_length(&self) -> f32 {
        self.arc.length()
    }

    /// Get half the length of the segment's base, also known as its chord
    #[inline(always)]
    #[doc(alias = "half_base_length")]
    pub fn half_chord_length(&self) -> f32 {
        self.arc.half_chord_length()
    }

    /// Get the length of the segment's base, also known as its chord
    #[inline(always)]
    #[doc(alias = "base_length")]
    #[doc(alias = "base")]
    pub fn chord_length(&self) -> f32 {
        self.arc.chord_length()
    }

    /// Get the midpoint of the segment's base, also known as its chord
    #[inline(always)]
    #[doc(alias = "base_midpoint")]
    pub fn chord_midpoint(&self) -> Vec2 {
        self.arc.chord_midpoint()
    }

    /// Get the length of the apothem of this segment,
    /// which is the signed distance between the segment and the center of its circle
    ///
    /// See [`Arc2d::apothem`]
    #[inline(always)]
    pub fn apothem(&self) -> f32 {
        self.arc.apothem()
    }

    /// Get the length of the sagitta of this segment, also known as its height
    ///
    /// See [`Arc2d::sagitta`]
    #[inline(always)]
    #[doc(alias = "height")]
    pub fn sagitta(&self) -> f32 {
        self.arc.sagitta()
    }

    /// Returns the area of this segment
    #[inline(always)]
    pub fn area(&self) -> f32 {
        0.5 * self.arc.radius.powi(2) * (self.arc.angle() - self.arc.angle().sin())
    }
}

#[cfg(test)]
mod arc_tests {
    use std::f32::consts::FRAC_PI_4;

    use approx::assert_abs_diff_eq;

    use super::*;

    struct ArcTestCase {
        radius: f32,
        half_angle: f32,
        angle: f32,
        length: f32,
        right_endpoint: Vec2,
        left_endpoint: Vec2,
        endpoints: [Vec2; 2],
        midpoint: Vec2,
        half_chord_length: f32,
        chord_length: f32,
        chord_midpoint: Vec2,
        apothem: f32,
        sagitta: f32,
        is_minor: bool,
        is_major: bool,
        sector_area: f32,
        segment_area: f32,
    }

    impl ArcTestCase {
        fn check_arc(&self, arc: Arc2d) {
            assert_abs_diff_eq!(self.radius, arc.radius);
            assert_abs_diff_eq!(self.half_angle, arc.half_angle);
            assert_abs_diff_eq!(self.angle, arc.angle());
            assert_abs_diff_eq!(self.length, arc.length());
            assert_abs_diff_eq!(self.right_endpoint, arc.right_endpoint());
            assert_abs_diff_eq!(self.left_endpoint, arc.left_endpoint());
            assert_abs_diff_eq!(self.endpoints[0], arc.endpoints()[0]);
            assert_abs_diff_eq!(self.endpoints[1], arc.endpoints()[1]);
            assert_abs_diff_eq!(self.midpoint, arc.midpoint());
            assert_abs_diff_eq!(self.half_chord_length, arc.half_chord_length());
            assert_abs_diff_eq!(self.chord_length, arc.chord_length(), epsilon = 0.00001);
            assert_abs_diff_eq!(self.chord_midpoint, arc.chord_midpoint());
            assert_abs_diff_eq!(self.apothem, arc.apothem());
            assert_abs_diff_eq!(self.sagitta, arc.sagitta());
            assert_eq!(self.is_minor, arc.is_minor());
            assert_eq!(self.is_major, arc.is_major());
        }

        fn check_sector(&self, sector: CircularSector) {
            assert_abs_diff_eq!(self.radius, sector.radius());
            assert_abs_diff_eq!(self.half_angle, sector.half_angle());
            assert_abs_diff_eq!(self.angle, sector.angle());
            assert_abs_diff_eq!(self.half_chord_length, sector.half_chord_length());
            assert_abs_diff_eq!(self.chord_length, sector.chord_length(), epsilon = 0.00001);
            assert_abs_diff_eq!(self.chord_midpoint, sector.chord_midpoint());
            assert_abs_diff_eq!(self.apothem, sector.apothem());
            assert_abs_diff_eq!(self.sagitta, sector.sagitta());
            assert_abs_diff_eq!(self.sector_area, sector.area());
        }

        fn check_segment(&self, segment: CircularSegment) {
            assert_abs_diff_eq!(self.radius, segment.radius());
            assert_abs_diff_eq!(self.half_angle, segment.half_angle());
            assert_abs_diff_eq!(self.angle, segment.angle());
            assert_abs_diff_eq!(self.half_chord_length, segment.half_chord_length());
            assert_abs_diff_eq!(self.chord_length, segment.chord_length(), epsilon = 0.00001);
            assert_abs_diff_eq!(self.chord_midpoint, segment.chord_midpoint());
            assert_abs_diff_eq!(self.apothem, segment.apothem());
            assert_abs_diff_eq!(self.sagitta, segment.sagitta());
            assert_abs_diff_eq!(self.segment_area, segment.area());
        }
    }

    #[test]
    fn zero_angle() {
        let tests = ArcTestCase {
            radius: 1.0,
            half_angle: 0.0,
            angle: 0.0,
            length: 0.0,
            left_endpoint: Vec2::Y,
            right_endpoint: Vec2::Y,
            endpoints: [Vec2::Y, Vec2::Y],
            midpoint: Vec2::Y,
            half_chord_length: 0.0,
            chord_length: 0.0,
            chord_midpoint: Vec2::Y,
            apothem: 1.0,
            sagitta: 0.0,
            is_minor: true,
            is_major: false,
            sector_area: 0.0,
            segment_area: 0.0,
        };

        tests.check_arc(Arc2d::new(1.0, 0.0));
        tests.check_sector(CircularSector::new(1.0, 0.0));
        tests.check_segment(CircularSegment::new(1.0, 0.0));
    }

    #[test]
    fn zero_radius() {
        let tests = ArcTestCase {
            radius: 0.0,
            half_angle: FRAC_PI_4,
            angle: FRAC_PI_2,
            length: 0.0,
            left_endpoint: Vec2::ZERO,
            right_endpoint: Vec2::ZERO,
            endpoints: [Vec2::ZERO, Vec2::ZERO],
            midpoint: Vec2::ZERO,
            half_chord_length: 0.0,
            chord_length: 0.0,
            chord_midpoint: Vec2::ZERO,
            apothem: 0.0,
            sagitta: 0.0,
            is_minor: true,
            is_major: false,
            sector_area: 0.0,
            segment_area: 0.0,
        };

        tests.check_arc(Arc2d::new(0.0, FRAC_PI_4));
        tests.check_sector(CircularSector::new(0.0, FRAC_PI_4));
        tests.check_segment(CircularSegment::new(0.0, FRAC_PI_4));
    }

    #[test]
    fn quarter_circle() {
        let sqrt_half: f32 = f32::sqrt(0.5);
        let tests = ArcTestCase {
            radius: 1.0,
            half_angle: FRAC_PI_4,
            angle: FRAC_PI_2,
            length: FRAC_PI_2,
            left_endpoint: Vec2::new(-sqrt_half, sqrt_half),
            right_endpoint: Vec2::splat(sqrt_half),
            endpoints: [Vec2::new(-sqrt_half, sqrt_half), Vec2::splat(sqrt_half)],
            midpoint: Vec2::Y,
            half_chord_length: sqrt_half,
            chord_length: f32::sqrt(2.0),
            chord_midpoint: Vec2::new(0.0, sqrt_half),
            apothem: sqrt_half,
            sagitta: 1.0 - sqrt_half,
            is_minor: true,
            is_major: false,
            sector_area: FRAC_PI_4,
            segment_area: FRAC_PI_4 - 0.5,
        };

        tests.check_arc(Arc2d::from_fraction(1.0, 0.25));
        tests.check_sector(CircularSector::from_fraction(1.0, 0.25));
        tests.check_segment(CircularSegment::from_fraction(1.0, 0.25));
    }

    #[test]
    fn half_circle() {
        let tests = ArcTestCase {
            radius: 1.0,
            half_angle: FRAC_PI_2,
            angle: PI,
            length: PI,
            left_endpoint: Vec2::NEG_X,
            right_endpoint: Vec2::X,
            endpoints: [Vec2::NEG_X, Vec2::X],
            midpoint: Vec2::Y,
            half_chord_length: 1.0,
            chord_length: 2.0,
            chord_midpoint: Vec2::ZERO,
            apothem: 0.0,
            sagitta: 1.0,
            is_minor: true,
            is_major: true,
            sector_area: FRAC_PI_2,
            segment_area: FRAC_PI_2,
        };

        tests.check_arc(Arc2d::from_radians(1.0, PI));
        tests.check_sector(CircularSector::from_radians(1.0, PI));
        tests.check_segment(CircularSegment::from_radians(1.0, PI));
    }

    #[test]
    fn full_circle() {
        let tests = ArcTestCase {
            radius: 1.0,
            half_angle: PI,
            angle: 2.0 * PI,
            length: 2.0 * PI,
            left_endpoint: Vec2::NEG_Y,
            right_endpoint: Vec2::NEG_Y,
            endpoints: [Vec2::NEG_Y, Vec2::NEG_Y],
            midpoint: Vec2::Y,
            half_chord_length: 0.0,
            chord_length: 0.0,
            chord_midpoint: Vec2::NEG_Y,
            apothem: -1.0,
            sagitta: 2.0,
            is_minor: false,
            is_major: true,
            sector_area: PI,
            segment_area: PI,
        };

        tests.check_arc(Arc2d::from_degrees(1.0, 360.0));
        tests.check_sector(CircularSector::from_degrees(1.0, 360.0));
        tests.check_segment(CircularSegment::from_degrees(1.0, 360.0));
    }
}

/// An ellipse primitive
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Ellipse {
    /// Half of the width and height of the ellipse.
    ///
    /// This corresponds to the two perpendicular radii defining the ellipse.
    pub half_size: Vec2,
}
impl Primitive2d for Ellipse {}

impl Default for Ellipse {
    /// Returns the default [`Ellipse`] with a half-width of `1.0` and a half-height of `0.5`.
    fn default() -> Self {
        Self {
            half_size: Vec2::new(1.0, 0.5),
        }
    }
}

impl Ellipse {
    /// Create a new `Ellipse` from half of its width and height.
    ///
    /// This corresponds to the two perpendicular radii defining the ellipse.
    #[inline(always)]
    pub const fn new(half_width: f32, half_height: f32) -> Self {
        Self {
            half_size: Vec2::new(half_width, half_height),
        }
    }

    /// Create a new `Ellipse` from a given full size.
    ///
    /// `size.x` is the diameter along the X axis, and `size.y` is the diameter along the Y axis.
    #[inline(always)]
    pub fn from_size(size: Vec2) -> Self {
        Self {
            half_size: size / 2.0,
        }
    }

    /// Returns the length of the semi-major axis. This corresponds to the longest radius of the ellipse.
    #[inline(always)]
    pub fn semi_major(self) -> f32 {
        self.half_size.max_element()
    }

    /// Returns the length of the semi-minor axis. This corresponds to the shortest radius of the ellipse.
    #[inline(always)]
    pub fn semi_minor(self) -> f32 {
        self.half_size.min_element()
    }

    /// Get the area of the ellipse
    #[inline(always)]
    pub fn area(&self) -> f32 {
        PI * self.half_size.x * self.half_size.y
    }
}

/// A primitive shape formed by the region between two circles, also known as a ring.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "Ring")]
pub struct Annulus {
    /// The inner circle of the annulus
    pub inner_circle: Circle,
    /// The outer circle of the annulus
    pub outer_circle: Circle,
}
impl Primitive2d for Annulus {}

impl Default for Annulus {
    /// Returns the default [`Annulus`] with radii of `0.5` and `1.0`.
    fn default() -> Self {
        Self {
            inner_circle: Circle::new(0.5),
            outer_circle: Circle::new(1.0),
        }
    }
}

impl Annulus {
    /// Create a new [`Annulus`] from the radii of the inner and outer circle
    #[inline(always)]
    pub const fn new(inner_radius: f32, outer_radius: f32) -> Self {
        Self {
            inner_circle: Circle::new(inner_radius),
            outer_circle: Circle::new(outer_radius),
        }
    }

    /// Get the diameter of the annulus
    #[inline(always)]
    pub fn diameter(&self) -> f32 {
        self.outer_circle.diameter()
    }

    /// Get the thickness of the annulus
    #[inline(always)]
    pub fn thickness(&self) -> f32 {
        self.outer_circle.radius - self.inner_circle.radius
    }

    /// Get the area of the annulus
    #[inline(always)]
    pub fn area(&self) -> f32 {
        PI * (self.outer_circle.radius.powi(2) - self.inner_circle.radius.powi(2))
    }

    /// Get the perimeter or circumference of the annulus,
    /// which is the sum of the perimeters of the inner and outer circles.
    #[inline(always)]
    #[doc(alias = "circumference")]
    pub fn perimeter(&self) -> f32 {
        2.0 * PI * (self.outer_circle.radius + self.inner_circle.radius)
    }

    /// Finds the point on the annulus that is closest to the given `point`:
    ///
    /// - If the point is outside of the annulus completely, the returned point will be on the outer perimeter.
    /// - If the point is inside of the inner circle (hole) of the annulus, the returned point will be on the inner perimeter.
    /// - Otherwise, the returned point is overlapping the annulus and returned as is.
    #[inline(always)]
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        let distance_squared = point.length_squared();

        if self.inner_circle.radius.powi(2) <= distance_squared {
            if distance_squared <= self.outer_circle.radius.powi(2) {
                // The point is inside the annulus.
                point
            } else {
                // The point is outside the annulus and closer to the outer perimeter.
                // Find the closest point on the perimeter of the annulus.
                let dir_to_point = point / distance_squared.sqrt();
                self.outer_circle.radius * dir_to_point
            }
        } else {
            // The point is outside the annulus and closer to the inner perimeter.
            // Find the closest point on the perimeter of the annulus.
            let dir_to_point = point / distance_squared.sqrt();
            self.inner_circle.radius * dir_to_point
        }
    }
}

/// An unbounded plane in 2D space. It forms a separating surface through the origin,
/// stretching infinitely far
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Plane2d {
    /// The normal of the plane. The plane will be placed perpendicular to this direction
    pub normal: Dir2,
}
impl Primitive2d for Plane2d {}

impl Default for Plane2d {
    /// Returns the default [`Plane2d`] with a normal pointing in the `+Y` direction.
    fn default() -> Self {
        Self { normal: Dir2::Y }
    }
}

impl Plane2d {
    /// Create a new `Plane2d` from a normal
    ///
    /// # Panics
    ///
    /// Panics if the given `normal` is zero (or very close to zero), or non-finite.
    #[inline(always)]
    pub fn new(normal: Vec2) -> Self {
        Self {
            normal: Dir2::new(normal).expect("normal must be nonzero and finite"),
        }
    }
}

/// An infinite line along a direction in 2D space.
///
/// For a finite line: [`Segment2d`]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Line2d {
    /// The direction of the line. The line extends infinitely in both the given direction
    /// and its opposite direction
    pub direction: Dir2,
}
impl Primitive2d for Line2d {}

/// A segment of a line along a direction in 2D space.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "LineSegment2d")]
pub struct Segment2d {
    /// The direction of the line segment
    pub direction: Dir2,
    /// Half the length of the line segment. The segment extends by this amount in both
    /// the given direction and its opposite direction
    pub half_length: f32,
}
impl Primitive2d for Segment2d {}

impl Segment2d {
    /// Create a new `Segment2d` from a direction and full length of the segment
    #[inline(always)]
    pub fn new(direction: Dir2, length: f32) -> Self {
        Self {
            direction,
            half_length: length / 2.0,
        }
    }

    /// Create a new `Segment2d` from its endpoints and compute its geometric center
    ///
    /// # Panics
    ///
    /// Panics if `point1 == point2`
    #[inline(always)]
    pub fn from_points(point1: Vec2, point2: Vec2) -> (Self, Vec2) {
        let diff = point2 - point1;
        let length = diff.length();

        (
            // We are dividing by the length here, so the vector is normalized.
            Self::new(Dir2::new_unchecked(diff / length), length),
            (point1 + point2) / 2.,
        )
    }

    /// Get the position of the first point on the line segment
    #[inline(always)]
    pub fn point1(&self) -> Vec2 {
        *self.direction * -self.half_length
    }

    /// Get the position of the second point on the line segment
    #[inline(always)]
    pub fn point2(&self) -> Vec2 {
        *self.direction * self.half_length
    }
}

/// A series of connected line segments in 2D space.
///
/// For a version without generics: [`BoxedPolyline2d`]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Polyline2d<const N: usize> {
    /// The vertices of the polyline
    #[cfg_attr(feature = "serialize", serde(with = "super::serde::array"))]
    pub vertices: [Vec2; N],
}
impl<const N: usize> Primitive2d for Polyline2d<N> {}

impl<const N: usize> FromIterator<Vec2> for Polyline2d<N> {
    fn from_iter<I: IntoIterator<Item = Vec2>>(iter: I) -> Self {
        let mut vertices: [Vec2; N] = [Vec2::ZERO; N];

        for (index, i) in iter.into_iter().take(N).enumerate() {
            vertices[index] = i;
        }
        Self { vertices }
    }
}

impl<const N: usize> Polyline2d<N> {
    /// Create a new `Polyline2d` from its vertices
    pub fn new(vertices: impl IntoIterator<Item = Vec2>) -> Self {
        Self::from_iter(vertices)
    }
}

/// A series of connected line segments in 2D space, allocated on the heap
/// in a `Box<[Vec2]>`.
///
/// For a version without alloc: [`Polyline2d`]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct BoxedPolyline2d {
    /// The vertices of the polyline
    pub vertices: Box<[Vec2]>,
}
impl Primitive2d for BoxedPolyline2d {}

impl FromIterator<Vec2> for BoxedPolyline2d {
    fn from_iter<I: IntoIterator<Item = Vec2>>(iter: I) -> Self {
        let vertices: Vec<Vec2> = iter.into_iter().collect();
        Self {
            vertices: vertices.into_boxed_slice(),
        }
    }
}

impl BoxedPolyline2d {
    /// Create a new `BoxedPolyline2d` from its vertices
    pub fn new(vertices: impl IntoIterator<Item = Vec2>) -> Self {
        Self::from_iter(vertices)
    }
}

/// A triangle in 2D space
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Triangle2d {
    /// The vertices of the triangle
    pub vertices: [Vec2; 3],
}
impl Primitive2d for Triangle2d {}

impl Default for Triangle2d {
    /// Returns the default [`Triangle2d`] with the vertices `[0.0, 0.5]`, `[-0.5, -0.5]`, and `[0.5, -0.5]`.
    fn default() -> Self {
        Self {
            vertices: [Vec2::Y * 0.5, Vec2::new(-0.5, -0.5), Vec2::new(0.5, -0.5)],
        }
    }
}

impl Triangle2d {
    /// Create a new `Triangle2d` from points `a`, `b`, and `c`
    #[inline(always)]
    pub const fn new(a: Vec2, b: Vec2, c: Vec2) -> Self {
        Self {
            vertices: [a, b, c],
        }
    }

    /// Get the area of the triangle
    #[inline(always)]
    pub fn area(&self) -> f32 {
        let [a, b, c] = self.vertices;
        (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)).abs() / 2.0
    }

    /// Get the perimeter of the triangle
    #[inline(always)]
    pub fn perimeter(&self) -> f32 {
        let [a, b, c] = self.vertices;

        let ab = a.distance(b);
        let bc = b.distance(c);
        let ca = c.distance(a);

        ab + bc + ca
    }

    /// Get the [`WindingOrder`] of the triangle
    #[inline(always)]
    #[doc(alias = "orientation")]
    pub fn winding_order(&self) -> WindingOrder {
        let [a, b, c] = self.vertices;
        let area = (b - a).perp_dot(c - a);
        if area > f32::EPSILON {
            WindingOrder::CounterClockwise
        } else if area < -f32::EPSILON {
            WindingOrder::Clockwise
        } else {
            WindingOrder::Invalid
        }
    }

    /// Compute the circle passing through all three vertices of the triangle.
    /// The vector in the returned tuple is the circumcenter.
    pub fn circumcircle(&self) -> (Circle, Vec2) {
        // We treat the triangle as translated so that vertex A is at the origin. This simplifies calculations.
        //
        //     A = (0, 0)
        //        *
        //       / \
        //      /   \
        //     /     \
        //    /       \
        //   /    U    \
        //  /           \
        // *-------------*
        // B             C

        let a = self.vertices[0];
        let (b, c) = (self.vertices[1] - a, self.vertices[2] - a);
        let b_length_sq = b.length_squared();
        let c_length_sq = c.length_squared();

        // Reference: https://en.wikipedia.org/wiki/Circumcircle#Cartesian_coordinates_2
        let inv_d = (2.0 * (b.x * c.y - b.y * c.x)).recip();
        let ux = inv_d * (c.y * b_length_sq - b.y * c_length_sq);
        let uy = inv_d * (b.x * c_length_sq - c.x * b_length_sq);
        let u = Vec2::new(ux, uy);

        // Compute true circumcenter and circumradius, adding the tip coordinate so that
        // A is translated back to its actual coordinate.
        let center = u + a;
        let radius = u.length();

        (Circle { radius }, center)
    }

    /// Reverse the [`WindingOrder`] of the triangle
    /// by swapping the first and last vertices
    #[inline(always)]
    pub fn reverse(&mut self) {
        self.vertices.swap(0, 2);
    }
}

/// A rectangle primitive
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "Quad")]
pub struct Rectangle {
    /// Half of the width and height of the rectangle
    pub half_size: Vec2,
}
impl Primitive2d for Rectangle {}

impl Default for Rectangle {
    /// Returns the default [`Rectangle`] with a half-width and half-height of `0.5`.
    fn default() -> Self {
        Self {
            half_size: Vec2::splat(0.5),
        }
    }
}

impl Rectangle {
    /// Create a new `Rectangle` from a full width and height
    #[inline(always)]
    pub fn new(width: f32, height: f32) -> Self {
        Self::from_size(Vec2::new(width, height))
    }

    /// Create a new `Rectangle` from a given full size
    #[inline(always)]
    pub fn from_size(size: Vec2) -> Self {
        Self {
            half_size: size / 2.0,
        }
    }

    /// Create a new `Rectangle` from two corner points
    #[inline(always)]
    pub fn from_corners(point1: Vec2, point2: Vec2) -> Self {
        Self {
            half_size: (point2 - point1).abs() / 2.0,
        }
    }

    /// Create a `Rectangle` from a single length.
    /// The resulting `Rectangle` will be the same size in every direction.
    #[inline(always)]
    pub fn from_length(length: f32) -> Self {
        Self {
            half_size: Vec2::splat(length / 2.0),
        }
    }

    /// Get the size of the rectangle
    #[inline(always)]
    pub fn size(&self) -> Vec2 {
        2.0 * self.half_size
    }

    /// Get the area of the rectangle
    #[inline(always)]
    pub fn area(&self) -> f32 {
        4.0 * self.half_size.x * self.half_size.y
    }

    /// Get the perimeter of the rectangle
    #[inline(always)]
    pub fn perimeter(&self) -> f32 {
        4.0 * (self.half_size.x + self.half_size.y)
    }

    /// Finds the point on the rectangle that is closest to the given `point`.
    ///
    /// If the point is outside the rectangle, the returned point will be on the perimeter of the rectangle.
    /// Otherwise, it will be inside the rectangle and returned as is.
    #[inline(always)]
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        // Clamp point coordinates to the rectangle
        point.clamp(-self.half_size, self.half_size)
    }
}

/// A polygon with N vertices.
///
/// For a version without generics: [`BoxedPolygon`]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Polygon<const N: usize> {
    /// The vertices of the `Polygon`
    #[cfg_attr(feature = "serialize", serde(with = "super::serde::array"))]
    pub vertices: [Vec2; N],
}
impl<const N: usize> Primitive2d for Polygon<N> {}

impl<const N: usize> FromIterator<Vec2> for Polygon<N> {
    fn from_iter<I: IntoIterator<Item = Vec2>>(iter: I) -> Self {
        let mut vertices: [Vec2; N] = [Vec2::ZERO; N];

        for (index, i) in iter.into_iter().take(N).enumerate() {
            vertices[index] = i;
        }
        Self { vertices }
    }
}

impl<const N: usize> Polygon<N> {
    /// Create a new `Polygon` from its vertices
    pub fn new(vertices: impl IntoIterator<Item = Vec2>) -> Self {
        Self::from_iter(vertices)
    }
}

/// A polygon with a variable number of vertices, allocated on the heap
/// in a `Box<[Vec2]>`.
///
/// For a version without alloc: [`Polygon`]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct BoxedPolygon {
    /// The vertices of the `BoxedPolygon`
    pub vertices: Box<[Vec2]>,
}
impl Primitive2d for BoxedPolygon {}

impl FromIterator<Vec2> for BoxedPolygon {
    fn from_iter<I: IntoIterator<Item = Vec2>>(iter: I) -> Self {
        let vertices: Vec<Vec2> = iter.into_iter().collect();
        Self {
            vertices: vertices.into_boxed_slice(),
        }
    }
}

impl BoxedPolygon {
    /// Create a new `BoxedPolygon` from its vertices
    pub fn new(vertices: impl IntoIterator<Item = Vec2>) -> Self {
        Self::from_iter(vertices)
    }
}

/// A polygon where all vertices lie on a circle, equally far apart.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct RegularPolygon {
    /// The circumcircle on which all vertices lie
    pub circumcircle: Circle,
    /// The number of sides
    pub sides: usize,
}
impl Primitive2d for RegularPolygon {}

impl Default for RegularPolygon {
    /// Returns the default [`RegularPolygon`] with six sides (a hexagon) and a circumradius of `0.5`.
    fn default() -> Self {
        Self {
            circumcircle: Circle { radius: 0.5 },
            sides: 6,
        }
    }
}

impl RegularPolygon {
    /// Create a new `RegularPolygon`
    /// from the radius of the circumcircle and a number of sides
    ///
    /// # Panics
    ///
    /// Panics if `circumradius` is non-positive
    #[inline(always)]
    pub fn new(circumradius: f32, sides: usize) -> Self {
        assert!(circumradius > 0.0, "polygon has a non-positive radius");
        assert!(sides > 2, "polygon has less than 3 sides");

        Self {
            circumcircle: Circle {
                radius: circumradius,
            },
            sides,
        }
    }

    /// Get the radius of the circumcircle on which all vertices
    /// of the regular polygon lie
    #[inline(always)]
    pub fn circumradius(&self) -> f32 {
        self.circumcircle.radius
    }

    /// Get the inradius or apothem of the regular polygon.
    /// This is the radius of the largest circle that can
    /// be drawn within the polygon
    #[inline(always)]
    #[doc(alias = "apothem")]
    pub fn inradius(&self) -> f32 {
        self.circumradius() * (PI / self.sides as f32).cos()
    }

    /// Get the length of one side of the regular polygon
    #[inline(always)]
    pub fn side_length(&self) -> f32 {
        2.0 * self.circumradius() * (PI / self.sides as f32).sin()
    }

    /// Get the area of the regular polygon
    #[inline(always)]
    pub fn area(&self) -> f32 {
        let angle: f32 = 2.0 * PI / (self.sides as f32);
        (self.sides as f32) * self.circumradius().powi(2) * angle.sin() / 2.0
    }

    /// Get the perimeter of the regular polygon.
    /// This is the sum of its sides
    #[inline(always)]
    pub fn perimeter(&self) -> f32 {
        self.sides as f32 * self.side_length()
    }

    /// Get the internal angle of the regular polygon in degrees.
    ///
    /// This is the angle formed by two adjacent sides with points
    /// within the angle being in the interior of the polygon
    #[inline(always)]
    pub fn internal_angle_degrees(&self) -> f32 {
        (self.sides - 2) as f32 / self.sides as f32 * 180.0
    }

    /// Get the internal angle of the regular polygon in radians.
    ///
    /// This is the angle formed by two adjacent sides with points
    /// within the angle being in the interior of the polygon
    #[inline(always)]
    pub fn internal_angle_radians(&self) -> f32 {
        (self.sides - 2) as f32 * PI / self.sides as f32
    }

    /// Get the external angle of the regular polygon in degrees.
    ///
    /// This is the angle formed by two adjacent sides with points
    /// within the angle being in the exterior of the polygon
    #[inline(always)]
    pub fn external_angle_degrees(&self) -> f32 {
        360.0 / self.sides as f32
    }

    /// Get the external angle of the regular polygon in radians.
    ///
    /// This is the angle formed by two adjacent sides with points
    /// within the angle being in the exterior of the polygon
    #[inline(always)]
    pub fn external_angle_radians(&self) -> f32 {
        2.0 * PI / self.sides as f32
    }

    /// Returns an iterator over the vertices of the regular polygon,
    /// rotated counterclockwise by the given angle in radians.
    ///
    /// With a rotation of 0, a vertex will be placed at the top `(0.0, circumradius)`.
    pub fn vertices(self, rotation: f32) -> impl IntoIterator<Item = Vec2> {
        // Add pi/2 so that the polygon has a vertex at the top (sin is 1.0 and cos is 0.0)
        let start_angle = rotation + std::f32::consts::FRAC_PI_2;
        let step = std::f32::consts::TAU / self.sides as f32;

        (0..self.sides).map(move |i| {
            let theta = start_angle + i as f32 * step;
            let (sin, cos) = theta.sin_cos();
            Vec2::new(cos, sin) * self.circumcircle.radius
        })
    }
}

/// A 2D capsule primitive, also known as a stadium or pill shape.
///
/// A two-dimensional capsule is defined as a neighborhood of points at a distance (radius) from a line
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "stadium", alias = "pill")]
pub struct Capsule2d {
    /// The radius of the capsule
    pub radius: f32,
    /// Half the height of the capsule, excluding the hemicircles
    pub half_length: f32,
}
impl Primitive2d for Capsule2d {}

impl Default for Capsule2d {
    /// Returns the default [`Capsule2d`] with a radius of `0.5` and a half-height of `0.5`,
    /// excluding the hemicircles.
    fn default() -> Self {
        Self {
            radius: 0.5,
            half_length: 0.5,
        }
    }
}

impl Capsule2d {
    /// Create a new `Capsule2d` from a radius and length
    pub fn new(radius: f32, length: f32) -> Self {
        Self {
            radius,
            half_length: length / 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    // Reference values were computed by hand and/or with external tools

    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn rectangle_closest_point() {
        let rectangle = Rectangle::new(2.0, 2.0);
        assert_eq!(rectangle.closest_point(Vec2::X * 10.0), Vec2::X);
        assert_eq!(rectangle.closest_point(Vec2::NEG_ONE * 10.0), Vec2::NEG_ONE);
        assert_eq!(
            rectangle.closest_point(Vec2::new(0.25, 0.1)),
            Vec2::new(0.25, 0.1)
        );
    }

    #[test]
    fn circle_closest_point() {
        let circle = Circle { radius: 1.0 };
        assert_eq!(circle.closest_point(Vec2::X * 10.0), Vec2::X);
        assert_eq!(
            circle.closest_point(Vec2::NEG_ONE * 10.0),
            Vec2::NEG_ONE.normalize()
        );
        assert_eq!(
            circle.closest_point(Vec2::new(0.25, 0.1)),
            Vec2::new(0.25, 0.1)
        );
    }

    #[test]
    fn annulus_closest_point() {
        let annulus = Annulus::new(1.5, 2.0);
        assert_eq!(annulus.closest_point(Vec2::X * 10.0), Vec2::X * 2.0);
        assert_eq!(
            annulus.closest_point(Vec2::NEG_ONE),
            Vec2::NEG_ONE.normalize() * 1.5
        );
        assert_eq!(
            annulus.closest_point(Vec2::new(1.55, 0.85)),
            Vec2::new(1.55, 0.85)
        );
    }

    #[test]
    fn circle_math() {
        let circle = Circle { radius: 3.0 };
        assert_eq!(circle.diameter(), 6.0, "incorrect diameter");
        assert_eq!(circle.area(), 28.274334, "incorrect area");
        assert_eq!(circle.perimeter(), 18.849556, "incorrect perimeter");
    }

    #[test]
    fn annulus_math() {
        let annulus = Annulus::new(2.5, 3.5);
        assert_eq!(annulus.diameter(), 7.0, "incorrect diameter");
        assert_eq!(annulus.thickness(), 1.0, "incorrect thickness");
        assert_eq!(annulus.area(), 18.849556, "incorrect area");
        assert_eq!(annulus.perimeter(), 37.699112, "incorrect perimeter");
    }

    #[test]
    fn ellipse_math() {
        let ellipse = Ellipse::new(3.0, 1.0);
        assert_eq!(ellipse.area(), 9.424778, "incorrect area");
    }

    #[test]
    fn triangle_math() {
        let triangle = Triangle2d::new(
            Vec2::new(-2.0, -1.0),
            Vec2::new(1.0, 4.0),
            Vec2::new(7.0, 0.0),
        );
        assert_eq!(triangle.area(), 21.0, "incorrect area");
        assert_eq!(triangle.perimeter(), 22.097439, "incorrect perimeter");
    }

    #[test]
    fn triangle_winding_order() {
        let mut cw_triangle = Triangle2d::new(
            Vec2::new(0.0, 2.0),
            Vec2::new(-0.5, -1.2),
            Vec2::new(-1.0, -1.0),
        );
        assert_eq!(cw_triangle.winding_order(), WindingOrder::Clockwise);

        let ccw_triangle = Triangle2d::new(
            Vec2::new(-1.0, -1.0),
            Vec2::new(-0.5, -1.2),
            Vec2::new(0.0, 2.0),
        );
        assert_eq!(ccw_triangle.winding_order(), WindingOrder::CounterClockwise);

        // The clockwise triangle should be the same as the counterclockwise
        // triangle when reversed
        cw_triangle.reverse();
        assert_eq!(cw_triangle, ccw_triangle);

        let invalid_triangle = Triangle2d::new(
            Vec2::new(0.0, 2.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, -1.2),
        );
        assert_eq!(invalid_triangle.winding_order(), WindingOrder::Invalid);
    }

    #[test]
    fn rectangle_math() {
        let rectangle = Rectangle::new(3.0, 7.0);
        assert_eq!(
            rectangle,
            Rectangle::from_corners(Vec2::new(-1.5, -3.5), Vec2::new(1.5, 3.5))
        );
        assert_eq!(rectangle.area(), 21.0, "incorrect area");
        assert_eq!(rectangle.perimeter(), 20.0, "incorrect perimeter");
    }

    #[test]
    fn regular_polygon_math() {
        let polygon = RegularPolygon::new(3.0, 6);
        assert_eq!(polygon.inradius(), 2.598076, "incorrect inradius");
        assert_eq!(polygon.side_length(), 3.0, "incorrect side length");
        assert_relative_eq!(polygon.area(), 23.38268, epsilon = 0.00001);
        assert_eq!(polygon.perimeter(), 18.0, "incorrect perimeter");
        assert_eq!(
            polygon.internal_angle_degrees(),
            120.0,
            "incorrect internal angle"
        );
        assert_eq!(
            polygon.internal_angle_radians(),
            120_f32.to_radians(),
            "incorrect internal angle"
        );
        assert_eq!(
            polygon.external_angle_degrees(),
            60.0,
            "incorrect external angle"
        );
        assert_eq!(
            polygon.external_angle_radians(),
            60_f32.to_radians(),
            "incorrect external angle"
        );
    }

    #[test]
    fn triangle_circumcenter() {
        let triangle = Triangle2d::new(
            Vec2::new(10.0, 2.0),
            Vec2::new(-5.0, -3.0),
            Vec2::new(2.0, -1.0),
        );
        let (Circle { radius }, circumcenter) = triangle.circumcircle();

        // Calculated with external calculator
        assert_eq!(radius, 98.34887);
        assert_eq!(circumcenter, Vec2::new(-28.5, 92.5));
    }

    #[test]
    fn regular_polygon_vertices() {
        let polygon = RegularPolygon::new(1.0, 4);

        // Regular polygons have a vertex at the top by default
        let mut vertices = polygon.vertices(0.0).into_iter();
        assert!((vertices.next().unwrap() - Vec2::Y).length() < 1e-7);

        // Rotate by 45 degrees, forming an axis-aligned square
        let mut rotated_vertices = polygon.vertices(std::f32::consts::FRAC_PI_4).into_iter();

        // Distance from the origin to the middle of a side, derived using Pythagorean theorem
        let side_sistance = std::f32::consts::FRAC_1_SQRT_2;
        assert!(
            (rotated_vertices.next().unwrap() - Vec2::new(-side_sistance, side_sistance)).length()
                < 1e-7,
        );
    }
}
