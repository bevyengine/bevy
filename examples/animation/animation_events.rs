//! Demonstrate how to use animation events.

use bevy::{
    animation::animation_event::{AnimationEvent, ReflectAnimationEvent},
    color::palettes::css::{ALICE_BLUE, CRIMSON},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .register_type::<Say>()
        .observe(Say::observer)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_text_opacity)
        .run();
}

#[derive(Event, Reflect, Clone)]
#[reflect(AnimationEvent)]
enum Say {
    Hello(Entity),
    Bye(Entity),
}

impl Say {
    fn entity(&self) -> Entity {
        match self {
            Say::Hello(entity) | Say::Bye(entity) => *entity,
        }
    }
}

impl Say {
    fn observer(trigger: Trigger<Self>, mut query: Query<&mut Text>) {
        let mut text = query.get_mut(trigger.event().entity()).unwrap();

        match trigger.event() {
            Say::Hello(_) => {
                text.sections[0].style.color = ALICE_BLUE.into();
                text.sections[0].value = "HELLO".into();
                println!("HELLO");
            }
            Say::Bye(_) => {
                text.sections[0].style.color = CRIMSON.into();
                text.sections[0].value = "BYE".into();
                println!("BYE");
            }
        }
    }
}

impl AnimationEvent for Say {
    fn trigger(&self, _player: Entity, _time: f32, target: Entity, world: &mut World) {
        world.entity_mut(target).trigger(self.clone());
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
    // Camera
    commands.spawn(Camera2dBundle::default());

    // A text entity that will have a message printed to it
    let message = commands
        .spawn(TextBundle {
            text: Text::from_section(
                "",
                TextStyle {
                    font_size: 71.0,
                    color: Color::NONE,
                    ..Default::default()
                },
            ),
            ..Default::default()
        })
        .id();

    // Container node for the message
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            ..Default::default()
        })
        .add_child(message);

    // Create a new animation clip
    let mut animation = AnimationClip::default();
    animation.set_duration(2.0);

    // Add events at the specified time
    // If `time` is `0.0` it will trigger twice on the first tick for some reason
    animation.add_event(0.001, Say::Hello(message));
    animation.add_event(1.0, Say::Bye(message));

    let (graph, animation_index) = AnimationGraph::from_clip(animations.add(animation));
    let mut player = AnimationPlayer::default();
    player.play(animation_index).repeat();

    commands.spawn((graphs.add(graph), player));
}

fn animate_text_opacity(mut query: Query<&mut Text>, time: Res<Time>) {
    for mut text in &mut query {
        let color = &mut text.sections[0].style.color;
        let a = color.alpha();
        color.set_alpha(a - time.delta_seconds());
    }
}
