//! This example shows what happens when there is a lot of buttons on screen.
//! 
//! To start the demo without text run
//! `cargo run --example many_buttons --release no-text`

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

// For a total of 110 * 110 = 12100 buttons with text
const ROW_COLUMN_COUNT: usize = 110;
<<<<<<< HEAD
=======

// For a total of 220 * 220 = 48400 buttons without text
#[cfg(not(feature = "bevy_text"))]
const ROW_COLUMN_COUNT: usize = 220;

#[cfg(feature = "bevy_text")]
>>>>>>> 85773ce157abe4f35a977aff70a09ec59baec2a7
const FONT_SIZE: f32 = 7.0;

/// This example shows what happens when there is a lot of buttons on screen.
fn main() {
<<<<<<< HEAD
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
        .init_resource::<UiFont>()
        .add_systems((setup.on_startup(), button_system))
        .run();
=======
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
    .add_systems((setup.on_startup(), button_system));

    #[cfg(feature = "bevy_text")]
    app.init_resource::<UiFont>();

    app.run();
>>>>>>> 85773ce157abe4f35a977aff70a09ec59baec2a7
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

<<<<<<< HEAD
fn setup(mut commands: Commands, font: Res<UiFont>) {
=======
fn setup(mut commands: Commands, #[cfg(feature = "bevy_text")] font: Res<UiFont>) {
>>>>>>> 85773ce157abe4f35a977aff70a09ec59baec2a7
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
            for i in 0..count {
                for j in 0..count {
                    let color = as_rainbow(j % i.max(1)).into();
<<<<<<< HEAD
                    spawn_button(commands, font.0.clone_weak(), color, count_f, i, j);
=======
                    spawn_button(
                        commands,
                        #[cfg(feature = "bevy_text")]
                        font.0.clone_weak(),
                        color,
                        count_f,
                        i,
                        j,
                    );
>>>>>>> 85773ce157abe4f35a977aff70a09ec59baec2a7
                }
            }
        });
}
fn spawn_button(
    commands: &mut ChildBuilder,
<<<<<<< HEAD
    font: Handle<Font>,
=======
    #[cfg(feature = "bevy_text")] font: Handle<Font>,
>>>>>>> 85773ce157abe4f35a977aff70a09ec59baec2a7
    color: BackgroundColor,
    total: f32,
    i: usize,
    j: usize,
) {
    let width = 90.0 / total;
    let mut builder = commands
        .spawn((
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

    if std::env::args().nth(1).as_deref() != Some("no-text") {
        builder
        .with_children(|commands| {
            commands.spawn(TextBundle::from_section(
                format!("{i}, {j}"),
                TextStyle {
                    font,
                    font_size: FONT_SIZE,
                    color: Color::rgb(0.2, 0.2, 0.2),
                },
            ));
        });
<<<<<<< HEAD
    }
=======

    #[cfg(not(feature = "bevy_text"))]
    commands.spawn((
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
>>>>>>> 85773ce157abe4f35a977aff70a09ec59baec2a7
}
