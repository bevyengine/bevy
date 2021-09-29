use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use crossbeam_channel::{unbounded, Receiver};
use futures_lite::future;
use rand::Rng;

fn main() {
    App::new()
        .add_event::<StreamEvent>()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_system)
        .add_system(read_stream_system)
        .add_system(spawn_text_system)
        .add_system(move_text_system)
        .run();
}

struct StreamReceiver(Receiver<u32>);
struct StreamTask(Task<()>);
struct StreamEvent(u32);

fn setup_system(mut commands: Commands, thread_pool: Res<AsyncComputeTaskPool>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let (tx, rx) = unbounded::<u32>();
    commands.insert_resource(StreamTask(thread_pool.spawn(async move {
        loop {
            // Everything here happens in a thread from the pool `AsyncComputeTaskPool`
            // This is where you could connect to an external data source
            tx.send(rand::thread_rng().gen_range(0..2000)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(
                rand::thread_rng().gen_range(0..200),
            ));
        }
    })));
    commands.insert_resource(StreamReceiver(rx));
}

// This system polls the tasks, and reads from the receiver and sends events to Bevy
fn read_stream_system(
    mut task: ResMut<StreamTask>,
    receiver: ResMut<StreamReceiver>,
    mut events: EventWriter<StreamEvent>,
) {
    future::block_on(future::poll_once(&mut task.0));
    for from_stream in receiver.0.try_iter() {
        events.send(StreamEvent(from_stream))
    }
}

fn spawn_text_system(
    mut commands: Commands,
    mut reader: EventReader<StreamEvent>,
    asset_server: Res<AssetServer>,
    mut loaded_font: Local<Option<Handle<Font>>>,
) {
    let font = if let Some(font) = &*loaded_font {
        font.clone()
    } else {
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");
        *loaded_font = Some(font.clone());
        font
    };
    let text_style = TextStyle {
        font,
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
            transform: Transform::from_translation(Vec3::new(
                per_frame as f32 * 100.0 + rand::thread_rng().gen_range(-40.0..40.0),
                300.0,
                0.0,
            )),
            ..Default::default()
        });
    }
}

fn move_text_system(
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
