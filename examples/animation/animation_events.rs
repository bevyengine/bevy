//! Demonstrate how to use animation events.

use bevy::{
    animation::events::{AnimationEvent, ReflectAnimationEvent},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .register_type::<Say>()
        .observe(Say::observer)
        .run();
}

#[derive(Event, Reflect, Clone)]
#[reflect(AnimationEvent)]
enum Say {
    Hello,
    Bye,
}

impl Say {
    fn observer(trigger: Trigger<Self>) {
        match trigger.event() {
            Say::Hello => println!("HELLO!"),
            Say::Bye => println!("BYE!"),
        }
    }
}

impl AnimationEvent for Say {
    fn trigger(&self, entity: Entity, world: &mut World) {
        world.entity_mut(entity).trigger(self.clone());
    }

    fn clone_value(&self) -> Box<dyn AnimationEvent> {
        Box::new(self.clone())
    }
}

fn setup(
    mut commands: Commands,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut animation = AnimationClip::default();

    animation.add_event(1.0, Say::Hello);
    animation.add_event(2.0, Say::Bye);

    let (graph, animation_index) = AnimationGraph::from_clip(animations.add(animation));
    let mut player = AnimationPlayer::default();
    player.play(animation_index).repeat();

    commands.spawn((graphs.add(graph), player));
}
