//! Simple text rendering benchmark.
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::BreakLineOn,
    window::{PresentMode, WindowPlugin},
};

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
                        value: "0123456789".repeat(10_000),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 4.,
                            color: Color::WHITE,
                        },
                    }],
                    alignment: TextAlignment::Left,
                    linebreak_behaviour: BreakLineOn::AnyCharacter,
                },
                style: Style {
                    size: Size::width(Val::Px(1000.)),
                    ..Default::default()
                },
                ..Default::default()
            });
        });
}
