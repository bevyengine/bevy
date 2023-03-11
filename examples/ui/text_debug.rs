//! Shows various text layout options.

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_startup_system(infotext_system)
        .add_system(change_text_system)
        .run();
}

#[derive(Component)]
struct TextChanges;

fn infotext_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2dBundle::default());
    commands.spawn(
        TextBundle::from_section(
            "This is\ntext with\nline breaks\nin the top left",
            TextStyle {
                font: font.clone(),
                font_size: 50.0,
                color: Color::WHITE,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                left: Val::Px(15.0),
                ..default()
            },
            ..default()
        }),
    );
    commands.spawn(TextBundle::from_section(
            "This text is very long, has a limited width, is centred, is positioned in the top right and is also coloured pink.",
            TextStyle {
                font: font.clone(),
                font_size: 50.0,
                color: Color::rgb(0.8, 0.2, 0.7),
            },
        )
        .with_text_alignment(TextAlignment::Center)
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                right: Val::Px(15.0),
                ..default()
            },
            max_size: Size {
                width: Val::Px(400.),
                height: Val::Undefined,
            },
            ..default()
        })
    );
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "This text changes in the bottom right",
                TextStyle {
                    font: font.clone(),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "\nThis text changes in the bottom right - ",
                TextStyle {
                    font: font.clone(),
                    font_size: 30.0,
                    color: Color::RED,
                },
            ),
            TextSection::from_style(TextStyle {
                font: font.clone(),
                font_size: 30.0,
                color: Color::ORANGE_RED,
            }),
            TextSection::new(
                " fps, ",
                TextStyle {
                    font: font.clone(),
                    font_size: 30.0,
                    color: Color::YELLOW,
                },
            ),
            TextSection::from_style(TextStyle {
                font: font.clone(),
                font_size: 30.0,
                color: Color::GREEN,
            }),
            TextSection::new(
                " ms/frame",
                TextStyle {
                    font: font.clone(),
                    font_size: 30.0,
                    color: Color::BLUE,
                },
            ),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                bottom: Val::Px(5.0),
                right: Val::Px(15.0),
                ..default()
            },
            ..default()
        }),
        TextChanges,
    ));
    commands.spawn(
        TextBundle::from_section(
            "This\ntext has\nline breaks and also a set width in the bottom left",
            TextStyle {
                font,
                font_size: 50.0,
                color: Color::WHITE,
            },
        )
        .with_style(Style {
            align_self: AlignSelf::FlexEnd,
            position_type: PositionType::Absolute,
            position: UiRect {
                bottom: Val::Px(5.0),
                left: Val::Px(15.0),
                ..default()
            },
            size: Size {
                width: Val::Px(200.0),
                ..default()
            },
            ..default()
        }),
    );
}

fn change_text_system(
    time: Res<Time>,
    diagnostics: Res<Diagnostics>,
    mut query: Query<&mut Text, With<TextChanges>>,
) {
    for mut text in &mut query {
        let mut fps = 0.0;
        if let Some(fps_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(fps_smoothed) = fps_diagnostic.smoothed() {
                fps = fps_smoothed;
            }
        }

        let mut frame_time = time.delta_seconds_f64();
        if let Some(frame_time_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME)
        {
            if let Some(frame_time_smoothed) = frame_time_diagnostic.smoothed() {
                frame_time = frame_time_smoothed;
            }
        }

        text.sections[0].value = format!(
            "This text changes in the bottom right - {fps:.1} fps, {frame_time:.3} ms/frame",
        );

        text.sections[2].value = format!("{fps:.1}");

        text.sections[4].value = format!("{frame_time:.3}");
    }
}
