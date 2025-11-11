//! General UI benchmark that stress tests layouting, text, interaction and rendering

use argh::FromArgs;
use bevy::{
    color::palettes::css::ORANGE_RED,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::TextColor,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};

const FONT_SIZE: f32 = 7.0;

#[derive(FromArgs, Resource)]
/// `many_buttons` general UI benchmark that stress tests layouting, text, interaction and rendering
struct Args {
    /// whether to add labels to each button
    #[argh(switch)]
    text: bool,

    /// whether to add borders to each button
    #[argh(switch)]
    no_borders: bool,

    /// whether to perform a full relayout each frame
    #[argh(switch)]
    relayout: bool,

    /// whether to recompute all text each frame (if text enabled)
    #[argh(switch)]
    recompute_text: bool,

    /// how many buttons per row and column of the grid.
    #[argh(option, default = "110")]
    buttons: usize,

    /// change the button icon every nth button, if `0` no icons are added.
    #[argh(option, default = "4")]
    image_freq: usize,

    /// use the grid layout model
    #[argh(switch)]
    grid: bool,

    /// at the start of each frame despawn any existing UI nodes and spawn a new UI tree
    #[argh(switch)]
    respawn: bool,

    /// set the root node to display none, removing all nodes from the layout.
    #[argh(switch)]
    display_none: bool,

    /// spawn the layout without a camera
    #[argh(switch)]
    no_camera: bool,

    /// a layout with a separate camera for each button
    #[argh(switch)]
    many_cameras: bool,
}

/// This example shows what happens when there is a lot of buttons on screen.
fn main() {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    warn!(include_str!("warning_string.txt"));

    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
    ))
    .insert_resource(WinitSettings::continuous())
    .add_systems(Update, (button_system, set_text_colors_changed));

    if !args.no_camera {
        app.add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        });
    }

    if args.many_cameras {
        app.add_systems(Startup, setup_many_cameras);
    } else if args.grid {
        app.add_systems(Startup, setup_grid);
    } else {
        app.add_systems(Startup, setup_flex);
    }

    if args.relayout {
        app.add_systems(Update, |mut nodes: Query<&mut Node>| {
            nodes.iter_mut().for_each(|mut node| node.set_changed());
        });
    }

    if args.recompute_text {
        app.add_systems(Update, |mut text_query: Query<&mut Text>| {
            text_query
                .iter_mut()
                .for_each(|mut text| text.set_changed());
        });
    }

    if args.respawn {
        if args.grid {
            app.add_systems(Update, (despawn_ui, setup_grid).chain());
        } else {
            app.add_systems(Update, (despawn_ui, setup_flex).chain());
        }
    }

    app.insert_resource(args).run();
}

fn set_text_colors_changed(mut colors: Query<&mut TextColor>) {
    for mut text_color in colors.iter_mut() {
        text_color.set_changed();
    }
}

#[derive(Component)]
struct IdleColor(Color);

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &IdleColor),
        Changed<Interaction>,
    >,
) {
    for (interaction, mut color, &IdleColor(idle_color)) in interaction_query.iter_mut() {
        *color = match interaction {
            Interaction::Hovered => ORANGE_RED.into(),
            _ => idle_color.into(),
        };
    }
}

fn setup_flex(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    let images = if 0 < args.image_freq {
        Some(vec![
            asset_server.load("branding/icon.png"),
            asset_server.load("textures/Game Icons/wrench.png"),
        ])
    } else {
        None
    };

    let buttons_f = args.buttons as f32;
    let border = if args.no_borders {
        UiRect::ZERO
    } else {
        UiRect::all(vmin(0.05 * 90. / buttons_f))
    };

    let as_rainbow = |i: usize| Color::hsl((i as f32 / buttons_f) * 360.0, 0.9, 0.8);
    commands
        .spawn(Node {
            display: if args.display_none {
                Display::None
            } else {
                Display::Flex
            },
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            width: percent(100),
            height: percent(100),
            ..default()
        })
        .with_children(|commands| {
            for column in 0..args.buttons {
                commands.spawn(Node::default()).with_children(|commands| {
                    for row in 0..args.buttons {
                        let color = as_rainbow(row % column.max(1));
                        let border_color = Color::WHITE.with_alpha(0.5).into();
                        spawn_button(
                            commands,
                            color,
                            buttons_f,
                            column,
                            row,
                            args.text,
                            border,
                            border_color,
                            images.as_ref().map(|images| {
                                images[((column + row) / args.image_freq) % images.len()].clone()
                            }),
                        );
                    }
                });
            }
        });
}

