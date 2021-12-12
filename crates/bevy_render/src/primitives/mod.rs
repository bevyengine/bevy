use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{Mat4, Vec3, Vec3A, Vec4};
use bevy_reflect::Reflect;

/// An Axis-Aligned Bounding Box
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Aabb {
    pub center: Vec3,
    pub half_extents: Vec3,
}

impl Aabb {
    pub fn from_min_max(minimum: Vec3, maximum: Vec3) -> Self {
        let center = 0.5 * (maximum + minimum);
        let half_extents = 0.5 * (maximum - minimum);
        Self {
            center,
            half_extents,
        }
    }

    /// Calculate the relative radius of the AABB with respect to a plane
    pub fn relative_radius(&self, p_normal: &Vec3A, axes: &[Vec3A]) -> f32 {
        // NOTE: dot products on Vec3A use SIMD and even with the overhead of conversion are net faster than Vec3
        let half_extents = Vec3A::from(self.half_extents);
        Vec3A::new(
            p_normal.dot(axes[0]),
            p_normal.dot(axes[1]),
            p_normal.dot(axes[2]),
        )
        .abs()
        .dot(half_extents)
    }

    pub fn min(&self) -> Vec3 {
        self.center - self.half_extents
    }

    pub fn max(&self) -> Vec3 {
        self.center + self.half_extents
    }
}

impl From<Sphere> for Aabb {
    fn from(sphere: Sphere) -> Self {
        Self {
            center: sphere.center,
            half_extents: Vec3::splat(sphere.radius),
        }
    }
}

#[derive(Debug, Default)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn intersects_obb(&self, aabb: &Aabb, local_to_world: &Mat4) -> bool {
        let aabb_center_world = *local_to_world * aabb.center.extend(1.0);
        let axes = [
            Vec3A::from(local_to_world.x_axis),
            Vec3A::from(local_to_world.y_axis),
            Vec3A::from(local_to_world.z_axis),
        ];
        let v = Vec3A::from(aabb_center_world) - Vec3A::from(self.center);
        let d = v.length();
        let relative_radius = aabb.relative_radius(&(v / d), &axes);
        d < self.radius + relative_radius
    }
}

/// A plane defined by a normal and distance value along the normal
/// Any point p is in the plane if n.p = d
/// For planes defining half-spaces such as for frusta, if n.p > d then p is on the positive side of the plane.
#[derive(Clone, Copy, Debug, Default)]
pub struct Plane {
    pub normal_d: Vec4,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    // NOTE: This approach of extracting the frustum planes from the view
    // projection matrix is from Foundations of Game Engine Development 2
    // Rendering by Lengyel. Slight modification has been made for when
    // the far plane is infinite but we still want to cull to a far plane.
    pub fn from_view_projection(
        view_projection: &Mat4,
        view_translation: &Vec3,
        view_backward: &Vec3,
        far: f32,
    ) -> Self {
        let row3 = view_projection.row(3);
        let mut planes = [Plane::default(); 6];
        for (i, plane) in planes.iter_mut().enumerate().take(5) {
            let row = view_projection.row(i / 2);
            plane.normal_d = if (i & 1) == 0 && i != 4 {
                row3 + row
            } else {
                row3 - row
            }
            .normalize();
        }
        let far_center = *view_translation - far * *view_backward;
        planes[5].normal_d = view_backward
            .extend(-view_backward.dot(far_center))
            .normalize();
        Self { planes }
    }

    pub fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        for plane in &self.planes {
            if plane.normal_d.dot(sphere.center.extend(1.0)) + sphere.radius <= 0.0 {
                return false;
            }
        }
        true
    }

    pub fn intersects_obb(&self, aabb: &Aabb, model_to_world: &Mat4) -> bool {
        let aabb_center_world = *model_to_world * aabb.center.extend(1.0);
        let axes = [
            Vec3A::from(model_to_world.x_axis),
            Vec3A::from(model_to_world.y_axis),
            Vec3A::from(model_to_world.z_axis),
        ];

        for plane in &self.planes {
            let p_normal = Vec3A::from(plane.normal_d);
            let relative_radius = aabb.relative_radius(&p_normal, &axes);
            if plane.normal_d.dot(aabb_center_world) + relative_radius <= 0.0 {
                return false;
            }
        }
        true
    }
}

#[derive(Component, Debug, Default)]
pub struct CubemapFrusta {
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
