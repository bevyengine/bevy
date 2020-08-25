use crate::components::{NonUniformScale, Rotation, Scale, Transform, Translation};
use bevy_math::{Mat4, Quat, Vec3};

/// Allows for intuitive composition of tranform components.
///
/// Results are of the most specific possible type. For instance
/// `rotation1.then_rotate(*rotation2)` returns another Rotation, but
/// `rotation.then_translate(*translation)` returns a Transform.
///
/// ```
/// # use bevy_transform::components::{NonUniformScale, ComposableTransform};
/// # use bevy_math::{Mat4, Quat, Vec3};
/// let comp = NonUniformScale::new(1.0, 2.0, 3.0)
///     .then_scale(4.0)
///     .then_rotate(Quat::from_rotation_ypr(5.0, 6.0, 7.0))
///     .then_translate(Vec3::new(8.0, 9.0, 10.0));
/// let expected = Mat4::from_scale_rotation_translation(
///     Vec3::new(4.0, 8.0, 12.0),
///     Quat::from_rotation_ypr(5.0, 6.0, 7.0),
///     Vec3::new(8.0, 9.0, 10.0),
/// );
/// assert!(comp.value.abs_diff_eq(expected, 0.0001));
/// ```
pub trait ComposableTransform {
    /// The resulting type when the current transform is composed with a NonUniformScale
    type WithNonUniformScale;
    /// The resulting type when the current transform is composed with a Rotation
    type WithRotation;
    /// The resulting type when the current transform is composed with a Scale
    type WithScale;
    /// The resulting type when the current transform is composed with a Translation
    type WithTranslation;

    /// Applies a general transform after the current transform
    fn then_transform(self, other: Mat4) -> Transform;
    /// Applies a non uniform scale after the current transform
    fn then_non_uniform_scale(self, other: Vec3) -> Self::WithNonUniformScale;
    /// Applies a rotation after the current transform
    fn then_rotate(self, other: Quat) -> Self::WithRotation;
    /// Applies a uniform scale after the current transform
    fn then_scale(self, other: f32) -> Self::WithScale;
    /// Applies a translation after the current transform
    fn then_translate(self, other: Vec3) -> Self::WithTranslation;
}

impl ComposableTransform for Transform {
    type WithNonUniformScale = Transform;
    type WithRotation = Transform;
    type WithScale = Transform;
    type WithTranslation = Transform;

    fn then_transform(self, other: Mat4) -> Transform {
        Transform {
            value: other * self.value,
            sync: false,
        }
    }

    fn then_non_uniform_scale(self, other: Vec3) -> Self::WithNonUniformScale {
        Transform {
            value: Mat4::from_scale(other) * self.value,
            sync: false,
        }
    }

    fn then_rotate(self, other: Quat) -> Self::WithRotation {
        Transform {
            value: Mat4::from_quat(other) * self.value,
            sync: false,
        }
    }

    fn then_scale(self, other: f32) -> Self::WithScale {
        Transform {
            value: Mat4::from_scale(Vec3::new(other, other, other)) * self.value,
            sync: false,
        }
    }

    fn then_translate(self, other: Vec3) -> Self::WithTranslation {
        Transform {
            value: Mat4::from_translation(other) * self.value,
            sync: false,
        }
    }
}

impl ComposableTransform for NonUniformScale {
    type WithNonUniformScale = NonUniformScale;
    type WithRotation = Transform;
    type WithScale = NonUniformScale;
    type WithTranslation = Transform;

    fn then_transform(self, other: Mat4) -> Transform {
        Transform {
            value: other * Mat4::from_scale(*self),
            sync: false,
        }
    }

    fn then_non_uniform_scale(self, other: Vec3) -> Self::WithNonUniformScale {
        NonUniformScale(other * *self)
    }

    fn then_rotate(self, other: Quat) -> Self::WithRotation {
        Transform {
            value: Mat4::from_quat(other) * Mat4::from_scale(*self),
            sync: false,
        }
    }

    fn then_scale(self, other: f32) -> Self::WithScale {
        NonUniformScale(other * *self)
    }

    fn then_translate(self, other: Vec3) -> Self::WithTranslation {
        Transform {
            value: Mat4::from_translation(other) * Mat4::from_scale(*self),
            sync: false,
        }
    }
}

impl ComposableTransform for Rotation {
    type WithNonUniformScale = Transform;
    type WithRotation = Rotation;
    type WithScale = Transform;
    type WithTranslation = Transform;

    fn then_transform(self, other: Mat4) -> Transform {
        Transform {
            value: other * Mat4::from_quat(*self),
            sync: false,
        }
    }

    fn then_non_uniform_scale(self, other: Vec3) -> Self::WithNonUniformScale {
        Transform {
            value: Mat4::from_scale(other) * Mat4::from_quat(*self),
            sync: false,
        }
    }

    fn then_rotate(self, other: Quat) -> Self::WithRotation {
        Rotation(other * *self)
    }

    fn then_scale(self, other: f32) -> Self::WithScale {
        Transform {
            value: Mat4::from_scale(Vec3::new(other, other, other)) * Mat4::from_quat(*self),
            sync: false,
        }
    }

    fn then_translate(self, other: Vec3) -> Self::WithTranslation {
        Transform {
            value: Mat4::from_translation(other) * Mat4::from_quat(*self),
            sync: false,
        }
    }
}

