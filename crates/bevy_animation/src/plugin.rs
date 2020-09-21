use crate::{
    spline_group::SplineGroup,
    spline_groups::{
        one::AnimationSplineOne, three::AnimationSplineThree, transform::AnimationSplineTransform,
    },
};

use bevy_app::{AppBuilder, Plugin};
use bevy_core::Time;
use bevy_ecs::{IntoForEachSystem, Mut, Res};
use bevy_math::{Quat, Vec3};
use bevy_transform::components::Transform;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(advance_animation_spline.system())
            .add_system(advance_animation_spline_three.system())
            .add_system(advance_animation_transform.system());
    }
}

impl Default for AnimationPlugin {
    fn default() -> Self {
        Self
    }
}

fn advance_animation_spline(time: Res<Time>, mut animation_spline: Mut<AnimationSplineOne>) {
    animation_spline.advance(time.delta_seconds);
}

fn advance_animation_spline_three(
    time: Res<Time>,
    mut animation_spline: Mut<AnimationSplineThree>,
) {
    animation_spline.advance(time.delta_seconds);
}

fn advance_animation_transform(
    time: Res<Time>,
    mut transform: Mut<Transform>,
    mut splines: Mut<AnimationSplineTransform>,
) {
    let mut translation = transform.translation();
    // let mut rotation = transform.rotation();
    let mut scale = transform.scale();
    splines.advance(time.delta_seconds);
    let s = splines.current();
    s.translation.alter(&mut translation);
    if let Some(sample_scale) = s.scale {
        scale = Vec3::one() * sample_scale;
    }
    let mut rot = Vec3::zero();
    s.rotation.alter(&mut rot);
    *transform = Transform::from_translation(translation)
        .with_rotation(Quat::from_rotation_ypr(rot.x(), rot.y(), rot.z()))
        .with_apply_non_uniform_scale(scale);
}
