//! 2d Animation for the game engine Bevy

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
use bevy_math::{Quat, Vec2};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_time::Time;
use bevy_transform::prelude::Transform;
use bevy_utils::tracing::warn;

use crate::animation_impl::*;

/// Specifies a 2d animation.
#[derive(Clone, Debug)]
pub struct Animation2d;

impl AnimationImpl for Animation2d {
    type Keyframes = Keyframes2d;
    type VariableCurve = VariableCurve2d;
    type AnimationClip = AnimationClip2d;
    type AnimationPlayer = AnimationPlayer2d;

    fn assign(keyframes: &Self::Keyframes, index: usize, transform: &mut Transform) {
        match keyframes {
            Keyframes2d::Rotation(keyframes) => {
                transform.rotation = Quat::from_rotation_z(keyframes[index])
            }
            Keyframes2d::Translation(keyframes) => {
                transform.translation = (keyframes[index], 0.0).into()
            }
            Keyframes2d::Scale(keyframes) => transform.scale = (keyframes[index], 1.0).into(),
        }
    }
    fn lerp(keyframes: &Self::Keyframes, step_start: usize, lerp: f32, transform: &mut Transform) {
        match keyframes {
            Keyframes2d::Rotation(keyframes) => {
                let rot_start = keyframes[step_start];
                let rot_end = keyframes[step_start + 1];

                let start = Vec2::from((rot_start * 0.5).sin_cos()).normalize();
                let mut end = Vec2::from((rot_end * 0.5).sin_cos()).normalize();

                // Choose the smallest angle for the rotation
                if end.dot(start) < 0.0 {
                    end = -end;
                }

                // Rotations are using linear interpolation
                let interpolated = (start + (end - start) * lerp).normalize();
                transform.rotation = Quat::from_xyzw(0.0, 0.0, interpolated.x, interpolated.y);
            }
            Keyframes2d::Translation(keyframes) => {
                let translation_start = keyframes[step_start];
                let translation_end = keyframes[step_start + 1];
                let result = translation_start.lerp(translation_end, lerp);
                transform.translation = (result, 0.0).into();
            }
            Keyframes2d::Scale(keyframes) => {
                let scale_start = keyframes[step_start];
                let scale_end = keyframes[step_start + 1];
                let result = scale_start.lerp(scale_end, lerp);
                transform.scale = (result, 1.0).into();
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
type Impl = Animation2d;

/// List of keyframes for one of the 2d attributes of a [`Transform`].
#[derive(Clone, Debug)]
pub enum Keyframes2d {
    /// Keyframes for rotation.
    Rotation(Vec<f32>),
    /// Keyframes for translation.
    Translation(Vec<Vec2>),
    /// Keyframes for scale.
    Scale(Vec<Vec2>),
}

/// Describes how an attribute of a [`Transform`] should be animated.
///
/// `keyframe_timestamps` and `keyframes` should have the same length.
#[derive(Clone, Debug)]
pub struct VariableCurve2d {
    /// Timestamp for each of the keyframes.
    pub keyframe_timestamps: Vec<f32>,
    /// List of the keyframes.
    pub keyframes: Keyframes2d,
}

/// A list of [`VariableCurve`], and the [`EntityPath`] to which they apply.
#[derive(Clone, TypeUuid, Debug, Default)]
#[uuid = "d81b7179-0448-4eb0-89fe-c067222725bf"]
#[repr(transparent)]
pub struct AnimationClip2d(AnimationClipImpl<Impl>);
wrapped_impl!(AnimationClip2d(AnimationClipImpl<Impl>));

/// Animation controls
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
#[repr(transparent)]
pub struct AnimationPlayer2d(AnimationPlayerImpl<Impl>);
wrapped_impl!(AnimationPlayer2d(AnimationPlayerImpl<Impl>));

/// System that will play all 2d animations, using any entity with a [`AnimationPlayer`]
/// and a [`Handle<AnimationClip>`] as an animation root
pub fn animation_player2d(
    time: Res<Time>,
    animations: Res<Assets<AnimationClip2d>>,
    animation_players: Query<(Entity, &mut AnimationPlayer2d)>,
    names: Query<&Name>,
    transforms: Query<&mut Transform>,
    children: Query<&Children>,
) {
    animation_player_impl::<Animation2d>(
        time,
        animations,
        animation_players,
        names,
        transforms,
        children,
    )
}

/// Adds 2d animation support to an app
#[derive(Default)]
pub struct AnimationPlugin2d {}
impl Plugin for AnimationPlugin2d {
    fn build(&self, app: &mut App) {
        build_plugin_impl::<Animation2d>(app)
    }
}
