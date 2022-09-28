//! 2d Animation for the game engine Bevy

use bevy_math::{Quat, Vec2, Vec3};

crate::anim_impl::anim_impl! {
  /// 2d

  type Rotation = f32;
  type Translation = Vec2;
  type Scale = Vec2;

  struct AnimationClip2d;
  struct AnimationPlayer2d;
  struct AnimationPlugin2d;
  struct Keyframes2d;
  struct VariableCurve2d;
}

#[inline]
fn rotation_to_quatlike(rotation: Rotation) -> Vec2 {
    (rotation * 0.5).sin_cos().into()
}
#[inline]
fn quatlike_to_quat(rotation: Vec2) -> Quat {
    Quat::from_xyzw(0.0, 0.0, rotation.x, rotation.y)
}
#[inline]
fn rotation_to_quat(rotation: Rotation) -> Quat {
    Quat::from_rotation_z(rotation)
}
#[inline]
fn translation_to_vec3(translation: Translation) -> Vec3 {
    (translation, 0.0).into()
}
#[inline]
fn scale_to_vec3(scale: Scale) -> Vec3 {
    (scale, 1.0).into()
}
#[inline]
fn slerp_quatlike(a: Vec2, b: Vec2, s: f32) -> Vec2 {
    let start = a.normalize();
    let mut end = b.normalize();

    // Choose the smallest angle for the rotation
    if end.dot(start) < 0.0 {
        end = -end;
    }

    // Rotations are using linear interpolation
    (start + (end - start) * s).normalize()
}
