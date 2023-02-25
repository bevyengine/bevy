//! Simple text rendering benchmark.
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin}, text::BreakLineOn,
};

/// This example shows what happens when there is a lot of buttons on screen.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Immediate,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_basis: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            commands.spawn(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        value: std::iter::repeat("0123456789").take(10_000).collect::<String>(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 20.0,
                            color: Color::WHITE,
                        },
                    }],
                    alignment: TextAlignment::Left,
                    linebreak_behaviour: BreakLineOn::AnyCharacter,
                    ..Default::default()
                },
                style: Style {
                    flex_basis: Val::Px(1000.),
                    ..Default::default()
                },
                ..Default::default()
            });
        });
}

