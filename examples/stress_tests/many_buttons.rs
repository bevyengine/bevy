/// General UI benchmark that stress tests layouting, text, interaction and rendering
use argh::FromArgs;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
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
    recompute_layout: bool,

    /// whether to recompute all text each frame
    #[argh(switch)]
    recompute_text: bool,

    /// how many buttons per row and column of the grid.
    /// The total number of buttons displayed will be this number squared.
    #[argh(option, default = "110")]
    buttons: usize,

    /// give every nth button an image
    #[argh(option, default = "4")]
    image_freq: usize,
}

/// This example shows what happens when there is a lot of buttons on screen.
fn main() {
    let args: Args = argh::from_env();
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: (800., 800.).into(),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin,
        LogDiagnosticsPlugin::default(),
    ))
    .add_systems(Startup, setup)
    .add_systems(Update, button_system);

    if args.recompute_layout {
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

#[derive(Component)]
struct IdleColor(BackgroundColor);

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &IdleColor),
        Changed<Interaction>,
    >,
) {
    for (interaction, mut button_color, IdleColor(idle_color)) in interaction_query.iter_mut() {
        if matches!(interaction, Interaction::Hovered) {
            *button_color = Color::ORANGE_RED.into();
        } else {
            *button_color = *idle_color;
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    warn!(include_str!("warning_string.txt"));
    let image = if 0 < args.image_freq {
        Some(asset_server.load("branding/icon.png"))
    } else {
        None
    };

    let count = args.buttons;
    let count_f = count as f32;
    let as_rainbow = |i: usize| Color::hsl((i as f32 / count_f) * 360.0, 0.9, 0.8);
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: Val::Percent(100.),
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            let border = if args.no_borders {
                UiRect::ZERO
            } else {
                let thickness = 0.05 * 90. / count_f;
                UiRect::all(Val::VMin(thickness))
            };
            for i in 0..count {
                commands
                    .spawn(NodeBundle::default())
                    .with_children(|commands| {
                        for j in 0..count {
                            let color = as_rainbow(j % i.max(1)).into();
                            let border_color = Color::WHITE.with_a(0.5).into();
                            spawn_button(
                                commands,
                                color,
                                count_f,
                                i,
                                j,
                                !args.no_text,
                                border,
                                border_color,
                                image
                                    .as_ref()
                                    .filter(|_| (i + j) % args.image_freq == 0)
                                    .cloned(),
                            );
                        }
                    });
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
    image: Option<Handle<Image>>,
) {
    let width = Val::Vw(90.0 / total);
    let height = Val::Vh(90.0 / total);
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
            background_color,
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
            parent.spawn(
                TextBundle::from_section(
                    format!("{i}, {j}"),
                    TextStyle {
                        font_size: FONT_SIZE,
                        color: Color::rgb(0.2, 0.2, 0.2),
                        ..default()
                    },
                )
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    ..default()
                }),
            );
        });
    }
}
