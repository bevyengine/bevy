//! How to use an external thread to run an infinite task and communicate with a channel.

use bevy::prelude::*;
// Using crossbeam_channel instead of std as std `Receiver` is `!Sync`
use crossbeam_channel::{bounded, Receiver};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::time::{Duration, Instant};

fn main() {
    App::new()
        .add_event::<StreamEvent>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (read_stream, spawn_text, move_text))
        .run();
}

#[derive(Resource, Deref)]
struct StreamReceiver(Receiver<u32>);

#[derive(Event)]
struct StreamEvent(u32);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    let (tx, rx) = bounded::<u32>(10);
    std::thread::spawn(move || {
        let mut rng = StdRng::seed_from_u64(19878367467713);
        loop {
            // Everything here happens in another thread
            // This is where you could connect to an external data source
            let start_time = Instant::now();
            let duration = Duration::from_secs_f32(rng.gen_range(0.0..0.2));
            while start_time.elapsed() < duration {
                // Spinning for 'duration', simulating doing hard work!
            }

            tx.send(rng.gen_range(0..2000)).unwrap();
        }
    });

    commands.insert_resource(StreamReceiver(rx));
}

// This system reads from the receiver and sends events to Bevy
fn read_stream(receiver: Res<StreamReceiver>, mut events: EventWriter<StreamEvent>) {
    for from_stream in receiver.try_iter() {
        events.send(StreamEvent(from_stream));
    }
}

fn spawn_text(mut commands: Commands, mut reader: EventReader<StreamEvent>) {
    let text_style = TextStyle {
        font_size: 20.0,
        color: Color::WHITE,
        ..default()
    };

    for (per_frame, event) in reader.read().enumerate() {
        commands.spawn(Text2dBundle {
            text: Text::from_section(event.0.to_string(), text_style.clone())
                .with_alignment(TextAlignment::Center),
            transform: Transform::from_xyz(per_frame as f32 * 100.0, 300.0, 0.0),
            ..default()
        });
    }
}

fn move_text(
    mut commands: Commands,
    mut texts: Query<(Entity, &mut Transform), With<Text>>,
    time: Res<Time>,
) {
    for (entity, mut position) in &mut texts {
        position.translation -= Vec3::new(0.0, 100.0 * time.delta_seconds(), 0.0);
        if position.translation.y < -300.0 {
            commands.entity(entity).despawn();
        }
    }
}
