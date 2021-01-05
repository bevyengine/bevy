use super::Transform;
use bevy_math::{Mat3, Mat4, Quat, Vec3};
use bevy_reflect::{Reflect, ReflectComponent};
use std::ops::Mul;

#[derive(Debug, PartialEq, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct GlobalTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl GlobalTransform {
    /// Create a new [`GlobalTransform`] at the position `(x, y, z)`
    #[inline]
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_translation(Vec3::new(x, y, z))
    }

    #[inline]
    pub fn identity() -> Self {
        GlobalTransform {
            translation: Vec3::zero(),
            rotation: Quat::identity(),
            scale: Vec3::one(),
        }
    }

    #[inline]
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();

        GlobalTransform {
            translation,
            rotation,
            scale,
        }
    }

    #[inline]
    pub fn from_translation(translation: Vec3) -> Self {
        GlobalTransform {
            translation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_rotation(rotation: Quat) -> Self {
        GlobalTransform {
            rotation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        GlobalTransform {
            scale,
            ..Default::default()
        }
    }

    /// Returns transform with the same translation and scale, but rotation so that transform.forward() points at target
    #[inline]
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        self.look_at(target, up);
        self
    }

    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    #[inline]
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::unit_z()
    }

    #[inline]
    /// Rotate the transform by the given rotation
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation *= rotation;
    }

    #[inline]
    pub fn mul_transform(&self, transform: Transform) -> GlobalTransform {
        let translation = self.mul_vec3(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        GlobalTransform {
            scale,
            rotation,
            translation,
        }
    }

    #[inline]
    pub fn mul_vec3(&self, mut value: Vec3) -> Vec3 {
        value = self.rotation * value;
        value = self.scale * value;
        value += self.translation;
        value
    }

    #[inline]
    pub fn apply_non_uniform_scale(&mut self, scale: Vec3) {
        self.scale *= scale;
    }

    #[inline]
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let forward = Vec3::normalize(self.translation - target);
        let right = up.cross(forward).normalize();
        let up = forward.cross(right);
        self.rotation = Quat::from_rotation_mat3(&Mat3::from_cols(right, up, forward));
    }
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl From<Transform> for GlobalTransform {
    fn from(transform: Transform) -> Self {
        Self {
            translation: transform.translation,
            rotation: transform.rotation,
            scale: transform.scale,
        }
    }
}

impl Mul<GlobalTransform> for GlobalTransform {
    type Output = GlobalTransform;

    #[inline]
    fn mul(self, global_transform: GlobalTransform) -> Self::Output {
        self.mul_transform(global_transform.into())
    }
}

impl Mul<Transform> for GlobalTransform {
    type Output = GlobalTransform;

    #[inline]
    fn mul(self, transform: Transform) -> Self::Output {
        self.mul_transform(transform)
    }
}

impl Mul<Vec3> for GlobalTransform {
    type Output = Vec3;

    #[inline]
    fn mul(self, value: Vec3) -> Self::Output {
        self.mul_vec3(value)
    }
}
