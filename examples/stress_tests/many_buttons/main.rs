mod args;
mod create_button_row;
mod idle_color;

/// General UI benchmark that stress tests layouting, text, interaction and rendering
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::{
        default, warn, AlignItems, App, AssetServer, BackgroundColor, BuildChildren,
        Camera2dBundle, Changed, Color, Commands, DetectChangesMut, Display, FlexDirection,
        Interaction, JustifyContent, NodeBundle, PluginGroup, Query, RepeatedGridTrack, Res,
        Startup, Style, Text, Update, Window,
    },
    window::{PresentMode, WindowPlugin, WindowResolution},
    DefaultPlugins,
};

use crate::{args::Args, create_button_row::create_button_row, idle_color::IdleColor};

/// This example shows what happens when there is a lot of buttons on screen.
fn main() {
    let args: Args = argh::from_env();
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920.0, 1080.0).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin,
        LogDiagnosticsPlugin::default(),
    ))
    .add_systems(Update, button_system);

    if args.grid {
        app.add_systems(Startup, setup_grid);
    } else {
        app.add_systems(Startup, setup_flex);
    }

    if args.relayout {
        app.add_systems(Update, |mut style_query: Query<&mut Style>| {
            style_query.for_each_mut(|mut style| style.set_changed());
        });
    }

    if args.recompute_text {
        app.add_systems(Update, |mut text_query: Query<&mut Text>| {
            text_query.for_each_mut(|mut text| text.set_changed());
        });
    }

    app.insert_resource(args).run();
}

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &IdleColor),
        Changed<Interaction>,
    >,
) {
    for (interaction, mut button_color, IdleColor(idle_color)) in interaction_query.iter_mut() {
        *button_color = match interaction {
            Interaction::Hovered => Color::ORANGE_RED.into(),
            _ => *idle_color,
        };
    }
}

fn setup_flex(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    warn!(include_str!("../warning_string.txt"));

    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            for column in 0..args.buttons {
                commands
                    .spawn(NodeBundle::default())
                    .with_children(|commands| {
                        create_button_row(&args, &asset_server, commands, column);
                    });
            }
        });
}

fn setup_grid(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    warn!(include_str!("../warning_string.txt"));

    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(args.buttons as u16, 1.0),
                grid_template_rows: RepeatedGridTrack::flex(args.buttons as u16, 1.0),
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            for column in 0..args.buttons {
                create_button_row(&args, &asset_server, commands, column);
            }
        });
}
