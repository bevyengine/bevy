use bevy_math::{Vec3, Vec3A};
use bevy_reflect::Reflect;

pub use rays::*;

#[derive(Debug, Clone, Reflect)]
pub struct IntersectionData {
    position: Vec3,
    normal: Vec3,
    barycentric_coord: Vec3,
    distance: f32,
    triangle: Option<[Vec3A; 3]>,
    triangle_index: Option<usize>,
}

impl From<rays::PrimitiveIntersection> for IntersectionData {
    fn from(data: rays::PrimitiveIntersection) -> Self {
        Self {
            position: data.position(),
            normal: data.normal(),
            distance: data.distance(),
            barycentric_coord: Vec3::ZERO,
            triangle: None,
            triangle_index: None,
        }
    }
}

impl IntersectionData {
    pub fn new(
        position: Vec3,
        normal: Vec3,
        barycentric: Vec3,
        distance: f32,
        triangle: Option<[Vec3A; 3]>,
        triangle_index: Option<usize>,
    ) -> Self {
        Self {
            position,
            normal,
            barycentric_coord: barycentric,
            distance,
            triangle,
            triangle_index,
        }
    }

    /// Get the intersection data's position.
    #[must_use]
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Get the intersection data's normal.
    #[must_use]
    pub fn normal(&self) -> Vec3 {
        self.normal
    }

    /// Get the intersection data's barycentric coord.
    #[must_use]
    pub fn barycentric_coord(&self) -> Vec3 {
        self.barycentric_coord
    }

    /// Get the intersection data's distance.
    #[must_use]
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// Get the intersection data's triangle.
    #[must_use]
    pub fn triangle(&self) -> Option<[Vec3A; 3]> {
        self.triangle
    }

    /// Get the intersection data's triangle index.
    #[must_use]
    pub fn triangle_index(&self) -> Option<usize> {
        self.triangle_index
    }
}

/// Encapsulates Ray3D, preventing use of struct literal syntax. This allows us to guarantee that
/// the `Ray3d` direction is normalized, because it can only be instantiated with the constructor.
pub mod rays {
    use bevy_math::{prelude::*, Ray3d, Vec3A};
    use bevy_render::primitives::Aabb;

    pub struct PrimitiveIntersection {
        position: Vec3,
        normal: Vec3,
        distance: f32,
    }

    impl PrimitiveIntersection {
        pub fn new(position: Vec3, normal: Vec3, distance: f32) -> Self {
            Self {
                position,
                normal,
                distance,
            }
        }

        /// Get the intersection's position
        #[must_use]
        pub fn position(&self) -> Vec3 {
            self.position
        }

        /// Get the normal vector of the primitive at the point of intersection
        #[must_use]
        pub fn normal(&self) -> Vec3 {
            self.normal
        }

        /// Get the distance between the ray origin and the intersection position
        #[must_use]
        pub fn distance(&self) -> f32 {
            self.distance
        }
    }

    pub fn to_transform(ray: Ray3d) -> Mat4 {
        to_aligned_transform(ray, [0., 1., 0.].into())
    }

    /// Create a transform whose origin is at the origin of the ray and
    /// whose up-axis is aligned with the direction of the ray. Use `up` to
    /// specify which axis of the transform should align with the ray.
    pub fn to_aligned_transform(ray: Ray3d, up: Vec3) -> Mat4 {
        let position = ray.origin;
        let normal = ray.direction;
        let new_rotation = Quat::from_rotation_arc(up, *normal);
        Mat4::from_rotation_translation(new_rotation, position)
    }

    pub fn ray_from_transform(transform: Mat4) -> Ray3d {
        let pick_position_ndc = Vec3::from([0.0, 0.0, -1.0]);
        let pick_position = transform.project_point3(pick_position_ndc);
        let (_, _, source_origin) = transform.to_scale_rotation_translation();
        let ray_direction = pick_position - source_origin;
        Ray3d::new(source_origin, ray_direction)
    }

    /// Checks if the ray intersects with an AABB of a mesh, returning `[near, far]` if it does.
    pub fn intersects_aabb(ray: Ray3d, aabb: &Aabb, model_to_world: &Mat4) -> Option<[f32; 2]> {
        // Transform the ray to model space
        let world_to_model = model_to_world.inverse();
        let ray_dir: Vec3A = world_to_model.transform_vector3(*ray.direction).into();
        let ray_origin: Vec3A = world_to_model.transform_point3(ray.origin).into();
        // Check if the ray intersects the mesh's AABB. It's useful to work in model space
        // because we can do an AABB intersection test, instead of an OBB intersection test.

        let t_0: Vec3A = (aabb.min() - ray_origin) / ray_dir;
        let t_1: Vec3A = (aabb.max() - ray_origin) / ray_dir;
        let t_min: Vec3A = t_0.min(t_1);
        let t_max: Vec3A = t_0.max(t_1);

        let mut hit_near = t_min.x;
        let mut hit_far = t_max.x;

        if hit_near > t_max.y || t_min.y > hit_far {
            return None;
        }

        if t_min.y > hit_near {
            hit_near = t_min.y;
        }
        if t_max.y < hit_far {
            hit_far = t_max.y;
        }

        if (hit_near > t_max.z) || (t_min.z > hit_far) {
            return None;
        }

        if t_min.z > hit_near {
            hit_near = t_min.z;
        }
        if t_max.z < hit_far {
            hit_far = t_max.z;
        }
        Some([hit_near, hit_far])
    }
}