fn setup_grid(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    let images = if 0 < args.image_freq {
        Some(vec![
            asset_server.load("branding/icon.png"),
            asset_server.load("textures/Game Icons/wrench.png"),
        ])
    } else {
        None
    };

    let buttons_f = args.buttons as f32;
    let border = if args.no_borders {
        UiRect::ZERO
    } else {
        UiRect::all(vmin(0.05 * 90. / buttons_f))
    };

    let as_rainbow = |i: usize| Color::hsl((i as f32 / buttons_f) * 360.0, 0.9, 0.8);
    commands
        .spawn(Node {
            display: if args.display_none {
                Display::None
            } else {
                Display::Grid
            },
            width: percent(100),
            height: percent(100),
            grid_template_columns: RepeatedGridTrack::flex(args.buttons as u16, 1.0),
            grid_template_rows: RepeatedGridTrack::flex(args.buttons as u16, 1.0),
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
                        args.text,
                        border,
                        border_color,
                        images.as_ref().map(|images| {
                            images[((column + row) / args.image_freq) % images.len()].clone()
                        }),
                    );
                }
            }
        });
}

fn spawn_button(
    commands: &mut ChildSpawnerCommands,
    background_color: Color,
    buttons: f32,
    column: usize,
    row: usize,
    spawn_text: bool,
    border: UiRect,
    border_color: BorderColor,
    image: Option<Handle<Image>>,
) {
    let width = vw(90.0 / buttons);
    let height = vh(90.0 / buttons);
    let margin = UiRect::axes(width * 0.05, height * 0.05);
    let mut builder = commands.spawn((
        Button,
        Node {
            width,
            height,
            margin,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border,
            ..default()
        },
        BackgroundColor(background_color),
        border_color,
        IdleColor(background_color),
    ));

    if let Some(image) = image {
        builder.insert(ImageNode::new(image));
    }

    if spawn_text {
        builder.with_children(|parent| {
            // These labels are split to stress test multi-span text
            parent
                .spawn((
                    Text(format!("{column}, ")),
                    TextFont {
                        font_size: FONT_SIZE,
                        ..default()
                    },
                    TextColor(Color::srgb(0.5, 0.2, 0.2)),
                ))
                .with_child((
                    TextSpan(format!("{row}")),
                    TextFont {
                        font_size: FONT_SIZE,
                        ..default()
                    },
                    TextColor(Color::srgb(0.2, 0.2, 0.5)),
                ));
        });
    }
}

fn despawn_ui(mut commands: Commands, root_node: Single<Entity, (With<Node>, Without<ChildOf>)>) {
    commands.entity(*root_node).despawn();
}

fn setup_many_cameras(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    let images = if 0 < args.image_freq {
        Some(vec![
            asset_server.load("branding/icon.png"),
            asset_server.load("textures/Game Icons/wrench.png"),
        ])
    } else {
        None
    };

    let buttons_f = args.buttons as f32;
    let border = if args.no_borders {
        UiRect::ZERO
    } else {
        UiRect::all(vmin(0.05 * 90. / buttons_f))
    };

    let as_rainbow = |i: usize| Color::hsl((i as f32 / buttons_f) * 360.0, 0.9, 0.8);
    for column in 0..args.buttons {
        for row in 0..args.buttons {
            let color = as_rainbow(row % column.max(1));
            let border_color = Color::WHITE.with_alpha(0.5).into();
            let camera = commands
                .spawn((
                    Camera2d,
                    Camera {
                        order: (column * args.buttons + row) as isize + 1,
                        ..Default::default()
                    },
                ))
                .id();
            commands
                .spawn((
                    Node {
                        display: if args.display_none {
                            Display::None
                        } else {
                            Display::Flex
                        },
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    UiTargetCamera(camera),
                ))
                .with_children(|commands| {
                    commands
                        .spawn(Node {
                            position_type: PositionType::Absolute,
                            top: vh(column as f32 * 100. / buttons_f),
                            left: vw(row as f32 * 100. / buttons_f),
                            ..Default::default()
                        })
                        .with_children(|commands| {
                            spawn_button(
                                commands,
                                color,
                                buttons_f,
                                column,
                                row,
                                args.text,
                                border,
                                border_color,
                                images.as_ref().map(|images| {
                                    images[((column + row) / args.image_freq) % images.len()]
                                        .clone()
                                }),
                            );
                        });
                });
        }
    }
}
