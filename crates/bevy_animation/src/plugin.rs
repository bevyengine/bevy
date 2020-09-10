use crate::animator::Animator;
use bevy_app::{AppBuilder, Plugin};
use bevy_core::Time;
use bevy_ecs::{IntoQuerySystem, Query, Res};
use bevy_transform::components::Translation;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(animate_translation.system());
    }
}

impl Default for AnimationPlugin {
    fn default() -> Self {
        Self
    }
}

fn animate_translation(
    time: Res<Time>,
    mut q: Query<(&mut Animator<Translation>, &mut Translation)>,
) {
    for (mut animator, mut component) in &mut q.iter() {
        animator.progress(&mut component, time.delta);
    }
}
