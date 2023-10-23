//! Shows various text layout options.

use bevy::{
    color::palettes::css::*,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
        ))
        .add_systems(Startup, infotext_system)
        .add_systems(Update, change_text_system)
        .run();
}

#[derive(Component)]
struct TextChanges;

fn infotext_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2dBundle::default());
    let root_uinode = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .id();

    let left_column = commands.spawn(NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Start,
            flex_grow: 1.,
            margin: UiRect::axes(Val::Px(15.), Val::Px(5.)),
            ..default()
        },
        ..default()
    }).with_children(|builder| {
        builder.spawn(
            TextBundle::from_section(
                "This is\ntext with\nline breaks\nin the top left.",
                TextStyle {
                    font: font.clone(),
                    font_size: 30.0,
                    ..default()
                },
            )
        );
        builder.spawn(TextBundle::from_section(
                "This text is right-justified. The `JustifyText` component controls the horizontal alignment of the lines of multi-line text relative to each other, and does not affect the text node's position in the UI layout.",                TextStyle {
                    font: font.clone(),
                    font_size: 30.0,
                    color: YELLOW.into(),
                },
            )
            .with_text_justify(JustifyText::Right)
            .with_style(Style {
                max_width: Val::Px(300.),
                ..default()
            })
        );
        builder.spawn(
            TextBundle::from_section(
                "This\ntext has\nline breaks and also a set width in the bottom left.",
                TextStyle {
                    font: font.clone(),
                    font_size: 30.0,
                    ..default()
                },
            )
            .with_style(Style {
                max_width: Val::Px(300.),
                ..default()
            })
        );
    }).id();

    let right_column = commands.spawn(NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::End,
            flex_grow: 1.,
            margin: UiRect::axes(Val::Px(15.), Val::Px(5.)),
            ..default()
        },
        ..default()
    }).with_children(|builder| {

        builder.spawn(TextBundle::from_section(
                "This text is very long, has a limited width, is center-justified, is positioned in the top right and is also colored pink.",
                TextStyle {
                    font: font.clone(),
                    font_size: 40.0,
                    color: Color::srgb(0.8, 0.2, 0.7),
                },
            )
            .with_text_justify(JustifyText::Center)
            .with_style(Style {
                max_width: Val::Px(400.),
                ..default()
            })
        );

        builder.spawn(
            TextBundle::from_section(
                "This text is left-justified and is vertically positioned to distribute the empty space equally above and below it.",
                TextStyle {
                    font: font.clone(),
                    font_size: 35.0,
                    color: YELLOW.into(),
                },
            )
            .with_text_justify(JustifyText::Left)
            .with_style(Style {
                max_width: Val::Px(300.),
                ..default()
            }),
        );

        builder.spawn((
            TextBundle::from_sections([
                TextSection::new(
                    "This text changes in the bottom right",
                    TextStyle {
                        font: font.clone(),
                        font_size: 25.0,
                        ..default()
                    },
                ),
                TextSection::new(
                    "\nThis text changes in the bottom right - ",
                    TextStyle {
                        font: font.clone(),
                        font_size: 25.0,
                        color: RED.into(),
                    },
                ),
                TextSection::from_style(TextStyle {
                    font: font.clone(),
                    font_size: 25.0,
                    color: ORANGE_RED.into(),
                }),
                TextSection::new(
                    " fps, ",
                    TextStyle {
                        font: font.clone(),
                        font_size: 25.0,
                        color: YELLOW.into(),
                    },
                ),
                TextSection::from_style(TextStyle {
                    font: font.clone(),
                    font_size: 25.0,
                    color: LIME.into(),
                }),
                TextSection::new(
                    " ms/frame",
                    TextStyle {
                        font: font.clone(),
                        font_size: 25.0,
                        color: BLUE.into(),
                    },
                ),
            ]),
            TextChanges,
        ));
    })
    .id();

    commands
        .entity(root_uinode)
        .push_children(&[left_column, right_column]);
}

fn change_text_system(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<TextChanges>>,
) {
    for mut text in &mut query {
        let mut fps = 0.0;
        if let Some(fps_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(fps_smoothed) = fps_diagnostic.smoothed() {
                fps = fps_smoothed;
            }
        }

        let mut frame_time = time.delta_seconds_f64();
        if let Some(frame_time_diagnostic) =
            diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
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
