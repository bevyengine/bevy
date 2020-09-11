use bevy_math::{Mat3, Mat4, Quat, Vec3};
use bevy_property::Properties;
use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy, Properties)]
pub struct Transform {
    local: Mat4,
    global: Mat4,
}

impl Transform {
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

    #[inline(always)]
    pub fn identity() -> Self {
        Transform {
            local: Mat4::identity(),
            global: Mat4::identity(),
        }
    }

    pub fn from_translation(translation: Vec3) -> Self {
        Transform::new(Mat4::from_translation(translation))
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        Transform::new(Mat4::from_quat(rotation))
    }

    // TODO: make sure scale is positive
    pub fn from_scale(scale: Vec3) -> Self {
        Transform::new(Mat4::from_scale(scale))
    }

    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translate(translation);
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotate(rotation);
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.apply_scale(scale);
        self
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

    pub fn local_translation(&self) -> Vec3 {
        Vec3::from(self.local.w_axis().truncate())
    }

    // FIXME: only gets updated post update
    pub fn global_translation(&self) -> Vec3 {
        Vec3::from(self.global.w_axis().truncate())
    }

    pub fn local_rotation(&self) -> Quat {
        let scale = self.local_scale();

        Quat::from_rotation_mat3(&Mat3::from_cols(
            Vec3::from(self.local.x_axis().truncate()) / scale.x(),
            Vec3::from(self.local.y_axis().truncate()) / scale.y(),
            Vec3::from(self.local.z_axis().truncate()) / scale.z(),
        ))
    }

    pub fn local_scale(&self) -> Vec3 {
        Vec3::new(
            self.local.x_axis().truncate().length(),
            self.local.y_axis().truncate().length(),
            self.local.z_axis().truncate().length(),
        )
    }

    // FIXME: only gets updated post update
    pub fn global_scale(&self) -> Vec3 {
        Vec3::new(
            self.global.x_axis().truncate().length(),
            self.global.y_axis().truncate().length(),
            self.global.z_axis().truncate().length(),
        )
    }

    pub fn set_local_translation(&mut self, translation: Vec3) {
        *self.local.w_axis_mut() = translation.extend(1.0);
    }

    pub fn set_local_rotation(&mut self, rotation: Quat) {
        let rotation = rotation * self.local_rotation().conjugate();
        rotation.normalize();
        self.local = Mat4::from_quat(rotation) * self.local;
    }

    pub fn set_local_scale(&mut self, scale: Vec3) {
        let scale = scale / self.local_scale();
        self.local = Mat4::from_scale(scale) * self.local;
    }

    pub fn apply_parent_matrix(&mut self, parent: Option<Mat4>) {
        match parent {
            Some(parent) => self.global = parent * self.local,
            None => self.global = self.local,
        };
    }

    pub fn translate(&mut self, translation: Vec3) {
        *self.local.w_axis_mut() += translation.extend(0.0);
    }

    pub fn rotate(&mut self, rotation: Quat) {
        self.local = Mat4::from_quat(rotation) * self.local;
    }

    pub fn apply_scale(&mut self, scale: Vec3) {
        self.local = Mat4::from_scale(scale) * self.local;
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
