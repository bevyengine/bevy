use bevy_math::{Mat3, Mat4, Quat, Vec3};
use bevy_property::Properties;
use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy, Properties)]
pub struct Transform {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
    matrix_cache: Option<Mat4>,
}

impl Transform {
    #[inline(always)]
    pub fn identity() -> Self {
        Transform {
            translation: Vec3::zero(),
            rotation: Quat::identity(),
            scale: Vec3::one(),
            matrix_cache: Some(Mat4::identity()),
        }
    }

    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();

        Transform {
            translation,
            rotation,
            scale,
            matrix_cache: Some(matrix),
        }
    }

    pub fn from_translation(translation: Vec3) -> Self {
        Transform {
            translation,
            matrix_cache: None,
            ..Default::default()
        }
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        Transform {
            rotation,
            matrix_cache: None,
            ..Default::default()
        }
    }

    pub fn from_scale(scale: f32) -> Self {
        Transform {
            scale: Vec3::one() * scale,
            matrix_cache: None,
            ..Default::default()
        }
    }

    pub fn from_non_uniform_scale(scale: Vec3) -> Self {
        Transform {
            scale,
            matrix_cache: None,
            ..Default::default()
        }
    }

    pub fn from_translation_rotation(translation: Vec3, rotation: Quat) -> Self {
        Transform {
            translation,
            rotation,
            matrix_cache: None,
            ..Default::default()
        }
    }

    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self.matrix_cache = None;
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self.matrix_cache = None;
        self
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = Vec3::one() * scale;
        self.matrix_cache = None;
        self
    }

    pub fn with_non_uniform_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self.matrix_cache = None;
        self
    }

    /// Returns transform with the same translation and scale, but rotation so that transform.forward() points at the origin
    pub fn looking_at_origin(self) -> Self {
        self.looking_at(Vec3::zero(), Vec3::unit_y())
    }

    /// Returns transform with the same translation and scale, but rotation so that transform.forward() points at target
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        self.look_at(target, up);
        self.matrix_cache = None;
        self
    }

    pub fn translation(&self) -> &Vec3 {
        &self.translation
    }

    pub fn translation_mut(&mut self) -> &mut Vec3 {
        self.matrix_cache = None;
        &mut self.translation
    }

    pub fn rotation(&self) -> &Quat {
        &self.rotation
    }

    pub fn rotation_mut(&mut self) -> &mut Quat {
        self.matrix_cache = None;
        &mut self.rotation
    }

    pub fn scale(&self) -> &Vec3 {
        &self.scale
    }

    pub fn scale_mut(&mut self) -> &mut Vec3 {
        self.matrix_cache = None;
        &mut self.scale
    }

    pub fn matrix(&mut self) -> Mat4 {
        if self.matrix_cache.is_none() {
            self.matrix_cache = Some(Mat4::from_scale_rotation_translation(
                self.scale,
                self.rotation,
                self.translation,
            ));
        }
        self.matrix_cache.unwrap()
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::unit_z()
    }

    /// Translates the transform by the given translation relative to its orientation
    pub fn translate(&mut self, translation: Vec3) {
        self.translation += self.rotation * translation;
        self.matrix_cache = None;
    }

    /// Rotate the transform by the given rotation
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation *= rotation;
        self.matrix_cache = None;
    }

    pub fn apply_scale(&mut self, scale: f32) {
        self.scale *= scale;
        self.matrix_cache = None;
    }

    pub fn apply_non_uniform_scale(&mut self, scale: Vec3) {
        self.scale *= scale;
        self.matrix_cache = None;
    }

    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let forward = Vec3::normalize(self.translation - target);
        let right = up.cross(forward).normalize();
        let up = forward.cross(right);
        self.rotation = Quat::from_rotation_mat3(&Mat3::from_cols(right, up, forward));
        self.matrix_cache = None;
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl fmt::Display for Transform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "translation: {}\nrotation: {}\nscale: {}\nmatrix {}computed",
            self.translation,
            self.rotation,
            self.scale,
            if self.matrix_cache.is_none() {
                "not "
            } else {
                ""
            }
        )
    }
}
