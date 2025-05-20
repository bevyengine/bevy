//! This example demonstrates how to hotpatch systems.
//!
//! It needs to be run with the dioxus CLI:
//! ```sh
//! dx serve --hot-patch --example hotpatching_systems --features hotpatching
//! ```
//!
//! You can change the text in the `update_text` system, or the color in the
//! `on_click` system, and those changes will be hotpatched into the running
//! application.

use std::time::Duration;

use bevy::{color::palettes, prelude::*};

fn main() {
    let (sender, receiver) = crossbeam_channel::unbounded::<()>();

    std::thread::spawn(move || {
        while receiver.recv().is_ok() {
            let start = bevy::platform::time::Instant::now();
            // You can also make any part outside of a system hot patchable by wrapping it
            // In this part, only the duration is hot patchable:
            let duration = bevy::dev_tools::hotpatch::call(|| Duration::from_secs(2));
            std::thread::sleep(duration);
            info!("done after {:?}", start.elapsed());
        }
    });

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(TaskSender(sender))
        .add_systems(Startup, setup)
        .add_systems(Update, update_text)
        .run();
}

fn update_text(mut text: Single<&mut Text>) {
    text.0 = "before".to_string();
}

fn on_click(
    _click: Trigger<Pointer<Click>>,
    mut color: Single<&mut TextColor>,
    task_sender: Res<TaskSender>,
) {
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
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
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
