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
    /// Rotates the -z axis to point at center, with the +y axis in the
    /// plane spanned by -z and up.
    fn then_look_at(self, center: Vec3, up: Vec3) -> Self::WithRotation;
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
            value: Mat4::from_scale(Vec3::splat(other)) * self.value,
            sync: false,
        }
    }

    fn then_translate(self, other: Vec3) -> Self::WithTranslation {
        Transform {
            value: Mat4::from_translation(other) * self.value,
            sync: false,
        }
    }

    fn then_look_at(self, center: Vec3, up: Vec3) -> Self::WithRotation {
        let (scale, _, translation) = self.value.to_scale_rotation_translation();
        let (_, rotation, _) =
            Mat4::look_at_rh(translation, center, up).to_scale_rotation_translation();
        Transform {
            value: Mat4::from_scale_rotation_translation(scale, rotation, translation),
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

    fn then_look_at(self, center: Vec3, up: Vec3) -> Self::WithRotation {
        let (_, rotation, _) =
            Mat4::look_at_rh(Vec3::zero(), center, up).to_scale_rotation_translation();
        Transform {
            value: Mat4::from_scale_rotation_translation(*self, rotation, Vec3::zero()),
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
            value: Mat4::from_scale(Vec3::splat(other)) * Mat4::from_quat(*self),
            sync: false,
        }
    }

    fn then_translate(self, other: Vec3) -> Self::WithTranslation {
        Transform {
            value: Mat4::from_translation(other) * Mat4::from_quat(*self),
            sync: false,
        }
    }

    fn then_look_at(self, center: Vec3, up: Vec3) -> Self::WithRotation {
        let (_, rotation, _) =
            Mat4::look_at_rh(Vec3::zero(), center, up).to_scale_rotation_translation();
        Rotation(rotation)
    }
}

impl ComposableTransform for Scale {
    type WithNonUniformScale = NonUniformScale;
    type WithRotation = Transform;
    type WithScale = Scale;
    type WithTranslation = Transform;

    fn then_transform(self, other: Mat4) -> Transform {
        Transform {
            value: other * Mat4::from_scale(Vec3::splat(*self)),
            sync: false,
        }
    }

    fn then_non_uniform_scale(self, other: Vec3) -> Self::WithNonUniformScale {
        NonUniformScale(other * *self)
    }

    fn then_rotate(self, other: Quat) -> Self::WithRotation {
        Transform {
            value: Mat4::from_quat(other) * Mat4::from_scale(Vec3::splat(*self)),
            sync: false,
        }
    }

    fn then_scale(self, other: f32) -> Self::WithScale {
        Scale(other * *self)
    }

    fn then_translate(self, other: Vec3) -> Self::WithTranslation {
        Transform {
            value: Mat4::from_translation(other) * Mat4::from_scale(Vec3::splat(*self)),
            sync: false,
        }
    }

    fn then_look_at(self, center: Vec3, up: Vec3) -> Self::WithRotation {
        let (_, rotation, _) =
            Mat4::look_at_rh(Vec3::zero(), center, up).to_scale_rotation_translation();
        Transform {
            value: Mat4::from_scale_rotation_translation(
                Vec3::splat(*self),
                rotation,
                Vec3::zero(),
            ),
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
            value: Mat4::from_scale(Vec3::splat(other)) * Mat4::from_translation(*self),
            sync: false,
        }
    }

    fn then_translate(self, other: Vec3) -> Self::WithTranslation {
        Translation(other + *self)
    }

    fn then_look_at(self, center: Vec3, up: Vec3) -> Self::WithRotation {
        let (_, rotation, _) = Mat4::look_at_rh(*self, center, up).to_scale_rotation_translation();
        Transform {
            value: Mat4::from_scale_rotation_translation(Vec3::one(), rotation, *self),
            sync: false,
        }
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

    #[test]
    fn test_look_at() {
        let comp = Transform::new(Mat4::from_scale_rotation_translation(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_rotation_ypr(4.0, 5.0, 6.0),
            Vec3::new(1.0, 0.0, -1.0),
        ))
        .then_look_at(Vec3::zero(), Vec3::unit_y());
        let expected = Mat4::from_scale_rotation_translation(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_rotation_y(5.0 * PI / 4.0),
            Vec3::new(1.0, 0.0, -1.0),
        );
        assert!(
            comp.value.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            comp.value,
            expected
        );

        let comp =
            NonUniformScale::new(1.0, 2.0, 3.0).then_look_at(-Vec3::unit_x(), Vec3::unit_y());
        let expected = Mat4::from_scale_rotation_translation(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_rotation_y(-PI / 2.0),
            Vec3::zero(),
        );
        assert!(
            comp.value.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            comp.value,
            expected,
        );

        let comp = Rotation(Quat::from_rotation_ypr(1.0, 2.0, 3.0))
            .then_look_at(-Vec3::unit_z(), Vec3::unit_y());
        let expected = Quat::identity();
        assert!(
            comp.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            *comp,
            expected
        );

        let comp = Scale(2.0).then_look_at(-Vec3::unit_x(), Vec3::unit_y());
        let expected = Mat4::from_scale_rotation_translation(
            Vec3::splat(2.0),
            Quat::from_rotation_y(-PI / 2.0),
            Vec3::zero(),
        );
        assert!(
            comp.value.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            comp.value,
            expected
        );

        let comp = Translation::new(1.0, 0.0, -1.0).then_look_at(Vec3::zero(), Vec3::unit_y());
        let expected = Mat4::from_scale_rotation_translation(
            Vec3::one(),
            Quat::from_rotation_y(5.0 * PI / 4.0),
            Vec3::new(1.0, 0.0, -1.0),
        );
        assert!(
            comp.value.abs_diff_eq(expected, 0.0001),
            "{:?} != {:?}",
            comp.value,
            expected
        );
    }
}
