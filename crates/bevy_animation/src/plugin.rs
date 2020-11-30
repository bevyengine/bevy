use crate::{
    spline_group::SplineGroup,
    spline_groups::{
        one::AnimationSplineOne, three::AnimationSplineThree, transform::AnimationSplineTransform,
    },
};

use bevy_app::{AppBuilder, Plugin};
use bevy_core::Time;
use bevy_ecs::{IntoSystem, Query, Res};
use bevy_math::Vec3;
use bevy_transform::components::Transform;

#[derive(Default)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(advance_animation_spline)
            .add_system(advance_animation_spline_three.system())
            .add_system(advance_animation_transform.system());
    }
}

fn advance_animation_spline(time: Res<Time>, mut q: Query<&mut AnimationSplineOne>) {
    for mut animation_spline in q.iter_mut() {
        animation_spline.advance(time.delta_seconds);
    }
}

fn advance_animation_spline_three(time: Res<Time>, mut q: Query<&mut AnimationSplineThree>) {
    for mut animation_spline in q.iter_mut() {
        animation_spline.advance(time.delta_seconds);
    }
}

fn advance_animation_transform(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &mut AnimationSplineTransform)>,
) {
    for (mut transform, mut splines) in q.iter_mut() {
        let mut scale = transform.scale;
        splines.advance(time.delta_seconds);
        let sample = splines.current();
        sample.translation.alter(&mut transform.translation);
        if let Some(sample_scale) = sample.scale {
            scale = Vec3::one() * sample_scale;
        }
        *transform = Transform::from_translation(transform.translation);
        if let Some(rotation) = sample.rotation {
            transform.rotation = rotation;
        }
        transform.apply_non_uniform_scale(scale);
    }
}
