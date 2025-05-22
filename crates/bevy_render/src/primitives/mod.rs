use core::borrow::Borrow;

use bevy_ecs::{component::Component, entity::EntityHashMap, reflect::ReflectComponent};
use bevy_math::{
    Affine3A, CompassOctant, Mat3A, Mat4, URect, UVec2, Vec2, Vec3, Vec3A, Vec4, Vec4Swizzles,
};
use bevy_reflect::prelude::*;

use crate::camera::Viewport;

/// An axis-aligned bounding box, defined by:
/// - a center,
/// - the distances from the center to each faces along the axis,
///   the faces are orthogonal to the axis.
///
/// It is typically used as a component on an entity to represent the local space
/// occupied by this entity, with faces orthogonal to its local axis.
///
/// This component is notably used during "frustum culling", a process to determine
/// if an entity should be rendered by a [`Camera`] if its bounding box intersects
/// with the camera's [`Frustum`].
///
/// It will be added automatically by the systems in [`CalculateBounds`] to entities that:
/// - could be subject to frustum culling, for example with a [`Mesh3d`]
///   or `Sprite` component,
/// - don't have the [`NoFrustumCulling`] component.
///
/// It won't be updated automatically if the space occupied by the entity changes,
/// for example if the vertex positions of a [`Mesh3d`] are updated.
///
/// [`Camera`]: crate::camera::Camera
/// [`NoFrustumCulling`]: crate::view::visibility::NoFrustumCulling
/// [`CalculateBounds`]: crate::view::visibility::VisibilitySystems::CalculateBounds
/// [`Mesh3d`]: crate::mesh::Mesh
#[derive(Component, Clone, Copy, Debug, Default, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
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

    /// Returns a bounding box enclosing the specified set of points.
    ///
    /// Returns `None` if the iterator is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Vec3, Vec3A};
    /// # use bevy_render::primitives::Aabb;
    /// let bb = Aabb::enclosing([Vec3::X, Vec3::Z * 2.0, Vec3::Y * -0.5]).unwrap();
    /// assert_eq!(bb.min(), Vec3A::new(0.0, -0.5, 0.0));
    /// assert_eq!(bb.max(), Vec3A::new(1.0, 0.0, 2.0));
    /// ```
    pub fn enclosing<T: Borrow<Vec3>>(iter: impl IntoIterator<Item = T>) -> Option<Self> {
        let mut iter = iter.into_iter().map(|p| *p.borrow());
        let mut min = iter.next()?;
        let mut max = min;
        for v in iter {
            min = Vec3::min(min, v);
            max = Vec3::max(max, v);
        }
        Some(Self::from_min_max(min, max))
    }

    /// Calculate the relative radius of the AABB with respect to a plane
    #[inline]
    pub fn relative_radius(&self, p_normal: &Vec3A, world_from_local: &Mat3A) -> f32 {
        // NOTE: dot products on Vec3A use SIMD and even with the overhead of conversion are net faster than Vec3
        let half_extents = self.half_extents;
        Vec3A::new(
            p_normal.dot(world_from_local.x_axis),
            p_normal.dot(world_from_local.y_axis),
            p_normal.dot(world_from_local.z_axis),
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

    /// Check if the AABB is at the front side of the bisecting plane.
    /// Referenced from: [AABB Plane intersection](https://gdbooks.gitbooks.io/3dcollisions/content/Chapter2/static_aabb_plane.html)
    #[inline]
    pub fn is_in_half_space(&self, half_space: &HalfSpace, world_from_local: &Affine3A) -> bool {
        // transform the half-extents into world space.
        let half_extents_world = world_from_local.matrix3.abs() * self.half_extents.abs();
        // collapse the half-extents onto the plane normal.
        let p_normal = half_space.normal();
        let r = half_extents_world.dot(p_normal.abs());
        let aabb_center_world = world_from_local.transform_point3a(self.center);
        let signed_distance = p_normal.dot(aabb_center_world) + half_space.d();
        signed_distance > r
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
    pub fn intersects_obb(&self, aabb: &Aabb, world_from_local: &Affine3A) -> bool {
        let aabb_center_world = world_from_local.transform_point3a(aabb.center);
        let v = aabb_center_world - self.center;
        let d = v.length();
        let relative_radius = aabb.relative_radius(&(v / d), &world_from_local.matrix3);
        d < self.radius + relative_radius
    }
}

/// A proportionally-sized "sub-rectangle".
///
/// When [`Camera::crop`] is `Some`, only the sub-section of the
/// image defined by `size` and `offset` (relative to the `full_size` of the
/// whole image) is projected to the cameras viewport.
///
/// Take the example of the following multi-monitor setup:
/// ```css
/// ┌───┬───┐
/// │ A │ B │
/// ├───┼───┤
/// │ C │ D │
/// └───┴───┘
/// ```
/// If each monitor is 1920x1080, the whole image will have a resolution of
/// 3840x2160. For each monitor we can use a single camera with a viewport of
/// the same size as the monitor it corresponds to. To ensure that the image is
/// cohesive, we can set a different crop rectangle on each camera:
/// - Camera A: `full_size` = 3840x2160, `size` = 1920x1080, `offset` = 0,0
/// - Camera B: `full_size` = 3840x2160, `size` = 1920x1080, `offset` = 1920,0
/// - Camera C: `full_size` = 3840x2160, `size` = 1920x1080, `offset` = 0,1080
/// - Camera D: `full_size` = 3840x2160, `size` = 1920x1080, `offset` =
///   1920,1080
///
/// However since only the ratio between the values is important, they could all
/// be divided by 120 and still produce the same image--[`SubRect::reduced`]
/// does this. Camera D would for example have the following values:
/// `full_size` = 32x18, `size` = 16x9, `offset` = 16,9
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
pub struct SubRect {
    /// Size of the whole rectantle
    pub full_size: UVec2,
    /// Size of the sub-rectangle.
    pub size: UVec2,
    /// Offset of the sub-rectangle from the top-left.
    pub offset: Vec2,
}

impl Default for SubRect {
    #[inline]
    fn default() -> Self {
        Self {
            full_size: UVec2::ONE,
            size: UVec2::ONE,
            offset: Vec2::ZERO,
        }
    }
}

impl SubRect {
    /// Returns a `SubRect` representing either a quadrant or half of the full view,
    /// depending on the value of `oct`. For each of the cardinal directions, this
    /// will return the corresponding half, while for each of the intercardinal (NE, NW, SE, SW)
    /// directions, this will return the corresponding quadrant.
    pub fn octant(oct: CompassOctant) -> Self {
        let size = match oct {
            CompassOctant::NorthEast
            | CompassOctant::NorthWest
            | CompassOctant::SouthEast
            | CompassOctant::SouthWest => UVec2::splat(1),
            CompassOctant::North | CompassOctant::South => UVec2::new(2, 1),
            CompassOctant::East | CompassOctant::West => UVec2::new(1, 2),
        };

        let offset = match oct {
            CompassOctant::NorthWest | CompassOctant::North | CompassOctant::West => {
                Vec2::splat(0.)
            }
            CompassOctant::NorthEast | CompassOctant::East => Vec2::new(1., 0.),
            CompassOctant::SouthWest | CompassOctant::South => Vec2::new(0., 1.),
            CompassOctant::SouthEast => Vec2::splat(1.),
        };

        Self {
            full_size: UVec2::splat(2),
            size,
            offset,
        }
    }

    /// Returns this [`SubRect`] with a new value for `full_size`
    #[inline]
    pub fn with_full_size(mut self, full_size: UVec2) -> Self {
        self.full_size = full_size;
        self
    }

    /// Returns this [`SubRect`] with a new value for `size`
    #[inline]
    pub fn with_size(mut self, size: UVec2) -> Self {
        self.size = size;
        self
    }

    /// Returns this [`SubRect`] with a new value for `offset`
    #[inline]
    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// Returns this [`SubRect`] but with all extraneous factors removed
    pub fn reduced(mut self) -> Self {
        let size_gcd = UVec2 {
            x: ops::gcd(self.full_size.x, self.size.x),
            y: ops::gcd(self.full_size.y, self.size.y),
        };

        self.full_size /= size_gcd;
        self.offset /= size_gcd.as_vec2();
        self.size /= size_gcd;
        self
    }

    /// Returns this [`SubRect`] scaled to a new full size.
    ///
    /// Returns Ok if the conversion is lossless (i.e. doesn't cause a change
    /// to the relative `size`), and returns Err otherwise with the closest
    /// possible approximation
    pub fn scaled_to(self, full_size: UVec2) -> Result<Self, Self> {
        let rough = self.scaled_roughly_to(full_size);

        let num = full_size;
        let denom = self.full_size;
        if ((self.size * num) % denom).cmpeq(UVec2::ZERO).all() {
            Ok(rough)
        } else {
            Err(rough)
        }
    }

    /// Returns this [`SubRect`] scaled to a new full size.
    ///
    /// Unlike [`Self::scaled_to`], this method does not check if the
    /// result is lossless, so there might be a change to the relative
    /// `size`
    pub fn scaled_roughly_to(self, full_size: UVec2) -> Self {
        let num = full_size;
        let denom = self.full_size;

        Self {
            full_size,
            size: self.size * num / denom,
            offset: self.offset * num.as_vec2() / denom.as_vec2(),
        }
    }

    /// Returns this [`SubRect`] centered in the full rectangle
    #[inline]
    pub fn centered(self) -> Self {
        self.with_offset((self.full_size - self.size).as_vec2() / 2.)
    }

    // Returns the inverse of this [`SubRect`].
    #[inline]
    pub fn inverted(self) -> Self {
        Self {
            full_size: self.size,
            offset: -self.offset,
            size: self.full_size,
        }
    }

    #[inline]
    pub fn as_urect(self) -> URect {
        let offset = self.offset.as_uvec2();
        URect {
            min: offset,
            max: self.size + offset,
        }
    }

    #[inline]
    pub fn from_urect(full_size: UVec2, rect: URect) -> Self {
        Self {
            full_size,
            offset: rect.min.as_vec2(),
            size: rect.max - rect.min,
        }
    }
}

mod ops {
    // implementations copied from `num` crate, though since they're standard algorithms (and
    // fairly small snippets) do we still need to credit?

    /// Calculates the Greatest Common Divisor (GCD) of the number and `other`
    #[inline]
    pub fn gcd(mut a: u32, mut b: u32) -> u32 {
        // Use Stein's algorithm
        if a == 0 || b == 0 {
            return 0;
        }

        // find common factors of 2
        let shift = (a | b).trailing_zeros();

        // divide n and m by 2 until odd
        a >>= a.trailing_zeros();
        b >>= b.trailing_zeros();

        while a != b {
            if a > b {
                a -= b;
                a >>= a.trailing_zeros();
            } else {
                b -= a;
                b >>= b.trailing_zeros();
            }
        }

        a << shift
    }
}

/// A region of 3D space, specifically an open set whose border is a bisecting 2D plane.
///
/// This bisecting plane partitions 3D space into two infinite regions,
/// the half-space is one of those regions and excludes the bisecting plane.
///
/// Each instance of this type is characterized by:
/// - the bisecting plane's unit normal, normalized and pointing "inside" the half-space,
/// - the signed distance along the normal from the bisecting plane to the origin of 3D space.
///
/// The distance can also be seen as:
/// - the distance along the inverse of the normal from the origin of 3D space to the bisecting plane,
/// - the opposite of the distance along the normal from the origin of 3D space to the bisecting plane.
///
/// Any point `p` is considered to be within the `HalfSpace` when the length of the projection
/// of p on the normal is greater or equal than the opposite of the distance,
/// meaning: if the equation `normal.dot(p) + distance > 0.` is satisfied.
///
/// For example, the half-space containing all the points with a z-coordinate lesser
/// or equal than `8.0` would be defined by: `HalfSpace::new(Vec3::NEG_Z.extend(-8.0))`.
/// It includes all the points from the bisecting plane towards `NEG_Z`, and the distance
/// from the plane to the origin is `-8.0` along `NEG_Z`.
///
/// It is used to define a [`Frustum`], but is also a useful mathematical primitive for rendering tasks such as  light computation.
#[derive(Clone, Copy, Debug, Default)]
pub struct HalfSpace {
    normal_d: Vec4,
}

impl HalfSpace {
    /// Constructs a `HalfSpace` from a 4D vector whose first 3 components
    /// represent the bisecting plane's unit normal, and the last component is
    /// the signed distance along the normal from the plane to the origin.
    /// The constructor ensures the normal vector is normalized and the distance is appropriately scaled.
    #[inline]
    pub fn new(normal_d: Vec4) -> Self {
        Self {
            normal_d: normal_d * normal_d.xyz().length_recip(),
        }
    }

    /// Returns the unit normal vector of the bisecting plane that characterizes the `HalfSpace`.
    #[inline]
    pub fn normal(&self) -> Vec3A {
        Vec3A::from_vec4(self.normal_d)
    }

    /// Returns the signed distance from the bisecting plane to the origin along
    /// the plane's unit normal vector.
    #[inline]
    pub fn d(&self) -> f32 {
        self.normal_d.w
    }

    /// Returns the bisecting plane's unit normal vector and the signed distance
    /// from the plane to the origin.
    #[inline]
    pub fn normal_d(&self) -> Vec4 {
        self.normal_d
    }
}

/// A region of 3D space defined by the intersection of 6 [`HalfSpace`]s.
///
/// Frustums are typically an apex-truncated square pyramid (a pyramid without the top) or a cuboid.
///
/// Half spaces are ordered left, right, top, bottom, near, far. The normal vectors
/// of the half-spaces point towards the interior of the frustum.
///
/// A frustum component is used on an entity with a [`Camera`] component to
/// determine which entities will be considered for rendering by this camera.
/// All entities with an [`Aabb`] component that are not contained by (or crossing
/// the boundary of) the frustum will not be rendered, and not be used in rendering computations.
///
/// This process is called frustum culling, and entities can opt out of it using
/// the [`NoFrustumCulling`] component.
///
/// The frustum component is typically added automatically for cameras, either `Camera2d` or `Camera3d`.
/// It is usually updated automatically by [`update_frusta`] from the
/// [`CameraProjection`] component and [`GlobalTransform`] of the camera entity.
///
/// [`Camera`]: crate::camera::Camera
/// [`NoFrustumCulling`]: crate::view::visibility::NoFrustumCulling
/// [`update_frusta`]: crate::view::visibility::update_frusta
/// [`CameraProjection`]: crate::camera::CameraProjection
/// [`GlobalTransform`]: bevy_transform::components::GlobalTransform
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct Frustum {
    #[reflect(ignore, clone)]
    pub half_spaces: [HalfSpace; 6],
}

impl Frustum {
    /// Returns a frustum derived from `clip_from_world`.
    #[inline]
    pub fn from_clip_from_world(clip_from_world: &Mat4) -> Self {
        let mut frustum = Frustum::from_clip_from_world_no_far(clip_from_world);
        frustum.half_spaces[5] = HalfSpace::new(clip_from_world.row(2));
        frustum
    }

    /// Returns a frustum derived from `clip_from_world`,
    /// but with a custom far plane.
    #[inline]
    pub fn from_clip_from_world_custom_far(
        clip_from_world: &Mat4,
        view_translation: &Vec3,
        view_backward: &Vec3,
        far: f32,
    ) -> Self {
        let mut frustum = Frustum::from_clip_from_world_no_far(clip_from_world);
        let far_center = *view_translation - far * *view_backward;
        frustum.half_spaces[5] =
            HalfSpace::new(view_backward.extend(-view_backward.dot(far_center)));
        frustum
    }

    // NOTE: This approach of extracting the frustum half-space from the view
    // projection matrix is from Foundations of Game Engine Development 2
    // Rendering by Lengyel.
    /// Returns a frustum derived from `view_projection`,
    /// without a far plane.
    fn from_clip_from_world_no_far(clip_from_world: &Mat4) -> Self {
        let row3 = clip_from_world.row(3);
        let mut half_spaces = [HalfSpace::default(); 6];
        for (i, half_space) in half_spaces.iter_mut().enumerate().take(5) {
            let row = clip_from_world.row(i / 2);
            *half_space = HalfSpace::new(if (i & 1) == 0 && i != 4 {
                row3 + row
            } else {
                row3 - row
            });
        }
        Self { half_spaces }
    }

    /// Checks if a sphere intersects the frustum.
    #[inline]
    pub fn intersects_sphere(&self, sphere: &Sphere, intersect_far: bool) -> bool {
        let sphere_center = sphere.center.extend(1.0);
        let max = if intersect_far { 6 } else { 5 };
        for half_space in &self.half_spaces[..max] {
            if half_space.normal_d().dot(sphere_center) + sphere.radius <= 0.0 {
                return false;
            }
        }
        true
    }

    /// Checks if an Oriented Bounding Box (obb) intersects the frustum.
    #[inline]
    pub fn intersects_obb(
        &self,
        aabb: &Aabb,
        world_from_local: &Affine3A,
        intersect_near: bool,
        intersect_far: bool,
    ) -> bool {
        let aabb_center_world = world_from_local.transform_point3a(aabb.center).extend(1.0);
        for (idx, half_space) in self.half_spaces.into_iter().enumerate() {
            if idx == 4 && !intersect_near {
                continue;
            }
            if idx == 5 && !intersect_far {
                continue;
            }
            let p_normal = half_space.normal();
            let relative_radius = aabb.relative_radius(&p_normal, &world_from_local.matrix3);
            if half_space.normal_d().dot(aabb_center_world) + relative_radius <= 0.0 {
                return false;
            }
        }
        true
    }

    /// Check if the frustum contains the Axis-Aligned Bounding Box (AABB).
    /// Referenced from: [Frustum Culling](https://learnopengl.com/Guest-Articles/2021/Scene/Frustum-Culling)
    #[inline]
    pub fn contains_aabb(&self, aabb: &Aabb, world_from_local: &Affine3A) -> bool {
        for half_space in &self.half_spaces {
            if !aabb.is_in_half_space(half_space, world_from_local) {
                return false;
            }
        }
        true
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct CubemapFrusta {
    #[reflect(ignore, clone)]
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

#[derive(Component, Debug, Default, Reflect, Clone)]
#[reflect(Component, Default, Debug, Clone)]
pub struct CascadesFrusta {
    #[reflect(ignore, clone)]
    pub frusta: EntityHashMap<Vec<Frustum>>,
}

#[cfg(test)]
mod tests {
    use core::f32::consts::PI;

    use bevy_math::{ops, Quat};
    use bevy_transform::components::GlobalTransform;

    use crate::camera::{CameraProjection, PerspectiveProjection};

    use super::*;

    // A big, offset frustum
    fn big_frustum() -> Frustum {
        Frustum {
            half_spaces: [
                HalfSpace::new(Vec4::new(-0.9701, -0.2425, -0.0000, 7.7611)),
                HalfSpace::new(Vec4::new(-0.0000, 1.0000, -0.0000, 4.0000)),
                HalfSpace::new(Vec4::new(-0.0000, -0.2425, -0.9701, 2.9104)),
                HalfSpace::new(Vec4::new(-0.0000, -1.0000, -0.0000, 4.0000)),
                HalfSpace::new(Vec4::new(-0.0000, -0.2425, 0.9701, 2.9104)),
                HalfSpace::new(Vec4::new(0.9701, -0.2425, -0.0000, -1.9403)),
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
            half_spaces: [
                HalfSpace::new(Vec4::new(-0.9701, -0.2425, -0.0000, 0.7276)),
                HalfSpace::new(Vec4::new(-0.0000, 1.0000, -0.0000, 1.0000)),
                HalfSpace::new(Vec4::new(-0.0000, -0.2425, -0.9701, 0.7276)),
                HalfSpace::new(Vec4::new(-0.0000, -1.0000, -0.0000, 1.0000)),
                HalfSpace::new(Vec4::new(-0.0000, -0.2425, 0.9701, 0.7276)),
                HalfSpace::new(Vec4::new(0.9701, -0.2425, -0.0000, 0.7276)),
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
            half_spaces: [
                HalfSpace::new(Vec4::new(-0.9998, -0.0222, -0.0000, -1.9543)),
                HalfSpace::new(Vec4::new(-0.0000, 1.0000, -0.0000, 45.1249)),
                HalfSpace::new(Vec4::new(-0.0000, -0.0168, -0.9999, 2.2718)),
                HalfSpace::new(Vec4::new(-0.0000, -1.0000, -0.0000, 45.1249)),
                HalfSpace::new(Vec4::new(-0.0000, -0.0168, 0.9999, 2.2718)),
                HalfSpace::new(Vec4::new(0.9998, -0.0222, -0.0000, 7.9528)),
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

    #[test]
    fn aabb_enclosing() {
        assert_eq!(Aabb::enclosing(<[Vec3; 0]>::default()), None);
        assert_eq!(
            Aabb::enclosing(vec![Vec3::ONE]).unwrap(),
            Aabb::from_min_max(Vec3::ONE, Vec3::ONE)
        );
        assert_eq!(
            Aabb::enclosing(&[Vec3::Y, Vec3::X, Vec3::Z][..]).unwrap(),
            Aabb::from_min_max(Vec3::ZERO, Vec3::ONE)
        );
        assert_eq!(
            Aabb::enclosing([
                Vec3::NEG_X,
                Vec3::X * 2.0,
                Vec3::NEG_Y * 5.0,
                Vec3::Z,
                Vec3::ZERO
            ])
            .unwrap(),
            Aabb::from_min_max(Vec3::new(-1.0, -5.0, 0.0), Vec3::new(2.0, 0.0, 1.0))
        );
    }

    // A frustum with an offset for testing the [`Frustum::contains_aabb`] algorithm.
    fn contains_aabb_test_frustum() -> Frustum {
        let proj = PerspectiveProjection {
            fov: 90.0_f32.to_radians(),
            aspect_ratio: 1.0,
            near: 1.0,
            far: 100.0,
        };
        proj.compute_frustum(&GlobalTransform::from_translation(Vec3::new(2.0, 2.0, 0.0)))
    }

    fn contains_aabb_test_frustum_with_rotation() -> Frustum {
        let half_extent_world = (((49.5 * 49.5) * 0.5) as f32).sqrt() + 0.5f32.sqrt();
        let near = 50.5 - half_extent_world;
        let far = near + 2.0 * half_extent_world;
        let fov = 2.0 * ops::atan(half_extent_world / near);
        let proj = PerspectiveProjection {
            aspect_ratio: 1.0,
            near,
            far,
            fov,
        };
        proj.compute_frustum(&GlobalTransform::IDENTITY)
    }

    #[test]
    fn aabb_inside_frustum() {
        let frustum = contains_aabb_test_frustum();
        let aabb = Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::new(0.99, 0.99, 49.49),
        };
        let model = Affine3A::from_translation(Vec3::new(2.0, 2.0, -50.5));
        assert!(frustum.contains_aabb(&aabb, &model));
    }

    #[test]
    fn aabb_intersect_frustum() {
        let frustum = contains_aabb_test_frustum();
        let aabb = Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::new(0.99, 0.99, 49.6),
        };
        let model = Affine3A::from_translation(Vec3::new(2.0, 2.0, -50.5));
        assert!(!frustum.contains_aabb(&aabb, &model));
    }

    #[test]
    fn aabb_outside_frustum() {
        let frustum = contains_aabb_test_frustum();
        let aabb = Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::new(0.99, 0.99, 0.99),
        };
        let model = Affine3A::from_translation(Vec3::new(0.0, 0.0, 49.6));
        assert!(!frustum.contains_aabb(&aabb, &model));
    }

    #[test]
    fn aabb_inside_frustum_rotation() {
        let frustum = contains_aabb_test_frustum_with_rotation();
        let aabb = Aabb {
            center: Vec3A::new(0.0, 0.0, 0.0),
            half_extents: Vec3A::new(0.99, 0.99, 49.49),
        };

        let model = Affine3A::from_rotation_translation(
            Quat::from_rotation_x(PI / 4.0),
            Vec3::new(0.0, 0.0, -50.5),
        );
        assert!(frustum.contains_aabb(&aabb, &model));
    }

    #[test]
    fn aabb_intersect_frustum_rotation() {
        let frustum = contains_aabb_test_frustum_with_rotation();
        let aabb = Aabb {
            center: Vec3A::new(0.0, 0.0, 0.0),
            half_extents: Vec3A::new(0.99, 0.99, 49.6),
        };

        let model = Affine3A::from_rotation_translation(
            Quat::from_rotation_x(PI / 4.0),
            Vec3::new(0.0, 0.0, -50.5),
        );
        assert!(!frustum.contains_aabb(&aabb, &model));
    }

    #[test]
    fn sub_rect_centered() {
        let top_left = SubRect::octant(CompassOctant::NorthWest);
        let right = SubRect::octant(CompassOctant::East);

        assert_eq!(
            top_left.centered(),
            SubRect {
                offset: Vec2::splat(0.5),
                ..top_left
            }
        );

        assert_eq!(
            right.centered(),
            SubRect {
                offset: Vec2::new(0.5, 0.0),
                ..right
            }
        );
    }

    #[test]
    fn sub_rect_reduced() {
        let reducible_same_factor = SubRect {
            full_size: UVec2::new(200, 160),
            size: UVec2::new(50, 40),
            offset: Vec2::ZERO,
        };

        let reducible_diff_factor = SubRect {
            full_size: UVec2::new(80, 160),
            size: UVec2::new(30, 40),
            offset: Vec2::ZERO,
        };

        let irreducible = SubRect {
            full_size: UVec2::new(17, 5),
            size: UVec2::new(4, 3),
            offset: Vec2::ZERO,
        };

        assert_eq!(
            reducible_same_factor.reduced(),
            SubRect {
                full_size: UVec2::splat(4),
                size: UVec2::splat(1),
                offset: Vec2::ZERO
            }
        );

        assert_eq!(
            reducible_diff_factor.reduced(),
            SubRect {
                full_size: UVec2::new(8, 4),
                size: UVec2::new(3, 1),
                offset: Vec2::ZERO
            }
        );

        assert_eq!(irreducible.reduced(), irreducible);
    }

    #[test]
    fn sub_rect_scaled_to() {
        let top_left = SubRect::octant(CompassOctant::NorthWest);

        assert_eq!(
            top_left.scaled_to(UVec2::splat(200)),
            Ok(SubRect {
                full_size: UVec2::splat(200),
                size: UVec2::splat(100),
                offset: Vec2::ZERO,
            }),
        );

        assert_eq!(
            top_left.scaled_to(UVec2::new(1920, 1080)),
            Ok(SubRect {
                full_size: UVec2::new(1920, 1080),
                size: UVec2::new(960, 540),
                offset: Vec2::ZERO,
            }),
        );

        // Don't need to guarantee exact error values, as long as they're approximately correct
        assert!(top_left.scaled_to(UVec2::new(100, 99)).is_err());
        assert!(top_left.scaled_to(UVec2::new(11, 11)).is_err());
    }

    #[test]
    fn sub_rect_inverse() {
        let rects = [
            SubRect::default(),
            SubRect::octant(CompassOctant::SouthEast),
            SubRect::octant(CompassOctant::SouthEast)
                .scaled_to(UVec2::new(184, 240))
                .unwrap(),
            SubRect {
                full_size: UVec2::new(1740, 1800),
                size: UVec2::splat(200),
                offset: Vec2::splat(100.),
            },
            SubRect {
                full_size: UVec2::new(203, 160),
                size: UVec2::new(1, 28),
                offset: Vec2::new(170., 100.),
            },
            SubRect {
                full_size: UVec2::new(10, 8202742),
                size: UVec2::new(10, 10000),
                offset: Vec2::splat(0.),
            },
            SubRect {
                full_size: UVec2::splat(180),
                size: UVec2::splat(179),
                offset: Vec2::splat(1.),
            },
        ];

        for r in rects {
            assert_eq!(r.inverted().inverted(), r);
        }
    }
}
