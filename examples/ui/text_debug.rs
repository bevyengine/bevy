//! Shows various text layout options.

use std::{collections::VecDeque, time::Duration};

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
        builder.spawn((
            TextNEW::new("This is\ntext with\nline breaks\nin the top left."),
                TextStyle {
                    font: font.clone(),
                    font_size: 25.0,
                    ..default()
                })
        );
        builder.spawn((
            TextNEW::new(
                "This text is right-justified. The `JustifyText` component controls the horizontal alignment of the lines of multi-line text relative to each other, and does not affect the text node's position in the UI layout.",
            ),
            TextStyle {
                font: font.clone(),
                font_size: 25.0,
                color: YELLOW.into(),
            },
            TextBlock::new_with_justify(JustifyText::Right),
            Style {
                max_width: Val::Px(300.),
                ..default()
            }
        ));
        builder.spawn((
            TextNEW::new(
                "This\ntext has\nline breaks and also a set width in the bottom left."),
            TextStyle {
                font: font.clone(),
                font_size: 25.0,
                ..default()
            },
            Style {
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

        builder.spawn((TextNEW::new(
            "This text is very long, has a limited width, is center-justified, is positioned in the top right and is also colored pink."),
            TextStyle {
                font: font.clone(),
                font_size: 33.0,
                color: Color::srgb(0.8, 0.2, 0.7),
            },
            TextBlock::new_with_justify(JustifyText::Center),
            Style {
                max_width: Val::Px(400.),
                ..default()
            })
        );

        builder.spawn((TextNEW::new(
            "This text is left-justified and is vertically positioned to distribute the empty space equally above and below it."),
            TextStyle {
                font: font.clone(),
                font_size: 29.0,
                color: YELLOW.into(),
            },
            TextBlock::new_with_justify(JustifyText::Left),
            Style {
                max_width: Val::Px(300.),
                ..default()
            })
        );

        builder.spawn((TextNEW::new(
                "This text is fully justified and is positioned in the same way."),
                TextStyle {
                    font: font.clone(),
                    font_size: 29.0,
                    color: GREEN_YELLOW.into(),
                },
            TextBlock::new_with_justify(JustifyText::Justified),
            Style {
                max_width: Val::Px(300.),
                ..default()
            })
        );

        builder.spawn((
            TextNEW::new(""),
            TextStyle {
                font: font.clone(),
                font_size: 21.0,
                ..default()
            },
            TextChanges
        ))
            .with_child((TextSpan::new(
                "\nThis text changes in the bottom right"),
                TextStyle {
                    font: font.clone(),
                    font_size: 21.0,
                    ..default()
                },
            ))
            .with_child((TextSpan::new(
                " this text has zero fontsize"),
                TextStyle {
                    font: font.clone(),
                    font_size: 0.0,
                    color: BLUE.into(),
                },
            ))
            .with_child((TextSpan::new(
                "\nThis text changes in the bottom right - "),
                TextStyle {
                    font: font.clone(),
                    font_size: 21.0,
                    color: RED.into(),
                },
            ))
            .with_child((TextSpan::default(),
            TextStyle {
                font: font.clone(),
                font_size: 21.0,
                color: ORANGE_RED.into(),
            }))
            .with_child((TextSpan::new(
                " fps, "),
                TextStyle {
                    font: font.clone(),
                    font_size: 10.0,
                    color: YELLOW.into(),
                },
            ))
            .with_child((TextSpan::default(),
            TextStyle {
                font: font.clone(),
                font_size: 21.0,
                color: LIME.into(),
            }))
            .with_child((TextSpan::new(
                " ms/frame"),
                TextStyle {
                    font: font.clone(),
                    font_size: 42.0,
                    color: BLUE.into(),
                },
            ))
            .with_child((TextSpan::new(
                " this text has negative fontsize"),
                TextStyle {
                    font: font.clone(),
                    font_size: -42.0,
                    color: BLUE.into(),
                },
            ));
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
    mut query: Query<&mut Text, With<TextChanges>>,
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

        text.sections[0].value =
            format!("{avg_fps:.1} avg fps, {fps_variance:.1} frametime variance",);

        text.sections[1].value = format!(
            "\nThis text changes in the bottom right - {fps:.1} fps, {frame_time:.3} ms/frame",
        );

        text.sections[4].value = format!("{fps:.1}");

        text.sections[6].value = format!("{frame_time:.3}");
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
