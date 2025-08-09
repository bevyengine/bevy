//! Shows various text layout options.

use std::{collections::VecDeque, time::Duration};

use bevy::{
    color::palettes::css::*,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    ui::widget::TextUiWriter,
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
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, infotext_system)
        .add_systems(Update, change_text_system)
        .run();
}

#[derive(Component)]
struct TextChanges;

fn infotext_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let background_color = MAROON.into();
    commands.spawn(Camera2d);

    let root_uinode = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .id();

    let left_column = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Start,
            flex_grow: 1.,
            margin: UiRect::axes(Val::Px(15.), Val::Px(5.)),
            ..default()
        }).with_children(|builder| {
        builder.spawn((
            Text::new("This is\ntext with\nline breaks\nin the top left."),
            TextFont {
                font: font.clone(),
                font_size: 25.0,
                ..default()
            },
            BackgroundColor(background_color)
        ));
        builder.spawn((
            Text::new(
                "This text is right-justified. The `Justify` component controls the horizontal alignment of the lines of multi-line text relative to each other, and does not affect the text node's position in the UI layout.",
            ),
            TextFont {
                font: font.clone(),
                font_size: 25.0,
                ..default()
            },
            TextColor(YELLOW.into()),
            TextLayout::new_with_justify(Justify::Right),
            Node {
                max_width: Val::Px(300.),
                ..default()
            },
            BackgroundColor(background_color)
        ));
        builder.spawn((
            Text::new(
                "This\ntext has\nline breaks and also a set width in the bottom left."),
            TextFont {
                font: font.clone(),
                font_size: 25.0,
                ..default()
            },
            Node {
                max_width: Val::Px(300.),
                ..default()
            },
            BackgroundColor(background_color)
        )
        );
    }).id();

    let right_column = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::End,
            flex_grow: 1.,
            margin: UiRect::axes(Val::Px(15.), Val::Px(5.)),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn((
                Text::new("This text is very long, has a limited width, is center-justified, is positioned in the top right and is also colored pink."),
                TextFont {
                    font: font.clone(),
                    font_size: 33.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.2, 0.7)),
                TextLayout::new_with_justify(Justify::Center),
                Node {
                    max_width: Val::Px(400.),
                    ..default()
                },
                BackgroundColor(background_color),
            ));

            builder.spawn((
                Text::new("This text is left-justified and is vertically positioned to distribute the empty space equally above and below it."),
                TextFont {
                    font: font.clone(),
                    font_size: 29.0,
                    ..default()
                },
                TextColor(YELLOW.into()),
                TextLayout::new_with_justify(Justify::Left),
                Node {
                    max_width: Val::Px(300.),
                    ..default()
                },
                BackgroundColor(background_color),
            ));

            builder.spawn((
                Text::new("This text is fully justified and is positioned in the same way."),
                TextFont {
                    font: font.clone(),
                    font_size: 29.0,
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Justified),
                TextColor(GREEN_YELLOW.into()),
                Node {
                    max_width: Val::Px(300.),
                    ..default()
                },
                BackgroundColor(background_color),
            ));

            builder
                .spawn((
                    Text::default(),
                    TextFont {
                        font: font.clone(),
                        font_size: 21.0,
                        ..default()
                    },
                    TextChanges,
                    BackgroundColor(background_color),
                ))
                .with_children(|p| {
                    p.spawn((
                        TextSpan::new("\nThis text changes in the bottom right"),
                        TextFont {
                            font: font.clone(),
                            font_size: 21.0,
                            ..default()
                        },
                    ));
                    p.spawn((
                        TextSpan::new(" this text has zero font size"),
                        TextFont {
                            font: font.clone(),
                            font_size: 0.0,
                            ..default()
                        },
                        TextColor(BLUE.into()),
                    ));
                    p.spawn((
                        TextSpan::new("\nThis text changes in the bottom right - "),
                        TextFont {
                            font: font.clone(),
                            font_size: 21.0,
                            ..default()
                        },
                        TextColor(RED.into()),
                    ));
                    p.spawn((
                        TextSpan::default(),
                        TextFont {
                            font: font.clone(),
                            font_size: 21.0,
                            ..default()
                        },
                        TextColor(ORANGE_RED.into()),
                    ));
                    p.spawn((
                        TextSpan::new(" fps, "),
                        TextFont {
                            font: font.clone(),
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(YELLOW.into()),
                    ));
                    p.spawn((
                        TextSpan::default(),
                        TextFont {
                            font: font.clone(),
                            font_size: 21.0,
                            ..default()
                        },
                        TextColor(LIME.into()),
                    ));
                    p.spawn((
                        TextSpan::new(" ms/frame"),
                        TextFont {
                            font: font.clone(),
                            font_size: 42.0,
                            ..default()
                        },
                        TextColor(BLUE.into()),
                    ));
                    p.spawn((
                        TextSpan::new(" this text has negative font size"),
                        TextFont {
                            font: font.clone(),
                            font_size: -42.0,
                            ..default()
                        },
                        TextColor(BLUE.into()),
                    ));
                });
        })
        .id();
    commands
        .entity(root_uinode)
        .add_children(&[left_column, right_column]);
}

fn change_text_system(
    mut fps_history: Local<VecDeque<f64>>,
    mut time_history: Local<VecDeque<Duration>>,
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    query: Query<Entity, With<TextChanges>>,
    mut writer: TextUiWriter,
) {
    time_history.push_front(time.elapsed());
    time_history.truncate(120);
    let avg_fps = (time_history.len() as f64)
        / (time_history.front().copied().unwrap_or_default()
            - time_history.back().copied().unwrap_or_default())
        .as_secs_f64()
        .max(0.0001);
    fps_history.push_front(avg_fps);
    fps_history.truncate(120);
    let fps_variance = std_deviation(fps_history.make_contiguous()).unwrap_or_default();

    for entity in &query {
        let mut fps = 0.0;
        if let Some(fps_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
            && let Some(fps_smoothed) = fps_diagnostic.smoothed()
        {
            fps = fps_smoothed;
        }

        let mut frame_time = time.delta_secs_f64();
        if let Some(frame_time_diagnostic) =
            diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
            && let Some(frame_time_smoothed) = frame_time_diagnostic.smoothed()
        {
            frame_time = frame_time_smoothed;
        }

        *writer.text(entity, 0) =
            format!("{avg_fps:.1} avg fps, {fps_variance:.1} frametime variance",);

        *writer.text(entity, 1) = format!(
            "\nThis text changes in the bottom right - {fps:.1} fps, {frame_time:.3} ms/frame",
        );

        *writer.text(entity, 4) = format!("{fps:.1}");

        *writer.text(entity, 6) = format!("{frame_time:.3}");
    }
}

fn mean(data: &[f64]) -> Option<f64> {
    let sum = data.iter().sum::<f64>();
    let count = data.len();

    match count {
        positive if positive > 0 => Some(sum / count as f64),
        _ => None,
    }
}

fn std_deviation(data: &[f64]) -> Option<f64> {
    match (mean(data), data.len()) {
        (Some(data_mean), count) if count > 0 => {
            let variance = data
                .iter()
                .map(|value| {
                    let diff = data_mean - *value;

                    diff * diff
                })
                .sum::<f64>()
                / count as f64;

            Some(variance.sqrt())
        }
        _ => None,
    }
}
