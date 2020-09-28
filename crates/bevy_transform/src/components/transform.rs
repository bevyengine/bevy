use bevy_math::{Mat3, Mat4, Quat, Vec3};
use bevy_property::Properties;
use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy, Properties)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    #[inline(always)]
    pub fn identity() -> Self {
        Transform {
            translation: Vec3::zero(),
            rotation: Quat::identity(),
            scale: Vec3::one(),
        }
    }

    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();

        Transform {
            translation,
            rotation,
            scale,
        }
    }

    pub fn from_translation(translation: Vec3) -> Self {
        Transform {
            translation,
            ..Default::default()
        }
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        Transform {
            rotation,
            ..Default::default()
        }
    }

    pub fn from_scale(scale: f32) -> Self {
        Transform {
            scale: Vec3::one() * scale,
            ..Default::default()
        }
    }

    pub fn from_non_uniform_scale(scale: Vec3) -> Self {
        Transform {
            scale,
            ..Default::default()
        }
    }

    pub fn from_translation_rotation(translation: Vec3, rotation: Quat) -> Self {
        Transform {
            translation,
            rotation,
            ..Default::default()
        }
    }

    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = Vec3::one() * scale;
        self
    }

    pub fn with_non_uniform_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Returns transform with the same translation and scale, but rotation so that transform.forward() points at the origin
    pub fn looking_at_origin(self) -> Self {
        self.looking_at(Vec3::zero(), Vec3::unit_y())
    }

    /// Returns transform with the same translation and scale, but rotation so that transform.forward() points at target
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        self.look_at(target, up);
        self
    }

    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::unit_z()
    }

    /// Translates the transform by the given translation relative to its orientation
    pub fn translate(&mut self, translation: Vec3) {
        self.translation += self.rotation * translation;
    }

    /// Rotate the transform by the given rotation
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation = self.rotation * rotation;
    }

    pub fn apply_scale(&mut self, scale: f32) {
        self.scale *= scale;
    }

    pub fn apply_non_uniform_scale(&mut self, scale: Vec3) {
        self.scale *= scale;
    }

    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let forward = Vec3::normalize(self.translation - target);
        let right = up.cross(forward).normalize();
        let up = forward.cross(right);
        self.rotation = Quat::from_rotation_mat3(&Mat3::from_cols(right, up, forward));
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl fmt::Display for Transform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.compute_matrix())
    }
}
