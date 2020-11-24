use crate::{
    spline_group::SplineGroup,
    spline_groups::{
        one::AnimationSplineOne, three::AnimationSplineThree, transform::AnimationSplineTransform,
    },
};

use bevy_app::{AppBuilder, Plugin};
use bevy_core::Time;
use bevy_ecs::{IntoSystem, Query, Res};
use bevy_math::{Quat, Vec3};
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
        let s = splines.current();
        s.translation.alter(&mut transform.translation);
        if let Some(sample_scale) = s.scale {
            scale = Vec3::one() * sample_scale;
        }
        let mut rot = Vec3::zero();
        s.rotation.alter(&mut rot);
        *transform = Transform::from_translation(transform.translation);
        transform.rotation = Quat::from_rotation_ypr(rot.x, rot.y, rot.z);
        transform.apply_non_uniform_scale(scale);
    }
}
