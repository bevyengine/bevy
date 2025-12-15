//! This example demonstrates how to hot patch systems.
//!
//! It needs to be run with the dioxus CLI:
//! ```sh
//! dx serve --hot-patch --example hotpatching_systems --features hotpatching
//! ```
//!
//! All systems are automatically hot patchable.
//!
//! You can change the text in the `update_text` system, or the color in the
//! `on_click` system, and those changes will be hotpatched into the running
//! application.
//!
//! It's also possible to make any function hot patchable by wrapping it with
//! `bevy::dev_tools::hotpatch::call`.

use std::time::Duration;

use bevy::{color::palettes, prelude::*};

fn main() {
    let (sender, receiver) = crossbeam_channel::unbounded::<()>();

    // This function is here to demonstrate how to make something hot patchable outside of a system
    // It uses a thread for simplicity but could be an async task, an asset loader, ...
    start_thread(receiver);

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(TaskSender(sender))
        .add_systems(Startup, setup)
        .add_systems(Update, update_text)
        .run();
}

fn update_text(mut text: Single<&mut Text>) {
    // Anything in the body of a system can be changed.
    // Changes to this string should be immediately visible in the example.
    text.0 = "before".to_string();
}

fn on_click(
    _click: On<Pointer<Click>>,
    mut color: Single<&mut TextColor>,
    task_sender: Res<TaskSender>,
) {
    // Observers are also hot patchable.
    // If you change this color and click on the text in the example, it will have the new color.
    color.0 = palettes::tailwind::RED_600.into();

    let _ = task_sender.0.send(());
}

#[derive(Resource)]
struct TaskSender(crossbeam_channel::Sender<()>);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            children![(
                Text::default(),
                TextFont {
                    font_size: 100.0,
                    ..default()
                },
            )],
        ))
        .observe(on_click);
}

fn start_thread(receiver: crossbeam_channel::Receiver<()>) {
    std::thread::spawn(move || {
        while receiver.recv().is_ok() {
            let start = bevy::platform::time::Instant::now();

            // You can also make any part outside of a system hot patchable by wrapping it
            // In this part, only the duration is hot patchable:
            let duration = bevy::app::hotpatch::call(|| Duration::from_secs(2));

            std::thread::sleep(duration);
            info!("done after {:?}", start.elapsed());
        }
    });
}
