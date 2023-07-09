//! This example shows what happens when there is a lot of buttons on screen.
//!
//! To start the demo without text run
//! `cargo run --example many_buttons --release no-text`
//!
//! //! To start the demo without borders run
//! `cargo run --example many_buttons --release no-borders`
//!
//| To do a full layout update each frame run
//! `cargo run --example many_buttons --release recompute-layout`
//!
//! To recompute all text each frame run
//! `cargo run --example many_buttons --release recompute-text`

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

// For a total of 110 * 110 = 12100 buttons with text
const ROW_COLUMN_COUNT: usize = 110;
const FONT_SIZE: f32 = 7.0;

/// This example shows what happens when there is a lot of buttons on screen.
fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin,
        LogDiagnosticsPlugin::default(),
    ))
    .add_systems(Startup, setup)
    .add_systems(Update, button_system);

    if std::env::args().any(|arg| arg == "recompute-layout") {
        app.add_systems(Update, |mut ui_scale: ResMut<UiScale>| {
            ui_scale.set_changed();
        });
    }

    if std::env::args().any(|arg| arg == "recompute-text") {
        app.add_systems(Update, |mut text_query: Query<&mut Text>| {
            text_query.for_each_mut(|mut text| text.set_changed());
        });
    }

    app.run();
}

#[derive(Component)]
struct IdleColor(BackgroundColor);

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &IdleColor),
        Changed<Interaction>,
    >,
) {
    for (interaction, mut material, IdleColor(idle_color)) in interaction_query.iter_mut() {
        if matches!(interaction, Interaction::Hovered) {
            *material = Color::ORANGE_RED.into();
        } else {
            *material = *idle_color;
        }
    }
}

fn setup(mut commands: Commands) {
    warn!(include_str!("warning_string.txt"));

    let count = ROW_COLUMN_COUNT;
    let count_f = count as f32;
    let as_rainbow = |i: usize| Color::hsl((i as f32 / count_f) * 360.0, 0.9, 0.8);
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            let spawn_text = std::env::args().all(|arg| arg != "no-text");
            let border = if std::env::args().all(|arg| arg != "no-borders") {
                UiRect::all(Val::Percent(10. / count_f))
            } else {
                UiRect::DEFAULT
            };
            for i in 0..count {
                for j in 0..count {
                    let color = as_rainbow(j % i.max(1)).into();
                    let border_color = as_rainbow(i % j.max(1)).into();
                    spawn_button(
                        commands,
                        color,
                        count_f,
                        i,
                        j,
                        spawn_text,
                        border,
                        border_color,
                    );
                }
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_button(
    commands: &mut ChildBuilder,
    background_color: BackgroundColor,
    total: f32,
    i: usize,
    j: usize,
    spawn_text: bool,
    border: UiRect,
    border_color: BorderColor,
) {
    let width = 90.0 / total;
    let mut builder = commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Percent(width),
                height: Val::Percent(width),
                bottom: Val::Percent(100.0 / total * i as f32),
                left: Val::Percent(100.0 / total * j as f32),
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                border,
                ..default()
            },
            background_color,
            border_color,
            ..default()
        },
        IdleColor(background_color),
    ));

    if spawn_text {
        builder.with_children(|commands| {
            commands.spawn(TextBundle::from_section(
                format!("{i}, {j}"),
                TextStyle {
                    font_size: FONT_SIZE,
                    color: Color::rgb(0.2, 0.2, 0.2),
                    ..default()
                },
            ));
        });
    }
}
