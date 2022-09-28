//! 3d Animation for the game engine Bevy

use bevy_math::{Quat, Vec3};

crate::anim_impl::anim_impl! {
  /// 3d

  type Rotation = Quat;
  type Translation = Vec3;
  type Scale = Vec3;

  struct AnimationClip;
  struct AnimationPlayer;
  struct AnimationPlugin;
  struct Keyframes;
  struct VariableCurve;
}

#[inline]
fn rotation_to_quatlike(rotation: Rotation) -> Quat {
    rotation
}
#[inline]
fn quatlike_to_quat(rotation: Quat) -> Quat {
    rotation
}
#[inline]
fn rotation_to_quat(rotation: Rotation) -> Quat {
    rotation
}
#[inline]
fn translation_to_vec3(translation: Translation) -> Vec3 {
    translation
}
#[inline]
fn scale_to_vec3(scale: Scale) -> Vec3 {
    scale
}
#[inline]
fn slerp_quatlike(a: Quat, b: Quat, s: f32) -> Quat {
    a.slerp(b, s)
}

include!("anim_impl.rs");
