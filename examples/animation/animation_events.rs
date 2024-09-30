// TODO: rename to animation_events

use bevy::{
    animation::{
        events::{AnimationEvent, ReflectAnimationEvent},
        AnimationTarget, AnimationTargetId,
    },
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

impl AnimationEvent for Say {}

fn setup(
    mut commands: Commands,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut animation = AnimationClip::default();

    let name = Name::new("abc");
    let id = AnimationTargetId::from(&name);
    animation.add_event_with_id(id, 1.0, Say::Hello);
    animation.add_event_with_id(id, 2.0, Say::Bye);

    let (graph, animation_index) = AnimationGraph::from_clip(animations.add(animation));
    let mut player = AnimationPlayer::default();
    player.play(animation_index).repeat();

    let player = commands.spawn((name, graphs.add(graph), player)).id();
    commands
        .entity(player)
        .insert(AnimationTarget { id, player });
}
