use crate::animation_spline::{AnimationSpline, AnimationSplineThree};
use bevy_app::{AppBuilder, Plugin};
use bevy_core::Time;
use bevy_ecs::{IntoForEachSystem, Mut, Res};

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(advance_animation_spline.system())
            .add_system(advance_animation_spline_three.system());
    }
}

impl Default for AnimationPlugin {
    fn default() -> Self {
        Self
    }
}

fn advance_animation_spline(time: Res<Time>, mut animation_spline: Mut<AnimationSpline>) {
    animation_spline.advance(time.delta_seconds);
}

fn advance_animation_spline_three(
    time: Res<Time>,
    mut animation_spline: Mut<AnimationSplineThree>,
) {
    animation_spline.advance(time.delta_seconds);
}
