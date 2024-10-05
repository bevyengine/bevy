mod annulus;
mod arc;
mod capsule;
mod circle;
mod circular_sector;
mod circular_segment;
mod ellipse;
mod line;
mod polygon;
mod polyline;
mod rectangle;
mod rhombus;
mod segment;
mod triangle;

use crate::{Dir2, Isometry2d, Ray2d};
#[cfg(all(feature = "bevy_reflect", feature = "serialize"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// An intersection between a ray and a shape in two-dimensional space.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "bevy_reflect", feature = "serialize"),
    reflect(Serialize, Deserialize)
)]
pub struct RayHit2d {
    /// The distance between the point of intersection and the ray origin.
    pub distance: f32,
    /// The surface normal on the shape at the point of intersection.
    pub normal: Dir2,
}

impl RayHit2d {
    /// Creates a new [`RayHit2d`] from the given distance and surface normal at the point of intersection.
    #[inline]
    pub const fn new(distance: f32, normal: Dir2) -> Self {
        Self { distance, normal }
    }
}

/// A trait for intersecting rays with [primitive shapes] in two-dimensional space.
///
/// [primitive shapes]: crate::primitives
pub trait PrimitiveRayCast2d {
    /// Computes the distance to the closest intersection along the given `ray`, expressed in the local space of `self`.
    /// Returns `None` if no intersection is found or if the distance exceeds the given `max_distance`.
    ///
    /// `solid` determines whether the shape should be treated as solid or hollow when the ray origin is in the interior
    /// of the shape. If `solid` is `true`, the distance of the hit will be `Some(0.0)`. Otherwise, the ray will travel
    /// until it hits the boundary, and compute the corresponding distance.
    ///
    /// # Example
    ///
    /// Casting a ray against a solid circle might look like this:
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// #
    /// let ray = Ray2d::new(Vec2::new(-2.0, 0.0), Vec2::X);
    /// let circle = Circle::new(1.0);
    ///
    /// let max_distance = f32::MAX;
    /// let solid = true;
    ///
    /// if let Some(distance) = circle.local_ray_distance(ray, max_distance, solid) {
    ///     // The ray intersects the circle at a distance of 1.0.
    ///     assert_eq!(distance, 1.0);
    ///
    ///     // The point of intersection can be computed using the distance along the ray:
    ///     let point = ray.get_point(distance);
    ///     assert_eq!(point, Vec2::new(-1.0, 0.0));
    /// }
    /// ```
    ///
    /// If the ray origin is inside of a solid shape, the hit distance will be `0.0`.
    /// This could be used to ignore intersections where the ray starts from inside of the shape.
    ///
    /// If the ray origin is instead inside of a hollow shape, the point of intersection
    /// will be at the boundary of the shape:
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// #
    /// let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
    /// let circle = Circle::new(1.0);
    ///
    /// let max_distance = f32::MAX;
    /// let solid = false;
    ///
    /// if let Some(distance) = circle.local_ray_distance(ray, max_distance, solid) {
    ///     // The ray origin is inside of the hollow circle, and hit its boundary.
    ///     assert_eq!(distance, circle.radius);
    ///     assert_eq!(ray.get_point(distance), Vec2::new(1.0, 0.0));
    /// }
    /// ```
    #[inline]
    fn local_ray_distance(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<f32> {
        self.local_ray_cast(ray, max_distance, solid)
            .map(|hit| hit.distance)
    }

    /// Computes the closest intersection along the given `ray`, expressed in the local space of `self`.
    /// Returns `None` if no intersection is found or if the distance exceeds the given `max_distance`.
    ///
    /// `solid` determines whether the shape should be treated as solid or hollow when the ray origin is in the interior
    /// of the shape. If `solid` is `true`, the distance of the hit will be `Some(0.0)`. Otherwise, the ray will travel
    /// until it hits the boundary, and compute the corresponding intersection.
    ///
    /// # Example
    ///
    /// Casting a ray against a solid circle might look like this:
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// #
    /// let ray = Ray2d::new(Vec2::new(-2.0, 0.0), Vec2::X);
    /// let circle = Circle::new(1.0);
    ///
    /// let max_distance = f32::MAX;
    /// let solid = true;
    ///
    /// if let Some(hit) = circle.local_ray_cast(ray, max_distance, solid) {
    ///     // The ray intersects the circle at a distance of 1.0.
    ///     // The hit normal at the point of intersection is -X.
    ///     assert_eq!(hit.distance, 1.0);
    ///     assert_eq!(hit.normal, Dir2::NEG_X);
    ///
    ///     // The point of intersection can be computed using the distance along the ray:
    ///     let point = ray.get_point(hit.distance);
    ///     assert_eq!(point, Vec2::new(-1.0, 0.0));
    /// }
    /// ```
    ///
    /// If the ray origin is inside of a solid shape, the hit distance will be `0.0`.
    /// This could be used to ignore intersections where the ray starts from inside of the shape.
    ///
    /// If the ray origin is instead inside of a hollow shape, the point of intersection
    /// will be at the boundary of the shape:
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// #
    /// let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
    /// let circle = Circle::new(1.0);
    ///
    /// let max_distance = f32::MAX;
    /// let solid = false;
    ///
    /// if let Some(hit) = circle.local_ray_cast(ray, max_distance, solid) {
    ///     // The ray origin is inside of the hollow circle, and hit its boundary.
    ///     assert_eq!(hit.distance, circle.radius);
    ///     assert_eq!(hit.normal, Dir2::NEG_X);
    ///     assert_eq!(ray.get_point(hit.distance), Vec2::new(1.0, 0.0));
    /// }
    /// ```
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d>;

    /// Returns `true` if `self` intersects the given `ray` in the local space of `self`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// #
    /// // Define a circle with a radius of `1.0` centered at the origin.
    /// let circle = Circle::new(1.0);
    ///
    /// // Test for ray intersections.
    /// assert!(circle.intersects_local_ray(Ray2d::new(Vec2::new(-2.0, 0.0), Vec2::X)));
    /// assert!(!circle.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 2.0), Vec2::X)));
    /// ```
    #[inline]
    fn intersects_local_ray(&self, ray: Ray2d) -> bool {
        self.local_ray_distance(ray, f32::MAX, true).is_some()
    }

    /// Computes the distance to the closest intersection along the given `ray` for `self` transformed by `iso`.
    /// Returns `None` if no intersection is found or if the distance exceeds the given `max_distance`.
    ///
    /// `solid` determines whether the shape should be treated as solid or hollow when the ray origin is in the interior
    /// of the shape. If `solid` is `true`, the distance of the hit will be `Some(0.0)`. Otherwise, the ray will travel
    /// until it hits the boundary, and compute the corresponding distance.
    ///
    /// # Example
    ///
    /// Casting a ray against a solid circle might look like this:
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// #
    /// let ray = Ray2d::new(Vec2::new(-1.0, 0.0), Vec2::X);
    /// let circle = Circle::new(1.0);
    /// let iso = Isometry2d::from_translation(Vec2::new(1.0, 0.0));
    ///
    /// let max_distance = f32::MAX;
    /// let solid = true;
    ///
    /// if let Some(distance) = circle.ray_distance(iso, ray, max_distance, solid) {
    ///     // The ray intersects the circle at a distance of 1.0.
    ///     assert_eq!(distance, 1.0);
    ///
    ///     // The point of intersection can be computed using the distance along the ray:
    ///     let point = ray.get_point(distance);
    ///     assert_eq!(point, Vec2::ZERO);
    /// }
    /// ```
    ///
    /// If the ray origin is inside of a solid shape, the hit distance will be `0.0`.
    /// This could be used to ignore intersections where the ray starts from inside of the shape.
    ///
    /// If the ray origin is instead inside of a hollow shape, the point of intersection
    /// will be at the boundary of the shape:
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// #
    /// let ray = Ray2d::new(Vec2::new(1.0, 0.0), Vec2::X);
    /// let circle = Circle::new(1.0);
    /// let iso = Isometry2d::from_translation(Vec2::new(1.0, 0.0));
    ///
    /// let max_distance = f32::MAX;
    /// let solid = false;
    ///
    /// if let Some(distance) = circle.ray_distance(iso, ray, max_distance, solid) {
    ///     // The ray origin is inside of the hollow circle, and hit its boundary.
    ///     assert_eq!(distance, circle.radius);
    ///     assert_eq!(ray.get_point(distance), Vec2::new(2.0, 0.0));
    /// }
    /// ```
    #[inline]
    fn ray_distance(
        &self,
        iso: Isometry2d,
        ray: Ray2d,
        max_distance: f32,
        solid: bool,
    ) -> Option<f32> {
        let local_ray = ray.inverse_transformed_by(iso);
        self.local_ray_distance(local_ray, max_distance, solid)
    }

    /// Computes the closest intersection along the given `ray` for `self` transformed by `iso`.
    /// Returns `None` if no intersection is found or if the distance exceeds the given `max_distance`.
    ///
    /// `solid` determines whether the shape should be treated as solid or hollow when the ray origin is in the interior
    /// of the shape. If `solid` is `true`, the distance of the hit will be `Some(0.0)`. Otherwise, the ray will travel
    /// until it hits the boundary, and compute the corresponding intersection.
    ///
    /// # Example
    ///
    /// Casting a ray against a solid circle might look like this:
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// #
    /// let ray = Ray2d::new(Vec2::new(-1.0, 0.0), Vec2::X);
    /// let circle = Circle::new(1.0);
    /// let iso = Isometry2d::from_translation(Vec2::new(1.0, 0.0));
    ///
    /// let max_distance = f32::MAX;
    /// let solid = true;
    ///
    /// if let Some(hit) = circle.ray_cast(iso, ray, max_distance, solid) {
    ///     // The ray intersects the circle at a distance of 1.0.
    ///     // The hit normal at the point of intersection is -X.
    ///     assert_eq!(hit.distance, 1.0);
    ///     assert_eq!(hit.normal, Dir2::NEG_X);
    ///
    ///     // The point of intersection can be computed using the distance along the ray:
    ///     let point = ray.get_point(hit.distance);
    ///     assert_eq!(point, Vec2::ZERO);
    /// }
    /// ```
    ///
    /// If the ray origin is inside of a solid shape, the hit distance will be `0.0`.
    /// This could be used to ignore intersections where the ray starts from inside of the shape.
    ///
    /// If the ray origin is instead inside of a hollow shape, the point of intersection
    /// will be at the boundary of the shape:
    ///
    /// ```
    /// # use bevy_math::prelude::*;
    /// #
    /// let ray = Ray2d::new(Vec2::new(1.0, 0.0), Vec2::X);
    /// let circle = Circle::new(1.0);
    /// let iso = Isometry2d::from_translation(Vec2::new(1.0, 0.0));
    ///
    /// let max_distance = f32::MAX;
    /// let solid = false;
    ///
    /// if let Some(hit) = circle.ray_cast(iso, ray, max_distance, solid) {
    ///     // The ray origin is inside of the hollow circle, and hit its boundary.
    ///     assert_eq!(hit.distance, circle.radius);
    ///     assert_eq!(hit.normal, Dir2::NEG_X);
    ///     assert_eq!(ray.get_point(hit.distance), Vec2::new(2.0, 0.0));
    /// }
    /// ```
    #[inline]
    fn ray_cast(
        &self,
        iso: Isometry2d,
        ray: Ray2d,
        max_distance: f32,
        solid: bool,
    ) -> Option<RayHit2d> {
        let local_ray = ray.inverse_transformed_by(iso);
        self.local_ray_cast(local_ray, max_distance, solid)
            .map(|mut hit| {
                hit.normal = iso.rotation * hit.normal;
                hit
            })
    }

    /// Returns `true` if `self` transformed by `iso` intersects the given `ray`.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_math::prelude::*;
    ///
    /// // Define a circle with a radius of `1.0` shifted by `1.0` along the X axis.
    /// let circle = Circle::new(1.0);
    /// let iso = Isometry2d::from_translation(Vec2::new(1.0, 0.0));
    ///
    /// // Test for ray intersections.
    /// assert!(circle.intersects_ray(iso, Ray2d::new(Vec2::new(-2.0, 0.0), Vec2::X)));
    /// assert!(!circle.intersects_ray(iso, Ray2d::new(Vec2::new(0.0, 2.0), Vec2::X)));
    /// ```
    #[inline]
    fn intersects_ray(&self, iso: Isometry2d, ray: Ray2d) -> bool {
        self.ray_distance(iso, ray, f32::MAX, true).is_some()
    }
}

#[cfg(test)]
mod tests {
    use core::f32::consts::SQRT_2;

    use crate::prelude::*;
    use approx::assert_relative_eq;

    #[test]
    fn ray_cast_2d() {
        let rectangle = Rectangle::new(2.0, 1.0);
        let iso = Isometry2d::new(Vec2::new(2.0, 0.0), Rot2::degrees(45.0));

        // Cast a ray on the transformed rectangle.
        let ray = Ray2d::new(Vec2::new(-1.0, SQRT_2 / 2.0), Vec2::X);
        let hit = rectangle.ray_cast(iso, ray, f32::MAX, true).unwrap();

        assert_relative_eq!(hit.distance, 3.0);
        assert_eq!(hit.normal, Dir2::NORTH_WEST);
    }
}
