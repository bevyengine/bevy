use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin}, text::BreakLineOn,
};

const GLYPH_COUNT: usize = 100_000;

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
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let mut text_sections = vec![];
    for i in 0..GLYPH_COUNT {
        let section = TextSection {
            value: (i % 10).to_string(),
            style: TextStyle {
                font: font.clone(),
                font_size: 8.,
                color: Color::WHITE,
                ..Default::default()
            }
        };
        text_sections.push(section);
    }
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::all(Val::Px(1050.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            commands.spawn(TextBundle {
                text: Text {
                    sections: text_sections,
                    alignment: TextAlignment::Left,
                    linebreak_behaviour: BreakLineOn::AnyCharacter,
                    ..Default::default()
                },
                style: Style {
                    size: Size::all(Val::Px(1050.)),
                    ..Default::default()
                },
                background_color: Color::MAROON.into(),
                ..Default::default()
            });
        });
}

