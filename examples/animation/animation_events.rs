//! Demonstrate how to use animation events.

use bevy::{
    color::palettes::css::{ALICE_BLUE, BLACK, CRIMSON},
    core_pipeline::bloom::Bloom,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_event::<MessageEvent>()
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, (animate_text_opacity, edit_message))
        .run();
}

#[derive(Component)]
struct MessageText;

#[derive(Event, Reflect, Clone)]
#[reflect(AnimationEvent)]
struct MessageEvent {
    value: String,
    color: Color,
}

// AnimationEvent can also be derived, but doing so will
// trigger it as an observer event which is triggered in PostUpdate.
// We need to set the message text before that so it is
// updated before rendering without a one frame delay.
impl AnimationEvent for MessageEvent {
    fn trigger(&self, _time: f32, _weight: f32, _entity: Entity, world: &mut World) {
        world.send_event(self.clone());
    }
}

fn edit_message(
    mut event_reader: EventReader<MessageEvent>,
    mut text: Single<&mut Text, With<MessageText>>,
) {
    for event in event_reader.read() {
        text.sections[0].value = event.value.clone();
        text.sections[0].style.color = event.color;
    }
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
            hdr: true,
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
fn animate_text_opacity(mut query: Query<&mut Text>, time: Res<Time>) {
    for mut text in &mut query {
        let color = &mut text.sections[0].style.color;
        let a = color.alpha();
        color.set_alpha(a - time.delta_seconds());
    }
}
