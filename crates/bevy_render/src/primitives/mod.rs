use bevy_ecs::{component::Component, prelude::Entity, reflect::ReflectComponent};
use bevy_math::{Mat4, Vec3, Vec3A, Vec4, Vec4Swizzles};
use bevy_reflect::{FromReflect, Reflect};
use bevy_utils::HashMap;

/// An axis-aligned bounding box.
#[derive(Component, Clone, Copy, Debug, Default, Reflect, FromReflect)]
#[reflect(Component)]
pub struct Aabb {
    pub center: Vec3A,
    pub half_extents: Vec3A,
}

impl Aabb {
    #[inline]
    pub fn from_min_max(minimum: Vec3, maximum: Vec3) -> Self {
        let minimum = Vec3A::from(minimum);
        let maximum = Vec3A::from(maximum);
        let center = 0.5 * (maximum + minimum);
        let half_extents = 0.5 * (maximum - minimum);
        Self {
            center,
            half_extents,
        }
    }

    /// Calculate the relative radius of the AABB with respect to a plane
    #[inline]
    pub fn relative_radius(&self, p_normal: &Vec3A, axes: &[Vec3A]) -> f32 {
        // NOTE: dot products on Vec3A use SIMD and even with the overhead of conversion are net faster than Vec3
        let half_extents = self.half_extents;
        Vec3A::new(
            p_normal.dot(axes[0]),
            p_normal.dot(axes[1]),
            p_normal.dot(axes[2]),
        )
        .abs()
        .dot(half_extents)
    }

    #[inline]
    pub fn min(&self) -> Vec3A {
        self.center - self.half_extents
    }

    #[inline]
    pub fn max(&self) -> Vec3A {
        self.center + self.half_extents
    }
}

