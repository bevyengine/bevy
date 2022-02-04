use bevy::prelude::*;
// Using crossbeam_channel instead of std as std `Receiver` is `!Sync`
use crossbeam_channel::{bounded, Receiver};
use rand::Rng;
use std::time::{Duration, Instant};

fn main() {
    App::new()
        .add_event::<StreamEvent>()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(read_stream)
        .add_system(spawn_text)
        .add_system(move_text)
        .run();
}

struct StreamReceiver(Receiver<u32>);
struct StreamEvent(u32);

struct LoadedFont(Handle<Font>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let (tx, rx) = bounded::<u32>(10);
    std::thread::spawn(move || loop {
        // Everything here happens in another thread
        // This is where you could connect to an external data source
        let mut rng = rand::thread_rng();
        let start_time = Instant::now();
        let duration = Duration::from_secs_f32(rng.gen_range(0.0..0.2));
        while Instant::now() - start_time < duration {
            // Spinning for 'duration', simulating doing hard work!
        }

        tx.send(rng.gen_range(0..2000)).unwrap();
    });

    commands.insert_resource(StreamReceiver(rx));
    commands.insert_resource(LoadedFont(asset_server.load("fonts/FiraSans-Bold.ttf")));
}

// This system reads from the receiver and sends events to Bevy
fn read_stream(receiver: ResMut<StreamReceiver>, mut events: EventWriter<StreamEvent>) {
    for from_stream in receiver.0.try_iter() {
        events.send(StreamEvent(from_stream))
    }
}

fn spawn_text(
    mut commands: Commands,
    mut reader: EventReader<StreamEvent>,
    loaded_font: Res<LoadedFont>,
) {
    let text_style = TextStyle {
        font: loaded_font.0.clone(),
        font_size: 20.0,
        color: Color::WHITE,
    };
    let text_alignment = TextAlignment {
        vertical: VerticalAlign::Center,
        horizontal: HorizontalAlign::Center,
    };
    for (per_frame, event) in reader.iter().enumerate() {
        commands.spawn_bundle(Text2dBundle {
            text: Text::with_section(format!("{}", event.0), text_style.clone(), text_alignment),
            transform: Transform::from_xyz(
                per_frame as f32 * 100.0 + rand::thread_rng().gen_range(-40.0..40.0),
                300.0,
                0.0,
            ),
            ..Default::default()
        });
    }
}

fn move_text(
    mut commands: Commands,
    mut texts: Query<(Entity, &mut Transform), With<Text>>,
    time: Res<Time>,
) {
    for (entity, mut position) in texts.iter_mut() {
        position.translation -= Vec3::new(0.0, 100.0 * time.delta_seconds(), 0.0);
        if position.translation.y < -300.0 {
            commands.entity(entity).despawn();
        }
    }
}
