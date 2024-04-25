//! General UI benchmark that stress tests layouting, text, interaction and rendering

use argh::FromArgs;
use bevy::{
    color::palettes::css::ORANGE_RED,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};

const FONT_SIZE: f32 = 7.0;

#[derive(FromArgs, Resource)]
/// `many_buttons` general UI benchmark that stress tests layouting, text, interaction and rendering
struct Args {
    /// whether to add text to each button
    #[argh(switch)]
    no_text: bool,

    /// whether to add borders to each button
    #[argh(switch)]
    no_borders: bool,

    /// whether to perform a full relayout each frame
    #[argh(switch)]
    relayout: bool,

    /// whether to recompute all text each frame
    #[argh(switch)]
    recompute_text: bool,

    /// how many buttons per row and column of the grid.
    #[argh(option, default = "110")]
    buttons: usize,

    /// give every nth button an image
    #[argh(option, default = "4")]
    image_freq: usize,

    /// use the grid layout model
    #[argh(switch)]
    grid: bool,
}

/// This example shows what happens when there is a lot of buttons on screen.
fn main() {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

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
    .insert_resource(WinitSettings {
        focused_mode: UpdateMode::Continuous,
        unfocused_mode: UpdateMode::Continuous,
    })
    .add_systems(Update, button_system);

    if args.grid {
        app.add_systems(Startup, setup_grid);
    } else {
        app.add_systems(Startup, setup_flex);
    }

    if args.relayout {
        app.add_systems(Update, |mut style_query: Query<&mut Style>| {
            style_query
                .iter_mut()
                .for_each(|mut style| style.set_changed());
        });
    }

    if args.recompute_text {
        app.add_systems(Update, |mut text_query: Query<&mut Text>| {
            text_query
                .iter_mut()
                .for_each(|mut text| text.set_changed());
        });
    }

    app.insert_resource(args).run();
}

#[derive(Component)]
struct IdleColor(Color);

fn button_system(
    mut interaction_query: Query<(&Interaction, &mut UiImage, &IdleColor), Changed<Interaction>>,
) {
    for (interaction, mut image, &IdleColor(idle_color)) in interaction_query.iter_mut() {
        image.color = match interaction {
            Interaction::Hovered => ORANGE_RED.into(),
            _ => idle_color,
        };
    }
}

fn setup_flex(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    warn!(include_str!("warning_string.txt"));
    let image = if 0 < args.image_freq {
        Some(asset_server.load("branding/icon.png"))
    } else {
        None
    };

    let buttons_f = args.buttons as f32;
    let border = if args.no_borders {
        UiRect::ZERO
    } else {
        UiRect::all(Val::VMin(0.05 * 90. / buttons_f))
    };

    let as_rainbow = |i: usize| Color::hsl((i as f32 / buttons_f) * 360.0, 0.9, 0.8);
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            for column in 0..args.buttons {
                commands
                    .spawn(NodeBundle::default())
                    .with_children(|commands| {
                        for row in 0..args.buttons {
                            let color = as_rainbow(row % column.max(1));
                            let border_color = Color::WHITE.with_alpha(0.5).into();
                            spawn_button(
                                commands,
                                color,
                                buttons_f,
                                column,
                                row,
                                !args.no_text,
                                border,
                                border_color,
                                image
                                    .as_ref()
                                    .filter(|_| (column + row) % args.image_freq == 0)
                                    .cloned(),
                            );
                        }
                    });
            }
        });
}

fn setup_grid(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    warn!(include_str!("warning_string.txt"));
    let image = if 0 < args.image_freq {
        Some(asset_server.load("branding/icon.png"))
    } else {
        None
    };

    let buttons_f = args.buttons as f32;
    let border = if args.no_borders {
        UiRect::ZERO
    } else {
        UiRect::all(Val::VMin(0.05 * 90. / buttons_f))
    };

    let as_rainbow = |i: usize| Color::hsl((i as f32 / buttons_f) * 360.0, 0.9, 0.8);
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                display: Display::Grid,
                width: Val::Percent(100.),
                height: Val::Percent(100.0),
                grid_template_columns: RepeatedGridTrack::flex(args.buttons as u16, 1.0),
                grid_template_rows: RepeatedGridTrack::flex(args.buttons as u16, 1.0),
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            for column in 0..args.buttons {
                for row in 0..args.buttons {
                    let color = as_rainbow(row % column.max(1));
                    let border_color = Color::WHITE.with_alpha(0.5).into();
                    spawn_button(
                        commands,
                        color,
                        buttons_f,
                        column,
                        row,
                        !args.no_text,
                        border,
                        border_color,
                        image
                            .as_ref()
                            .filter(|_| (column + row) % args.image_freq == 0)
                            .cloned(),
                    );
                }
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_button(
    commands: &mut ChildBuilder,
    background_color: Color,
    buttons: f32,
    column: usize,
    row: usize,
    spawn_text: bool,
    border: UiRect,
    border_color: BorderColor,
    image: Option<Handle<Image>>,
) {
    let width = Val::Vw(90.0 / buttons);
    let height = Val::Vh(90.0 / buttons);
    let margin = UiRect::axes(width * 0.05, height * 0.05);
    let mut builder = commands.spawn((
        ButtonBundle {
            style: Style {
                width,
                height,
                margin,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border,
                ..default()
            },
            image: UiImage::default().with_color(background_color),
            border_color,
            ..default()
        },
        IdleColor(background_color),
    ));

    if let Some(image) = image {
        builder.insert(UiImage::new(image));
    }

    if spawn_text {
        builder.with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                format!("{column}, {row}"),
                TextStyle {
                    font_size: FONT_SIZE,
                    color: Color::srgb(0.2, 0.2, 0.2),
                    ..default()
                },
            ));
        });
    }
}
