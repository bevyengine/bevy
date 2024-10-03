//! Demonstrate how to use animation events.

use bevy::{
    color::palettes::css::{ALICE_BLUE, BLACK, CRIMSON},
    core_pipeline::bloom::Bloom,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .observe(Say::observer)
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, animate_text_opacity)
        .run();
}

#[derive(Event, AnimationEvent, Reflect, Clone)]
#[reflect(AnimationEvent)]
enum Say {
    Hello,
    Bye,
}

impl Say {
    fn observer(trigger: Trigger<Self>, mut text: Query<&mut Text, With<MessageText>>) {
        let mut text = text.get_single_mut().unwrap();
        match trigger.event() {
            Say::Hello => {
                text.sections[0].style.color = ALICE_BLUE.into();
                text.sections[0].value = "HELLO".into();
                println!("HELLO");
            }
            Say::Bye => {
                text.sections[0].style.color = CRIMSON.into();
                text.sections[0].value = "BYE".into();
                println!("BYE");
            }
        }
    }
}

#[derive(Component)]
struct MessageText;

fn setup(
    mut commands: Commands,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Camera
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                clear_color: ClearColorConfig::Custom(BLACK.into()),
                hdr: true,
                ..Default::default()
            },
            ..Default::default()
        },
        Bloom {
            intensity: 0.4,
            ..Bloom::NATURAL
        },
    ));

    // The text that will be changed by animation events.
    commands.spawn((
        MessageText,
        Text2dBundle {
            text: Text::from_section(
                "",
                TextStyle {
                    font_size: 119.0,
                    color: Color::NONE,
                    ..Default::default()
                },
            ),
            ..Default::default()
        },
    ));

    // Create a new animation clip.
    let mut animation = AnimationClip::default();

    // This is only necessary if you want the duration of the
    // animation to be longer than the last event in the clip.
    animation.set_duration(2.0);

    // Add events at the specified time.
    animation.add_event(0.0, Say::Hello);
    animation.add_event(1.0, Say::Bye);

    // Create the animation graph.
    let (graph, animation_index) = AnimationGraph::from_clip(animations.add(animation));
    let mut player = AnimationPlayer::default();
    player.play(animation_index).repeat();

    commands.spawn((graphs.add(graph), player));
}

// Slowly fade out the text opacity.
fn animate_text_opacity(mut query: Query<&mut Text>, time: Res<Time>) {
    for mut text in &mut query {
        let color = &mut text.sections[0].style.color;
        let a = color.alpha();
        color.set_alpha(a - time.delta_seconds());
    }
}