impl ComposableTransform for Scale {
    type WithNonUniformScale = NonUniformScale;
    type WithRotation = Transform;
    type WithScale = Scale;
    type WithTranslation = Transform;

    fn then_transform(self, other: Mat4) -> Transform {
        Transform {
            value: other * Mat4::from_scale(Vec3::new(*self, *self, *self)),
            sync: false,
        }
    }

    fn then_non_uniform_scale(self, other: Vec3) -> Self::WithNonUniformScale {
        NonUniformScale(other * *self)
    }

    fn then_rotate(self, other: Quat) -> Self::WithRotation {
        Transform {
            value: Mat4::from_quat(other) * Mat4::from_scale(Vec3::new(*self, *self, *self)),
            sync: false,
        }
    }

    fn then_scale(self, other: f32) -> Self::WithScale {
        Scale(other * *self)
    }

    fn then_translate(self, other: Vec3) -> Self::WithTranslation {
        Transform {
            value: Mat4::from_translation(other) * Mat4::from_scale(Vec3::new(*self, *self, *self)),
            sync: false,
        }
    }
}

impl ComposableTransform for Translation {
    type WithNonUniformScale = Transform;
    type WithRotation = Transform;
    type WithScale = Transform;
    type WithTranslation = Translation;

    fn then_transform(self, other: Mat4) -> Transform {
        Transform {
            value: other * Mat4::from_translation(*self),
            sync: false,
        }
    }

    fn then_non_uniform_scale(self, other: Vec3) -> Self::WithNonUniformScale {
        Transform {
            value: Mat4::from_scale(other) * Mat4::from_translation(*self),
            sync: false,
        }
    }

    fn then_rotate(self, other: Quat) -> Self::WithRotation {
        Transform {
            value: Mat4::from_quat(other) * Mat4::from_translation(*self),
            sync: false,
        }
    }

    fn then_scale(self, other: f32) -> Self::WithScale {
        Transform {
            value: Mat4::from_scale(Vec3::new(other, other, other)) * Mat4::from_translation(*self),
            sync: false,
        }
    }

    fn then_translate(self, other: Vec3) -> Self::WithTranslation {
        Translation(other + *self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_compose_transform() {
        let comp = Transform::new(Mat4::from_rotation_x(PI / 3.0))
            .then_transform(Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)));
        let expected = Mat4::from_rotation_translation(
            Quat::from_rotation_x(PI / 3.0),
            Vec3::new(1.0, 2.0, 3.0),
        );
        assert!(
            comp.value.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            comp.value,
            expected
        );
    }

    #[test]
    fn test_compose_non_uniform_scale() {
        let comp =
            NonUniformScale::new(1.0, 2.0, 3.0).then_non_uniform_scale(Vec3::new(4.0, 5.0, 6.0));
        let expected = Vec3::new(4.0, 10.0, 18.0);
        assert!(
            comp.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            *comp,
            expected
        );

        let comp = NonUniformScale::new(1.0, 2.0, 3.0).then_scale(4.0);
        let expected = Vec3::new(4.0, 8.0, 12.0);
        assert!(
            comp.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            *comp,
            expected
        );
    }

    #[test]
    fn test_compose_rotation() {
        let comp =
            Rotation(Quat::from_rotation_y(PI / 2.0)).then_rotate(Quat::from_rotation_x(PI / 2.0));
        let expected = Quat::from_axis_angle(Vec3::new(1.0, 1.0, 1.0).normalize(), 2.0 * PI / 3.0);
        assert!(
            comp.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            *comp,
            expected
        );

        let comp = Rotation(Quat::from_rotation_z(PI / 4.0))
            .then_rotate(Quat::from_rotation_x(PI / 3.0))
            .then_rotate(Quat::from_rotation_y(PI / 2.0));
        let expected = Quat::from_rotation_ypr(PI / 2.0, PI / 3.0, PI / 4.0);
        assert!(
            comp.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            *comp,
            expected
        );
    }

    #[test]
    fn test_compose_scale() {
        let comp = Scale(2.0).then_scale(3.0);
        let expected = 6.0;
        assert!(
            (*comp - expected).abs() < 0.0001,
            "{:?} != {:?}",
            *comp,
            expected
        );
    }

    #[test]
    fn test_compose_translation() {
        let comp = Translation::new(1.0, 2.0, 3.0).then_translate(Vec3::new(4.0, 5.0, 6.0));
        let expected = Vec3::new(5.0, 7.0, 9.0);
        assert!(
            comp.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            *comp,
            expected
        );
    }

    #[test]
    fn test_compose_all() {
        let comp = NonUniformScale::new(1.0, 2.0, 3.0)
            .then_scale(4.0)
            .then_rotate(Quat::from_rotation_ypr(5.0, 6.0, 7.0))
            .then_translate(Vec3::new(8.0, 9.0, 10.0));
        let expected = Mat4::from_scale_rotation_translation(
            Vec3::new(4.0, 8.0, 12.0),
            Quat::from_rotation_ypr(5.0, 6.0, 7.0),
            Vec3::new(8.0, 9.0, 10.0),
        );
        assert!(
            comp.value.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            comp.value,
            expected
        );
    }
}
