//! This example shows what happens when there is a lot of buttons on screen.
//!
//! To start the demo without text run
//! `cargo run --example many_buttons --release no-text`
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

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            present_mode: PresentMode::Immediate,
            ..default()
        }),
        ..default()
    }))
    .add_plugin(FrameTimeDiagnosticsPlugin::default())
    .add_plugin(LogDiagnosticsPlugin::default())
    .init_resource::<UiFont>()
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

#[derive(Resource)]
struct UiFont(Handle<Font>);

impl FromWorld for UiFont {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        UiFont(asset_server.load("fonts/FiraSans-Bold.ttf"))
    }
}

fn setup(mut commands: Commands, font: Res<UiFont>) {
    let count = ROW_COLUMN_COUNT;
    let count_f = count as f32;
    let as_rainbow = |i: usize| Color::hsl((i as f32 / count_f) * 360.0, 0.9, 0.8);
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            let spawn_text = std::env::args().all(|arg| arg != "no-text");
            for i in 0..count {
                for j in 0..count {
                    let color = as_rainbow(j % i.max(1)).into();
                    spawn_button(
                        commands,
                        font.0.clone_weak(),
                        color,
                        count_f,
                        i,
                        j,
                        spawn_text,
                    );
                }
            }
        });
}

fn spawn_button(
    commands: &mut ChildBuilder,
    font: Handle<Font>,
    color: BackgroundColor,
    total: f32,
    i: usize,
    j: usize,
    spawn_text: bool,
) {
    let width = 90.0 / total;
    let mut builder = commands.spawn((
        ButtonBundle {
            style: Style {
                size: Size::new(Val::Percent(width), Val::Percent(width)),
                bottom: Val::Percent(100.0 / total * i as f32),
                left: Val::Percent(100.0 / total * j as f32),
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            background_color: color,
            ..default()
        },
        IdleColor(color),
    ));

    if spawn_text {
        builder.with_children(|commands| {
            commands.spawn(TextBundle::from_section(
                format!("{i}, {j}"),
                TextStyle {
                    font,
                    font_size: FONT_SIZE,
                    color: Color::rgb(0.2, 0.2, 0.2),
                },
            ));
        });
    }
}
