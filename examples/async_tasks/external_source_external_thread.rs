//! How to use an external thread to run an infinite task and communicate with a channel.

use bevy::prelude::*;
// Using crossbeam_channel instead of std as std `Receiver` is `!Sync`
use crossbeam_channel::{bounded, Receiver};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_event::<StreamEvent>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (spawn_text, move_text))
        .add_systems(FixedUpdate, read_stream)
        .insert_resource(Time::<Fixed>::from_seconds(0.5))
        .run();
}

#[derive(Resource, Deref)]
struct StreamReceiver(Receiver<u32>);

#[derive(BufferedEvent)]
struct StreamEvent(u32);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let (tx, rx) = bounded::<u32>(1);
    std::thread::spawn(move || {
        // We're seeding the PRNG here to make this example deterministic for testing purposes.
        // This isn't strictly required in practical use unless you need your app to be deterministic.
        let mut rng = ChaCha8Rng::seed_from_u64(19878367467713);
        loop {
            // Everything here happens in another thread
            // This is where you could connect to an external data source

            // This will block until the previous value has been read in system `read_stream`
            tx.send(rng.random_range(0..2000)).unwrap();
        }
    });

    commands.insert_resource(StreamReceiver(rx));
}

// This system reads from the receiver and sends events to Bevy
fn read_stream(receiver: Res<StreamReceiver>, mut events: EventWriter<StreamEvent>) {
    for from_stream in receiver.try_iter() {
        events.write(StreamEvent(from_stream));
    }
}

fn spawn_text(mut commands: Commands, mut reader: EventReader<StreamEvent>) {
    for (per_frame, event) in reader.read().enumerate() {
        commands.spawn((
            Text2d::new(event.0.to_string()),
            TextLayout::new_with_justify(Justify::Center),
            Transform::from_xyz(per_frame as f32 * 100.0, 300.0, 0.0),
        ));
    }
}

fn move_text(
    mut commands: Commands,
    mut texts: Query<(Entity, &mut Transform), With<Text2d>>,
    time: Res<Time>,
) {
    for (entity, mut position) in &mut texts {
        position.translation -= Vec3::new(0.0, 100.0 * time.delta_secs(), 0.0);
        if position.translation.y < -300.0 {
            commands.entity(entity).despawn();
        }
    }
}