impl From<Sphere> for Aabb {
    #[inline]
    fn from(sphere: Sphere) -> Self {
        Self {
            center: sphere.center,
            half_extents: Vec3A::splat(sphere.radius),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Sphere {
    pub center: Vec3A,
    pub radius: f32,
}

impl Sphere {
    #[inline]
    pub fn intersects_obb(&self, aabb: &Aabb, local_to_world: &Mat4) -> bool {
        let aabb_center_world = *local_to_world * aabb.center.extend(1.0);
        let axes = [
            Vec3A::from(local_to_world.x_axis),
            Vec3A::from(local_to_world.y_axis),
            Vec3A::from(local_to_world.z_axis),
        ];
        let v = Vec3A::from(aabb_center_world) - self.center;
        let d = v.length();
        let relative_radius = aabb.relative_radius(&(v / d), &axes);
        d < self.radius + relative_radius
    }
}

/// A plane defined by a unit normal and distance from the origin along the normal
/// Any point `p` is in the plane if `n.p + d = 0`
/// For planes defining half-spaces such as for frusta, if `n.p + d > 0` then `p` is on
/// the positive side (inside) of the plane.
#[derive(Clone, Copy, Debug, Default)]
pub struct Plane {
    normal_d: Vec4,
}

impl Plane {
    /// Constructs a `Plane` from a 4D vector whose first 3 components
    /// are the normal and whose last component is the distance along the normal
    /// from the origin.
    /// This constructor ensures that the normal is normalized and the distance is
    /// scaled accordingly so it represents the signed distance from the origin.
    #[inline]
    pub fn new(normal_d: Vec4) -> Self {
        Self {
            normal_d: normal_d * normal_d.xyz().length_recip(),
        }
    }

    /// `Plane` unit normal
    #[inline]
    pub fn normal(&self) -> Vec3A {
        Vec3A::from(self.normal_d)
    }

    /// Signed distance from the origin along the unit normal such that n.p + d = 0 for point p in
    /// the `Plane`
    #[inline]
    pub fn d(&self) -> f32 {
        self.normal_d.w
    }

    /// `Plane` unit normal and signed distance from the origin such that n.p + d = 0 for point p
    /// in the `Plane`
    #[inline]
    pub fn normal_d(&self) -> Vec4 {
        self.normal_d
    }
}

/// A frustum defined by the 6 containing planes
/// Planes are ordered left, right, top, bottom, near, far
/// Normals point into the contained volume
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Frustum {
    #[reflect(ignore)]
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Returns a frustum derived from `view_projection`.
    #[inline]
    pub fn from_view_projection(view_projection: &Mat4) -> Self {
        let mut frustum = Frustum::from_view_projection_no_far(view_projection);
        frustum.planes[5] = Plane::new(view_projection.row(2));
        frustum
    }

    /// Returns a frustum derived from `view_projection`, but with a custom
    /// far plane.
    #[inline]
    pub fn from_view_projection_custom_far(
        view_projection: &Mat4,
        view_translation: &Vec3,
        view_backward: &Vec3,
        far: f32,
    ) -> Self {
        let mut frustum = Frustum::from_view_projection_no_far(view_projection);
        let far_center = *view_translation - far * *view_backward;
        frustum.planes[5] = Plane::new(view_backward.extend(-view_backward.dot(far_center)));
        frustum
    }

    // NOTE: This approach of extracting the frustum planes from the view
    // projection matrix is from Foundations of Game Engine Development 2
    // Rendering by Lengyel.
    fn from_view_projection_no_far(view_projection: &Mat4) -> Self {
        let row3 = view_projection.row(3);
        let mut planes = [Plane::default(); 6];
        for (i, plane) in planes.iter_mut().enumerate().take(5) {
            let row = view_projection.row(i / 2);
            *plane = Plane::new(if (i & 1) == 0 && i != 4 {
                row3 + row
            } else {
                row3 - row
            });
        }
        Self { planes }
    }

    #[inline]
    pub fn intersects_sphere(&self, sphere: &Sphere, intersect_far: bool) -> bool {
        let sphere_center = sphere.center.extend(1.0);
        let max = if intersect_far { 6 } else { 5 };
        for plane in &self.planes[..max] {
            if plane.normal_d().dot(sphere_center) + sphere.radius <= 0.0 {
                return false;
            }
        }
        true
    }

    #[inline]
    pub fn intersects_obb(
        &self,
        aabb: &Aabb,
        model_to_world: &Mat4,
        intersect_near: bool,
        intersect_far: bool,
    ) -> bool {
        let aabb_center_world = model_to_world.transform_point3a(aabb.center).extend(1.0);
        let axes = [
            Vec3A::from(model_to_world.x_axis),
            Vec3A::from(model_to_world.y_axis),
            Vec3A::from(model_to_world.z_axis),
        ];

        for (idx, plane) in self.planes.into_iter().enumerate() {
            if idx == 4 && !intersect_near {
                continue;
            }
            if idx == 5 && !intersect_far {
                continue;
            }
            let p_normal = Vec3A::from(plane.normal_d());
            let relative_radius = aabb.relative_radius(&p_normal, &axes);
            if plane.normal_d().dot(aabb_center_world) + relative_radius <= 0.0 {
                return false;
            }
        }
        true
    }
}

#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct CubemapFrusta {
    #[reflect(ignore)]
    pub frusta: [Frustum; 6],
}

impl CubemapFrusta {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Frustum> {
        self.frusta.iter()
    }
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut Frustum> {
        self.frusta.iter_mut()
    }
}

#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct CascadesFrusta {
    #[reflect(ignore)]
    pub frusta: HashMap<Entity, Vec<Frustum>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // A big, offset frustum
    fn big_frustum() -> Frustum {
        Frustum {
            planes: [
                Plane::new(Vec4::new(-0.9701, -0.2425, -0.0000, 7.7611)),
                Plane::new(Vec4::new(-0.0000, 1.0000, -0.0000, 4.0000)),
                Plane::new(Vec4::new(-0.0000, -0.2425, -0.9701, 2.9104)),
                Plane::new(Vec4::new(-0.0000, -1.0000, -0.0000, 4.0000)),
                Plane::new(Vec4::new(-0.0000, -0.2425, 0.9701, 2.9104)),
                Plane::new(Vec4::new(0.9701, -0.2425, -0.0000, -1.9403)),
            ],
        }
    }

    #[test]
    fn intersects_sphere_big_frustum_outside() {
        // Sphere outside frustum
        let frustum = big_frustum();
        let sphere = Sphere {
            center: Vec3A::new(0.9167, 0.0000, 0.0000),
            radius: 0.7500,
        };
        assert!(!frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_big_frustum_intersect() {
        // Sphere intersects frustum boundary
        let frustum = big_frustum();
        let sphere = Sphere {
            center: Vec3A::new(7.9288, 0.0000, 2.9728),
            radius: 2.0000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    // A frustum
    fn frustum() -> Frustum {
        Frustum {
            planes: [
                Plane::new(Vec4::new(-0.9701, -0.2425, -0.0000, 0.7276)),
                Plane::new(Vec4::new(-0.0000, 1.0000, -0.0000, 1.0000)),
                Plane::new(Vec4::new(-0.0000, -0.2425, -0.9701, 0.7276)),
                Plane::new(Vec4::new(-0.0000, -1.0000, -0.0000, 1.0000)),
                Plane::new(Vec4::new(-0.0000, -0.2425, 0.9701, 0.7276)),
                Plane::new(Vec4::new(0.9701, -0.2425, -0.0000, 0.7276)),
            ],
        }
    }

    #[test]
    fn intersects_sphere_frustum_surrounding() {
        // Sphere surrounds frustum
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(0.0000, 0.0000, 0.0000),
            radius: 3.0000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_frustum_contained() {
        // Sphere is contained in frustum
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(0.0000, 0.0000, 0.0000),
            radius: 0.7000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_frustum_intersects_plane() {
        // Sphere intersects a plane
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(0.0000, 0.0000, 0.9695),
            radius: 0.7000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_frustum_intersects_2_planes() {
        // Sphere intersects 2 planes
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(1.2037, 0.0000, 0.9695),
            radius: 0.7000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_frustum_intersects_3_planes() {
        // Sphere intersects 3 planes
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(1.2037, -1.0988, 0.9695),
            radius: 0.7000,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_frustum_dodges_1_plane() {
        // Sphere avoids intersecting the frustum by 1 plane
        let frustum = frustum();
        let sphere = Sphere {
            center: Vec3A::new(-1.7020, 0.0000, 0.0000),
            radius: 0.7000,
        };
        assert!(!frustum.intersects_sphere(&sphere, true));
    }

    // A long frustum.
    fn long_frustum() -> Frustum {
        Frustum {
            planes: [
                Plane::new(Vec4::new(-0.9998, -0.0222, -0.0000, -1.9543)),
                Plane::new(Vec4::new(-0.0000, 1.0000, -0.0000, 45.1249)),
                Plane::new(Vec4::new(-0.0000, -0.0168, -0.9999, 2.2718)),
                Plane::new(Vec4::new(-0.0000, -1.0000, -0.0000, 45.1249)),
                Plane::new(Vec4::new(-0.0000, -0.0168, 0.9999, 2.2718)),
                Plane::new(Vec4::new(0.9998, -0.0222, -0.0000, 7.9528)),
            ],
        }
    }

    #[test]
    fn intersects_sphere_long_frustum_outside() {
        // Sphere outside frustum
        let frustum = long_frustum();
        let sphere = Sphere {
            center: Vec3A::new(-4.4889, 46.9021, 0.0000),
            radius: 0.7500,
        };
        assert!(!frustum.intersects_sphere(&sphere, true));
    }

    #[test]
    fn intersects_sphere_long_frustum_intersect() {
        // Sphere intersects frustum boundary
        let frustum = long_frustum();
        let sphere = Sphere {
            center: Vec3A::new(-4.9957, 0.0000, -0.7396),
            radius: 4.4094,
        };
        assert!(frustum.intersects_sphere(&sphere, true));
    }
}
