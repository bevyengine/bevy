//! Illustrates how to use bevy off the main thread

use std::sync::mpsc;

use bevy::prelude::*;
use bevy_internal::winit::WinitSettings;

fn main() {
    println!("Press Enter to spawn the bevy thread, or type 'exit' to exit");

    let (tx, rx) = mpsc::channel();
    let (end_tx, end_rx) = mpsc::channel();

    {
        let Some(line) = std::io::stdin().lines().next() else {
            eprintln!("No Lines");
            return;
        };

        let typed: String = line.unwrap_or_default();
        if typed == "exit" {
            return;
        } else {
            run_bevy(rx, end_tx);
            println!("Spawned a bevy window!");
            println!("Now - type things in the terminal, and they will show up in the bevy UI, or type 'exit' to exit");
        }
    }

    for line in std::io::stdin().lines() {
        let typed: String = line.unwrap_or_default();
        if typed == "exit" {
            return;
        } else {
            if end_rx.try_recv().is_ok() {
                return;
            }
            tx.send(typed);
        }
    }
}

fn run_bevy(rx: mpsc::Receiver<String>, end_tx: mpsc::Sender<()>) {
    std::thread::spawn(move || {
        App::new()
            .insert_resource(WinitSettings {
                return_from_run: true,
                ..default()
            })
            .insert_resource(ClearColor(Color::WHITE))
            .insert_non_send_resource(TextReceiver(rx))
            .add_plugins(DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Close the window to return to the main function".into(),
                    ..default()
                }),
                ..default()
            }))
            .add_systems(Startup, setup)
            .add_systems(Update, system)
            .run();
        println!("Exited the window - press anything to exit the app");
        end_tx.send(());
    });
}

struct TextReceiver(mpsc::Receiver<String>);

#[derive(Component)]
struct Root;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        Root,
        NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                top: Val::Px(0.),
                left: Val::Px(0.),
                bottom: Val::Px(0.),
                right: Val::Px(0.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            ..Default::default()
        },
    ));
}

fn system(
    rx: NonSend<TextReceiver>,
    mut commands: Commands,
    root: Query<Entity, With<Root>>,
    asset_server: Res<AssetServer>,
) {
    if let Ok(msg) = rx.0.try_recv() {
        if let Ok(root) = root.get_single() {
            commands.entity(root).with_children(|p| {
                p.spawn(TextBundle::from_section(
                    msg,
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 30.0,
                        color: Color::BLACK,
                    },
                ));
            });
        }
    }
}
