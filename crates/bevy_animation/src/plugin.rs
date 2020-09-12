use crate::{
    spline_group::SplineGroup,
    spline_groups::{
        one::AnimationSplineOne, three::AnimationSplineThree, transform::AnimationSplineTransform,
    },
};

use bevy_app::{AppBuilder, Plugin};
use bevy_core::Time;
use bevy_ecs::{IntoForEachSystem, Mut, Res};
use bevy_math::Vec3;
use bevy_transform::components::{Rotation, Scale, Translation};

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
    mut translation: Mut<Translation>,
    mut rotation: Mut<Rotation>,
    mut scale: Mut<Scale>,
    mut splines: Mut<AnimationSplineTransform>,
) {
    splines.advance(time.delta_seconds);
    let s = splines.current();
    s.translation.alter(&mut translation.0);
    if let Some(sample_scale) = s.scale {
        scale.0 = sample_scale;
    }
    let mut rot = Vec3::zero();
    s.rotation.alter(&mut rot);
    *rotation = Rotation::from_rotation_xyz(rot.x(), rot.y(), rot.z());
}
