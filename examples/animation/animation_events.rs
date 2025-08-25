//! Demonstrate how to use animation events.

use bevy::{
    color::palettes::css::{ALICE_BLUE, BLACK, CRIMSON},
    core_pipeline::bloom::Bloom,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_text_opacity)
        .add_observer(edit_message)
        .run();
}

#[derive(Component)]
struct MessageText;

#[derive(EntityEvent, Clone)]
struct MessageEvent {
    value: String,
    color: Color,
}

fn edit_message(
    event: On<MessageEvent>,
    text: Single<(&mut Text2d, &mut TextColor), With<MessageText>>,
) {
    let (mut text, mut color) = text.into_inner();
    text.0 = event.value.clone();
    color.0 = event.color;
}

fn setup(
    mut commands: Commands,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Camera
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(BLACK.into()),
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
        Text2d::default(),
        TextFont {
            font_size: 119.0,
            ..default()
        },
        TextColor(Color::NONE),
    ));

    // Create a new animation clip.
    let mut animation = AnimationClip::default();

    // This is only necessary if you want the duration of the
    // animation to be longer than the last event in the clip.
    animation.set_duration(2.0);

    // Add events at the specified time.
    animation.add_event(
        0.0,
        MessageEvent {
            value: "HELLO".into(),
            color: ALICE_BLUE.into(),
        },
    );
    animation.add_event(
        1.0,
        MessageEvent {
            value: "BYE".into(),
            color: CRIMSON.into(),
        },
    );

    // Create the animation graph.
    let (graph, animation_index) = AnimationGraph::from_clip(animations.add(animation));
    let mut player = AnimationPlayer::default();
    player.play(animation_index).repeat();

    commands.spawn((AnimationGraphHandle(graphs.add(graph)), player));
}

// Slowly fade out the text opacity.
fn animate_text_opacity(mut colors: Query<&mut TextColor>, time: Res<Time>) {
    for mut color in &mut colors {
        let a = color.0.alpha();
        color.0.set_alpha(a - time.delta_secs());
    }
}
