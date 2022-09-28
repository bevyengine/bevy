//! 3d Animation for the game engine Bevy

#![warn(missing_docs)]

use bevy_app::{App, Plugin};
use bevy_asset::Assets;
use bevy_core::Name;
use bevy_ecs::{
    entity::Entity,
    prelude::Component,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_hierarchy::Children;
use bevy_math::{Quat, Vec3};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_time::Time;
use bevy_transform::prelude::Transform;
use bevy_utils::tracing::warn;

use crate::animation_impl::*;

/// Specifies a 3d animation.
#[derive(Clone, Debug)]
pub struct Animation3d;

impl AnimationImpl for Animation3d {
    type Keyframes = Keyframes;
    type VariableCurve = VariableCurve;
    type AnimationClip = AnimationClip;
    type AnimationPlayer = AnimationPlayer;

    fn assign(keyframes: &Self::Keyframes, index: usize, transform: &mut Transform) {
        match keyframes {
            Keyframes::Rotation(keyframes) => transform.rotation = keyframes[index],
            Keyframes::Translation(keyframes) => transform.translation = keyframes[index],
            Keyframes::Scale(keyframes) => transform.scale = keyframes[index],
        }
    }
    fn lerp(keyframes: &Self::Keyframes, step_start: usize, lerp: f32, transform: &mut Transform) {
        match keyframes {
            Keyframes::Rotation(keyframes) => {
                let rot_start = keyframes[step_start];
                let mut rot_end = keyframes[step_start + 1];
                // Choose the smallest angle for the rotation
                if rot_end.dot(rot_start) < 0.0 {
                    rot_end = -rot_end;
                }
                // Rotations are using a spherical linear interpolation
                transform.rotation = rot_start.normalize().slerp(rot_end.normalize(), lerp);
            }
            Keyframes::Translation(keyframes) => {
                let translation_start = keyframes[step_start];
                let translation_end = keyframes[step_start + 1];
                let result = translation_start.lerp(translation_end, lerp);
                transform.translation = result;
            }
            Keyframes::Scale(keyframes) => {
                let scale_start = keyframes[step_start];
                let scale_end = keyframes[step_start + 1];
                let result = scale_start.lerp(scale_end, lerp);
                transform.scale = result;
            }
        }
    }
    fn keyframe_timestamps(curve: &Self::VariableCurve) -> &[f32] {
        &curve.keyframe_timestamps
    }
    fn keyframes(curve: &Self::VariableCurve) -> &Self::Keyframes {
        &curve.keyframes
    }
}
type Impl = Animation3d;

/// List of keyframes for one of the 3d attributes of a [`Transform`].
#[derive(Clone, Debug)]
pub enum Keyframes {
    /// Keyframes for rotation.
    Rotation(Vec<Quat>),
    /// Keyframes for translation.
    Translation(Vec<Vec3>),
    /// Keyframes for scale.
    Scale(Vec<Vec3>),
}

/// Describes how an attribute of a [`Transform`] should be animated.
///
/// `keyframe_timestamps` and `keyframes` should have the same length.
#[derive(Clone, Debug)]
pub struct VariableCurve {
    /// Timestamp for each of the keyframes.
    pub keyframe_timestamps: Vec<f32>,
    /// List of the keyframes.
    pub keyframes: Keyframes,
}

/// A list of [`VariableCurve`], and the [`EntityPath`] to which they apply.
#[derive(Clone, TypeUuid, Debug, Default)]
#[uuid = "0f47d392-8d73-4b05-8810-0a775b1fccaa"]
#[repr(transparent)]
pub struct AnimationClip(AnimationClipImpl<Impl>);
wrapped_impl!(AnimationClip(AnimationClipImpl<Impl>));

/// Animation controls
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
#[repr(transparent)]
pub struct AnimationPlayer(AnimationPlayerImpl<Impl>);
wrapped_impl!(AnimationPlayer(AnimationPlayerImpl<Impl>));

/// System that will play all 3d animations, using any entity with a [`AnimationPlayer`]
/// and a [`Handle<AnimationClip>`] as an animation root
pub fn animation_player(
    time: Res<Time>,
    animations: Res<Assets<AnimationClip>>,
    animation_players: Query<(Entity, &mut AnimationPlayer)>,
    names: Query<&Name>,
    transforms: Query<&mut Transform>,
    children: Query<&Children>,
) {
    animation_player_impl::<Animation3d>(
        time,
        animations,
        animation_players,
        names,
        transforms,
        children,
    )
}

/// Adds 3d animation support to an app
#[derive(Default)]
pub struct AnimationPlugin {}
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        build_plugin_impl::<Animation3d>(app)
    }
}
