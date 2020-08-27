use super::{NonUniformScale, Rotation, Scale, Translation};
use bevy_math::{Mat4, Vec3};
use bevy_property::Properties;
use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy, Properties)]
pub struct Transform {
    local: Mat4,
    global: Mat4,
}

impl Transform {
    #[inline(always)]
    pub fn identity() -> Self {
        Transform {
            local: Mat4::identity(),
            global: Mat4::identity(),
        }
    }

    #[inline(always)]
    pub fn new(local: Mat4) -> Self {
        Transform {
            local,
            global: local,
        }
    }

    pub fn new_with_parent(local: Mat4, parent: &Mat4) -> Self {
        Transform {
            local,
            global: *parent * local,
        }
    }

    pub fn from_parent(parent: &Mat4) -> Self {
        Transform {
            local: Mat4::default(),
            global: *parent,
        }
    }

    pub fn local_matrix(&self) -> &Mat4 {
        &self.local
    }

    pub fn local_matrix_mut(&mut self) -> &mut Mat4 {
        &mut self.local
    }

    pub fn global_matrix(&self) -> &Mat4 {
        &self.global
    }

    pub fn apply_parent_matrix(&mut self, parent: Option<Mat4>) {
        match parent {
            Some(parent) => self.global = parent * self.local,
            None => self.global = self.local,
        };
    }

    pub fn local_translation(&self) -> Translation {
        Vec3::from(self.local.w_axis().truncate()).into()
    }

    // FIXME: only gets updated post update
    pub fn global_translation(&self) -> Translation {
        Vec3::from(self.global.w_axis().truncate()).into()
    }

    pub fn set_local_translation(&mut self, translation: &Translation) {
        *self.local.w_axis_mut() = translation.extend(1.0);
    }

    pub fn translate(&mut self, translation: &Translation) {
        *self.local.w_axis_mut() += translation.extend(0.0);
    }

    pub fn rotate(&mut self, rotation: &Rotation) {
        self.local = self.local.mul_mat4(&Mat4::from_quat(rotation.0));
    }

    pub fn local_non_uniform_scale(&self) -> NonUniformScale {
        NonUniformScale::new(
            self.local.x_axis().truncate().length(),
            self.local.y_axis().truncate().length(),
            self.local.z_axis().truncate().length(),
        )
    }

    // FIXME: only gets updated post update
    pub fn global_non_uniform_scale(&self) -> NonUniformScale {
        NonUniformScale::new(
            self.global.x_axis().truncate().length(),
            self.global.y_axis().truncate().length(),
            self.global.z_axis().truncate().length(),
        )
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl fmt::Display for Transform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.local)
    }
}

impl From<Translation> for Transform {
    fn from(translation: Translation) -> Self {
        Transform::new(Mat4::from_translation(translation.0))
    }
}

impl From<Rotation> for Transform {
    fn from(rotation: Rotation) -> Self {
        Transform::new(Mat4::from_quat(rotation.0))
    }
}

// NOTE: extra simple for testing purposes
pub struct TransformBuilder {
    translation: Translation,
    rotation: Rotation,
    nu_scale: NonUniformScale,
}

impl TransformBuilder {
    pub fn new() -> Self {
        TransformBuilder {
            translation: Translation::default(),
            rotation: Rotation::default(),
            nu_scale: NonUniformScale::default(),
        }
    }

    pub fn set_translation(mut self, translation: Translation) -> Self {
        self.translation = translation;
        self
    }

    pub fn set_rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn set_scale(mut self, scale: Scale) -> Self {
        self.nu_scale = NonUniformScale::new(scale.0, scale.0, scale.0);
        self
    }

    pub fn set_non_uniform_scale(mut self, nu_scale: NonUniformScale) -> Self {
        self.nu_scale = nu_scale;
        self
    }

    pub fn build(self) -> Transform {
        Transform::new(Mat4::from_scale_rotation_translation(
            self.nu_scale.0,
            self.rotation.0,
            self.translation.0,
        ))
    }
}
