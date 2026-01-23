//! UI testbed
//!
//! You can switch scene by pressing the spacebar

mod helpers;

use argh::FromArgs;
use bevy::prelude::*;
use helpers::Next;

#[derive(FromArgs)]
/// ui testbed
pub struct Args {
    #[argh(positional)]
    scene: Option<Scene>,
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args: Args = Args::from_args(&[], &[]).unwrap();

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            // The ViewportCoords scene relies on these specific viewport dimensions,
            // so let's explicitly define them and set resizable to false
            resolution: (1280, 720).into(),
            resizable: false,
            ..Default::default()
        }),
        ..Default::default()
    }))
    .add_systems(OnEnter(Scene::Image), image::setup)
    .add_systems(OnEnter(Scene::Text), text::setup)
    .add_systems(OnEnter(Scene::Grid), grid::setup)
    .add_systems(OnEnter(Scene::Borders), borders::setup)
    .add_systems(OnEnter(Scene::BoxShadow), box_shadow::setup)
    .add_systems(OnEnter(Scene::TextWrap), text_wrap::setup)
    .add_systems(OnEnter(Scene::Overflow), overflow::setup)
    .add_systems(OnEnter(Scene::Slice), slice::setup)
    .add_systems(OnEnter(Scene::LayoutRounding), layout_rounding::setup)
    .add_systems(OnEnter(Scene::LinearGradient), linear_gradient::setup)
    .add_systems(OnEnter(Scene::RadialGradient), radial_gradient::setup)
    .add_systems(OnEnter(Scene::Transformations), transformations::setup)
    .add_systems(OnEnter(Scene::ViewportCoords), viewport_coords::setup)
    .add_systems(Update, switch_scene);

    match args.scene {
        None => app.init_state::<Scene>(),
        Some(scene) => app.insert_state(scene),
    };

    #[cfg(feature = "bevy_ui_debug")]
    {
        app.add_systems(OnEnter(Scene::DebugOutlines), debug_outlines::setup);
        app.add_systems(OnExit(Scene::DebugOutlines), debug_outlines::teardown);
    }

    #[cfg(feature = "bevy_ci_testing")]
    app.add_systems(Update, helpers::switch_scene_in_ci::<Scene>);

    app.run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
#[states(scoped_entities)]
enum Scene {
    #[default]
    Image,
    Text,
    Grid,
    Borders,
    BoxShadow,
    TextWrap,
    Overflow,
    Slice,
    LayoutRounding,
    LinearGradient,
    RadialGradient,
    Transformations,
    #[cfg(feature = "bevy_ui_debug")]
    DebugOutlines,
    ViewportCoords,
}

impl std::str::FromStr for Scene {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut isit = Self::default();
        while s.to_lowercase() != format!("{isit:?}").to_lowercase() {
            isit = isit.next();
            if isit == Self::default() {
                return Err(format!("Invalid Scene name: {s}"));
            }
        }
        Ok(isit)
    }
}

impl Next for Scene {
    fn next(&self) -> Self {
        match self {
            Scene::Image => Scene::Text,
            Scene::Text => Scene::Grid,
            Scene::Grid => Scene::Borders,
            Scene::Borders => Scene::BoxShadow,
            Scene::BoxShadow => Scene::TextWrap,
            Scene::TextWrap => Scene::Overflow,
            Scene::Overflow => Scene::Slice,
            Scene::Slice => Scene::LayoutRounding,
            Scene::LayoutRounding => Scene::LinearGradient,
            Scene::LinearGradient => Scene::RadialGradient,
            #[cfg(feature = "bevy_ui_debug")]
            Scene::RadialGradient => Scene::DebugOutlines,
            #[cfg(feature = "bevy_ui_debug")]
            Scene::DebugOutlines => Scene::Transformations,
            #[cfg(not(feature = "bevy_ui_debug"))]
            Scene::RadialGradient => Scene::Transformations,
            Scene::Transformations => Scene::ViewportCoords,
            Scene::ViewportCoords => Scene::Image,
        }
    }
}

fn switch_scene(
    keyboard: Res<ButtonInput<KeyCode>>,
    scene: Res<State<Scene>>,
    mut next_scene: ResMut<NextState<Scene>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        info!("Switching scene");
        next_scene.set(scene.get().next());
    }
}

mod image {
    use bevy::prelude::*;

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Image)));
        commands.spawn((
            ImageNode::new(asset_server.load("branding/bevy_logo_dark.png")),
            DespawnOnExit(super::Scene::Image),
        ));
    }
}

mod text {
    use bevy::{color::palettes::css::*, prelude::*, text::FontSmoothing};

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Text)));
        commands.spawn((
            Text::new("Hello World."),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: 200.,
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
        ));

        commands.spawn((
            Node {
                left: px(100.),
                top: px(230.),
                ..Default::default()
            },
            Text::new("white "),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                (TextSpan::new("red "), TextColor(RED.into()),),
                (TextSpan::new("green "), TextColor(GREEN.into()),),
                (TextSpan::new("blue "), TextColor(BLUE.into()),),
                (
                    TextSpan::new("black"),
                    TextColor(Color::BLACK),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                        ..default()
                    },
                    TextBackgroundColor(Color::WHITE)
                ),
            ],
        ));

        commands.spawn((
            Node {
                left: px(100.),
                top: px(260.),
                ..Default::default()
            },
            Text::new(""),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                (
                    TextSpan::new("white "),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                        ..default()
                    }
                ),
                (TextSpan::new("red "), TextColor(RED.into()),),
                (TextSpan::new("green "), TextColor(GREEN.into()),),
                (TextSpan::new("blue "), TextColor(BLUE.into()),),
                (
                    TextSpan::new("black"),
                    TextColor(Color::BLACK),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                        ..default()
                    },
                    TextBackgroundColor(Color::WHITE)
                ),
            ],
        ));

        let mut top = 300.;
        commands.spawn((
            Node {
                left: px(100.),
                top: px(top),
                ..Default::default()
            },
            Text::new(""),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                (TextSpan::new(""), TextColor(YELLOW.into()),),
                TextSpan::new(""),
                (
                    TextSpan::new("white "),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                        ..default()
                    }
                ),
                TextSpan::new(""),
                (TextSpan::new("red "), TextColor(RED.into()),),
                TextSpan::new(""),
                TextSpan::new(""),
                (TextSpan::new("green "), TextColor(GREEN.into()),),
                (TextSpan::new(""), TextColor(YELLOW.into()),),
                (TextSpan::new("blue "), TextColor(BLUE.into()),),
                TextSpan::new(""),
                (TextSpan::new(""), TextColor(YELLOW.into()),),
                (
                    TextSpan::new("black"),
                    TextColor(Color::BLACK),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                        ..default()
                    },
                    TextBackgroundColor(Color::WHITE)
                ),
                TextSpan::new(""),
            ],
        ));

        top += 35.;
        commands.spawn((
            Node {
                left: px(100.),
                top: px(top),
                ..Default::default()
            },
            Text::new("FiraSans_"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: 25.,
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                (
                    TextSpan::new("MonaSans_"),
                    TextFont {
                        font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                        font_size: 25.,
                        ..default()
                    }
                ),
                (
                    TextSpan::new("EBGaramond_"),
                    TextFont {
                        font: asset_server.load("fonts/EBGaramond12-Regular.otf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
                (
                    TextSpan::new("FiraMono"),
                    TextFont {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
            ],
        ));
        top += 35.;
        commands.spawn((
            Node {
                left: px(100.),
                top: px(top),
                ..Default::default()
            },
            Text::new("FiraSans "),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: 25.,
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                (
                    TextSpan::new("MonaSans "),
                    TextFont {
                        font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                        font_size: 25.,
                        ..default()
                    }
                ),
                (
                    TextSpan::new("EBGaramond "),
                    TextFont {
                        font: asset_server.load("fonts/EBGaramond12-Regular.otf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
                (
                    TextSpan::new("FiraMono"),
                    TextFont {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
            ],
        ));

        top += 35.;
        commands.spawn((
            Node {
                left: px(100.),
                top: px(top),
                ..Default::default()
            },
            Text::new("FiraSans "),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: 25.,
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                (
                    TextSpan::new("MonaSans_"),
                    TextFont {
                        font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                        font_size: 25.,
                        ..default()
                    }
                ),
                (
                    TextSpan::new("EBGaramond "),
                    TextFont {
                        font: asset_server.load("fonts/EBGaramond12-Regular.otf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
                (
                    TextSpan::new("FiraMono"),
                    TextFont {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
            ],
        ));

        top += 35.;
        commands.spawn((
            Node {
                left: px(100.),
                top: px(top),
                ..Default::default()
            },
            Text::new("FiraSans"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: 25.,
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                TextSpan::new(" "),
                (
                    TextSpan::new("MonaSans"),
                    TextFont {
                        font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                        font_size: 25.,
                        ..default()
                    }
                ),
                TextSpan::new(" "),
                (
                    TextSpan::new("EBGaramond"),
                    TextFont {
                        font: asset_server.load("fonts/EBGaramond12-Regular.otf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
                TextSpan::new(" "),
                (
                    TextSpan::new("FiraMono"),
                    TextFont {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
            ],
        ));

        top += 35.;
        commands.spawn((
            Node {
                left: px(100.),
                top: px(top),
                ..Default::default()
            },
            Text::new("Fira Sans_"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: 25.,
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                (
                    TextSpan::new("Mona Sans_"),
                    TextFont {
                        font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                        font_size: 25.,
                        ..default()
                    }
                ),
                (
                    TextSpan::new("EB Garamond_"),
                    TextFont {
                        font: asset_server.load("fonts/EBGaramond12-Regular.otf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
                (
                    TextSpan::new("Fira Mono"),
                    TextFont {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
            ],
        ));

        top += 35.;
        commands.spawn((
            Node {
                left: px(100.),
                top: px(top),
                ..Default::default()
            },
            Text::new("FontWeight(100)_"),
            TextFont {
                font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                font_size: 25.,
                weight: FontWeight(100),
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                (
                    TextSpan::new("FontWeight(500)_"),
                    TextFont {
                        font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                        font_size: 25.,
                        weight: FontWeight(500),
                        ..default()
                    }
                ),
                (
                    TextSpan::new("FontWeight(900)"),
                    TextFont {
                        font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                        font_size: 25.,
                        weight: FontWeight(900),
                        ..default()
                    },
                ),
            ],
        ));

        top += 35.;
        commands.spawn((
            Node {
                left: px(100.),
                top: px(top),
                ..Default::default()
            },
            Text::new("FiraSans_"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: 25.,
                weight: FontWeight(900),
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                (
                    TextSpan::new("MonaSans_"),
                    TextFont {
                        font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                        font_size: 25.,
                        weight: FontWeight(700),
                        ..default()
                    }
                ),
                (
                    TextSpan::new("EBGaramond_"),
                    TextFont {
                        font: asset_server.load("fonts/EBGaramond12-Regular.otf").into(),
                        font_size: 25.,
                        weight: FontWeight(500),
                        ..default()
                    },
                ),
                (
                    TextSpan::new("FiraMono"),
                    TextFont {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                        font_size: 25.,
                        weight: FontWeight(300),
                        ..default()
                    },
                ),
            ],
        ));

        top += 35.;
        commands.spawn((
            Node {
                left: px(100.),
                top: px(top),
                ..Default::default()
            },
            Text::new("FiraSans\t"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: 25.,
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
            children![
                (
                    TextSpan::new("MonaSans\t"),
                    TextFont {
                        font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                        font_size: 25.,
                        ..default()
                    }
                ),
                (
                    TextSpan::new("EBGaramond\t"),
                    TextFont {
                        font: asset_server.load("fonts/EBGaramond12-Regular.otf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
                (
                    TextSpan::new("FiraMono"),
                    TextFont {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                        font_size: 25.,
                        ..default()
                    },
                ),
            ],
        ));

        for font_smoothing in [FontSmoothing::AntiAliased, FontSmoothing::None] {
            top += 35.;
            commands.spawn((
                Node {
                    left: px(100.),
                    top: px(top),
                    ..Default::default()
                },
                Text::new(format!("FontSmoothing::{:?}", font_smoothing)),
                TextFont {
                    font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                    font_size: 25.,
                    font_smoothing,
                    ..default()
                },
                DespawnOnExit(super::Scene::Text),
            ));
        }
    }
}

mod grid {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Grid)));
        // Top-level grid (app frame)
        commands.spawn((
            Node {
                display: Display::Grid,
                width: percent(100),
                height: percent(100),
                grid_template_columns: vec![GridTrack::min_content(), GridTrack::flex(1.0)],
                grid_template_rows: vec![
                    GridTrack::auto(),
                    GridTrack::flex(1.0),
                    GridTrack::px(40.),
                ],
                ..default()
            },
            BackgroundColor(Color::WHITE),
            DespawnOnExit(super::Scene::Grid),
            children![
                // Header
                (
                    Node {
                        display: Display::Grid,
                        grid_column: GridPlacement::span(2),
                        padding: UiRect::all(px(40)),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                ),
                // Main content grid (auto placed in row 2, column 1)
                (
                    Node {
                        height: percent(100),
                        aspect_ratio: Some(1.0),
                        display: Display::Grid,
                        grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
                        grid_template_rows: RepeatedGridTrack::flex(2, 1.0),
                        row_gap: px(12),
                        column_gap: px(12),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
                    children![
                        (Node::default(), BackgroundColor(ORANGE.into())),
                        (Node::default(), BackgroundColor(BISQUE.into())),
                        (Node::default(), BackgroundColor(BLUE.into())),
                        (Node::default(), BackgroundColor(CRIMSON.into())),
                        (Node::default(), BackgroundColor(AQUA.into())),
                    ]
                ),
                // Right side bar (auto placed in row 2, column 2)
                (Node::DEFAULT, BackgroundColor(BLACK.into())),
            ],
        ));
    }
}

mod borders {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Borders)));
        let root = commands
            .spawn((
                Node {
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                DespawnOnExit(super::Scene::Borders),
            ))
            .id();

        // all the different combinations of border edges
        let borders = [
            UiRect::default(),
            UiRect::all(px(20)),
            UiRect::left(px(20)),
            UiRect::vertical(px(20)),
            UiRect {
                left: px(40),
                top: px(20),
                ..Default::default()
            },
            UiRect {
                right: px(20),
                bottom: px(30),
                ..Default::default()
            },
            UiRect {
                right: px(20),
                top: px(40),
                bottom: px(20),
                ..Default::default()
            },
            UiRect {
                left: px(20),
                top: px(20),
                bottom: px(20),
                ..Default::default()
            },
            UiRect {
                left: px(20),
                right: px(20),
                bottom: px(40),
                ..Default::default()
            },
        ];

        let non_zero = |x, y| x != px(0) && y != px(0);
        let border_size = |x, y| if non_zero(x, y) { f32::MAX } else { 0. };

        for border in borders {
            for rounded in [true, false] {
                let border_node = commands
                    .spawn((
                        Node {
                            width: px(100),
                            height: px(100),
                            border,
                            margin: UiRect::all(px(30)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border_radius: if rounded {
                                BorderRadius::px(
                                    border_size(border.left, border.top),
                                    border_size(border.right, border.top),
                                    border_size(border.right, border.bottom),
                                    border_size(border.left, border.bottom),
                                )
                            } else {
                                BorderRadius::ZERO
                            },
                            ..default()
                        },
                        BackgroundColor(MAROON.into()),
                        BorderColor::all(RED),
                        Outline {
                            width: px(10),
                            offset: px(10),
                            color: Color::WHITE,
                        },
                    ))
                    .id();

                commands.entity(root).add_child(border_node);
            }
        }
    }
}

mod box_shadow {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::BoxShadow)));

        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    padding: UiRect::all(px(30)),
                    column_gap: px(200),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                BackgroundColor(GREEN.into()),
                DespawnOnExit(super::Scene::BoxShadow),
            ))
            .with_children(|commands| {
                let example_nodes = [
                    (
                        Vec2::splat(100.),
                        Vec2::ZERO,
                        10.,
                        0.,
                        BorderRadius::bottom_right(px(10)),
                    ),
                    (Vec2::new(200., 50.), Vec2::ZERO, 10., 0., BorderRadius::MAX),
                    (
                        Vec2::new(100., 50.),
                        Vec2::ZERO,
                        10.,
                        10.,
                        BorderRadius::ZERO,
                    ),
                    (
                        Vec2::splat(100.),
                        Vec2::splat(20.),
                        10.,
                        10.,
                        BorderRadius::bottom_right(px(10)),
                    ),
                    (
                        Vec2::splat(100.),
                        Vec2::splat(50.),
                        0.,
                        10.,
                        BorderRadius::ZERO,
                    ),
                    (
                        Vec2::new(50., 100.),
                        Vec2::splat(10.),
                        0.,
                        10.,
                        BorderRadius::MAX,
                    ),
                ];

                for (size, offset, spread, blur, border_radius) in example_nodes {
                    commands.spawn((
                        Node {
                            width: px(size.x),
                            height: px(size.y),
                            border: UiRect::all(px(2)),
                            border_radius,
                            ..default()
                        },
                        BorderColor::all(WHITE),
                        BackgroundColor(BLUE.into()),
                        BoxShadow::new(
                            Color::BLACK.with_alpha(0.9),
                            percent(offset.x),
                            percent(offset.y),
                            percent(spread),
                            px(blur),
                        ),
                    ));
                }
            });
    }
}

mod text_wrap {
    use bevy::prelude::*;

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::TextWrap)));

        let root = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    width: px(200),
                    height: percent(100),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                DespawnOnExit(super::Scene::TextWrap),
            ))
            .id();

        for linebreak in [
            LineBreak::AnyCharacter,
            LineBreak::WordBoundary,
            LineBreak::WordOrCharacter,
            LineBreak::NoWrap,
        ] {
            let messages = [
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit.".to_string(),
                "pneumonoultramicroscopicsilicovolcanoconiosis".to_string(),
            ];

            for (j, message) in messages.into_iter().enumerate() {
                commands.entity(root).with_child((
                    Text(message.clone()),
                    TextLayout::new(Justify::Left, linebreak),
                    BackgroundColor(Color::srgb(0.8 - j as f32 * 0.3, 0., 0.)),
                ));
            }
        }
    }
}

mod overflow {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Overflow)));
        let image = asset_server.load("branding/icon.png");

        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceAround,
                    ..Default::default()
                },
                BackgroundColor(BLUE.into()),
                DespawnOnExit(super::Scene::Overflow),
            ))
            .with_children(|parent| {
                for overflow in [
                    Overflow::visible(),
                    Overflow::clip_x(),
                    Overflow::clip_y(),
                    Overflow::clip(),
                ] {
                    parent
                        .spawn((
                            Node {
                                width: px(100),
                                height: px(100),
                                padding: UiRect {
                                    left: px(25),
                                    top: px(25),
                                    ..Default::default()
                                },
                                border: UiRect::all(px(5)),
                                overflow,
                                ..default()
                            },
                            BorderColor::all(RED),
                            BackgroundColor(Color::WHITE),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                ImageNode::new(image.clone()),
                                Node {
                                    min_width: px(100),
                                    min_height: px(100),
                                    ..default()
                                },
                                Interaction::default(),
                                Outline {
                                    width: px(2),
                                    offset: px(2),
                                    color: Color::NONE,
                                },
                            ));
                        });
                }
            });
    }
}

mod slice {
    use bevy::prelude::*;

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Slice)));
        let image = asset_server.load("textures/fantasy_ui_borders/numbered_slices.png");

        let slicer = TextureSlicer {
            border: BorderRect::all(16.0),
            center_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
            sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
            ..default()
        };
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceAround,
                    ..default()
                },
                DespawnOnExit(super::Scene::Slice),
            ))
            .with_children(|parent| {
                for [w, h] in [[150.0, 150.0], [300.0, 150.0], [150.0, 300.0]] {
                    parent.spawn((
                        Button,
                        ImageNode {
                            image: image.clone(),
                            image_mode: NodeImageMode::Sliced(slicer.clone()),
                            ..default()
                        },
                        Node {
                            width: px(w),
                            height: px(h),
                            ..default()
                        },
                    ));
                }

                parent.spawn((
                    ImageNode {
                        image: asset_server
                            .load("textures/fantasy_ui_borders/panel-border-010.png"),
                        image_mode: NodeImageMode::Sliced(TextureSlicer {
                            border: BorderRect::all(22.0),
                            center_scale_mode: SliceScaleMode::Stretch,
                            sides_scale_mode: SliceScaleMode::Stretch,
                            max_corner_scale: 1.0,
                        }),
                        ..Default::default()
                    },
                    Node {
                        width: px(100),
                        height: px(100),
                        ..default()
                    },
                    BackgroundColor(bevy::color::palettes::css::NAVY.into()),
                ));
            });
    }
}

mod layout_rounding {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::LayoutRounding)));

        commands
            .spawn((
                Node {
                    display: Display::Grid,
                    width: percent(100),
                    height: percent(100),
                    grid_template_rows: vec![RepeatedGridTrack::fr(10, 1.)],
                    ..Default::default()
                },
                BackgroundColor(Color::WHITE),
                DespawnOnExit(super::Scene::LayoutRounding),
            ))
            .with_children(|commands| {
                for i in 2..12 {
                    commands
                        .spawn(Node {
                            display: Display::Grid,
                            grid_template_columns: vec![RepeatedGridTrack::fr(i, 1.)],
                            ..Default::default()
                        })
                        .with_children(|commands| {
                            for _ in 0..i {
                                commands.spawn((
                                    Node {
                                        border: UiRect::all(px(5)),
                                        ..Default::default()
                                    },
                                    BackgroundColor(MAROON.into()),
                                    BorderColor::all(DARK_BLUE),
                                ));
                            }
                        });
                }
            });
    }
}

mod linear_gradient {
    use bevy::camera::Camera2d;
    use bevy::color::palettes::css::BLUE;
    use bevy::color::palettes::css::LIME;
    use bevy::color::palettes::css::RED;
    use bevy::color::palettes::css::YELLOW;
    use bevy::color::Color;
    use bevy::ecs::prelude::*;
    use bevy::state::state_scoped::DespawnOnExit;
    use bevy::text::TextFont;
    use bevy::ui::AlignItems;
    use bevy::ui::BackgroundGradient;
    use bevy::ui::ColorStop;
    use bevy::ui::GridPlacement;
    use bevy::ui::InterpolationColorSpace;
    use bevy::ui::JustifyContent;
    use bevy::ui::LinearGradient;
    use bevy::ui::Node;
    use bevy::ui::PositionType;
    use bevy::utils::default;

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::LinearGradient)));
        commands
            .spawn((
                Node {
                    flex_direction: bevy::ui::FlexDirection::Column,
                    width: bevy::ui::percent(100),
                    height: bevy::ui::percent(100),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: bevy::ui::px(5),
                    ..default()
                },
                DespawnOnExit(super::Scene::LinearGradient),
            ))
            .with_children(|commands| {
                let mut i = 0;
                commands
                    .spawn(Node {
                        display: bevy::ui::Display::Grid,
                        row_gap: bevy::ui::px(4),
                        column_gap: bevy::ui::px(4),
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        for stops in [
                            vec![ColorStop::auto(RED), ColorStop::auto(YELLOW)],
                            vec![
                                ColorStop::auto(Color::BLACK),
                                ColorStop::auto(RED),
                                ColorStop::auto(Color::WHITE),
                            ],
                            vec![
                                Color::hsl(180.71191, 0.0, 0.3137255).into(),
                                Color::hsl(180.71191, 0.5, 0.3137255).into(),
                                Color::hsl(180.71191, 1.0, 0.3137255).into(),
                            ],
                            vec![
                                Color::hsl(180.71191, 0.825, 0.0).into(),
                                Color::hsl(180.71191, 0.825, 0.5).into(),
                                Color::hsl(180.71191, 0.825, 1.0).into(),
                            ],
                            vec![
                                Color::hsl(0.0 + 0.0001, 1.0, 0.5).into(),
                                Color::hsl(180.0, 1.0, 0.5).into(),
                                Color::hsl(360.0 - 0.0001, 1.0, 0.5).into(),
                            ],
                            vec![
                                Color::WHITE.into(),
                                RED.into(),
                                LIME.into(),
                                BLUE.into(),
                                Color::BLACK.into(),
                            ],
                        ] {
                            for color_space in [
                                InterpolationColorSpace::LinearRgba,
                                InterpolationColorSpace::Srgba,
                                InterpolationColorSpace::Oklaba,
                                InterpolationColorSpace::Oklcha,
                                InterpolationColorSpace::OklchaLong,
                                InterpolationColorSpace::Hsla,
                                InterpolationColorSpace::HslaLong,
                                InterpolationColorSpace::Hsva,
                                InterpolationColorSpace::HsvaLong,
                            ] {
                                let row = i % 18 + 1;
                                let column = i / 18 + 1;
                                i += 1;

                                commands.spawn((
                                    Node {
                                        grid_row: GridPlacement::start(row as i16 + 1),
                                        grid_column: GridPlacement::start(column as i16 + 1),
                                        justify_content: JustifyContent::SpaceEvenly,
                                        ..Default::default()
                                    },
                                    children![(
                                        Node {
                                            height: bevy::ui::px(30),
                                            width: bevy::ui::px(300),
                                            justify_content: JustifyContent::Center,
                                            ..Default::default()
                                        },
                                        BackgroundGradient::from(LinearGradient {
                                            color_space,
                                            angle: LinearGradient::TO_RIGHT,
                                            stops: stops.clone(),
                                        }),
                                        children![
                                            Node {
                                                position_type: PositionType::Absolute,
                                                ..default()
                                            },
                                            TextFont::from_font_size(10.),
                                            bevy::ui::widget::Text(format!("{color_space:?}")),
                                        ]
                                    )],
                                ));
                            }
                        }
                    });
            });
    }
}

mod radial_gradient {
    use bevy::color::palettes::css::RED;
    use bevy::color::palettes::tailwind::GRAY_700;
    use bevy::prelude::*;
    use bevy::ui::ColorStop;

    const CELL_SIZE: f32 = 80.;
    const GAP: f32 = 10.;

    pub fn setup(mut commands: Commands) {
        let color_stops = vec![
            ColorStop::new(Color::BLACK, px(5)),
            ColorStop::new(Color::WHITE, px(5)),
            ColorStop::new(Color::WHITE, percent(100)),
            ColorStop::auto(RED),
        ];

        commands.spawn((Camera2d, DespawnOnExit(super::Scene::RadialGradient)));
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    display: Display::Grid,
                    align_items: AlignItems::Start,
                    grid_template_columns: vec![RepeatedGridTrack::px(
                        GridTrackRepetition::AutoFill,
                        CELL_SIZE,
                    )],
                    grid_auto_flow: GridAutoFlow::Row,
                    row_gap: px(GAP),
                    column_gap: px(GAP),
                    padding: UiRect::all(px(GAP)),
                    ..default()
                },
                DespawnOnExit(super::Scene::RadialGradient),
            ))
            .with_children(|commands| {
                for (shape, shape_label) in [
                    (RadialGradientShape::ClosestSide, "ClosestSide"),
                    (RadialGradientShape::FarthestSide, "FarthestSide"),
                    (RadialGradientShape::Circle(percent(55)), "Circle(55%)"),
                    (RadialGradientShape::FarthestCorner, "FarthestCorner"),
                ] {
                    for (position, position_label) in [
                        (UiPosition::TOP_LEFT, "TOP_LEFT"),
                        (UiPosition::LEFT, "LEFT"),
                        (UiPosition::BOTTOM_LEFT, "BOTTOM_LEFT"),
                        (UiPosition::TOP, "TOP"),
                        (UiPosition::CENTER, "CENTER"),
                        (UiPosition::BOTTOM, "BOTTOM"),
                        (UiPosition::TOP_RIGHT, "TOP_RIGHT"),
                        (UiPosition::RIGHT, "RIGHT"),
                        (UiPosition::BOTTOM_RIGHT, "BOTTOM_RIGHT"),
                    ] {
                        for (w, h) in [(CELL_SIZE, CELL_SIZE), (CELL_SIZE, CELL_SIZE / 2.)] {
                            commands
                                .spawn((
                                    BackgroundColor(GRAY_700.into()),
                                    Node {
                                        display: Display::Grid,
                                        width: px(CELL_SIZE),
                                        ..Default::default()
                                    },
                                ))
                                .with_children(|commands| {
                                    commands.spawn((
                                        Node {
                                            margin: UiRect::all(px(2)),
                                            ..default()
                                        },
                                        Text(format!("{shape_label}\n{position_label}")),
                                        TextFont::from_font_size(9.),
                                    ));
                                    commands.spawn((
                                        Node {
                                            width: px(w),
                                            height: px(h),
                                            ..default()
                                        },
                                        BackgroundGradient::from(RadialGradient {
                                            stops: color_stops.clone(),
                                            position,
                                            shape,
                                            ..default()
                                        }),
                                    ));
                                });
                        }
                    }
                }
            });
    }
}

mod transformations {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Transformations)));
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    display: Display::Block,
                    ..default()
                },
                DespawnOnExit(super::Scene::Transformations),
            ))
            .with_children(|parent| {
                for (transformation, label, background) in [
                    (
                        UiTransform::from_rotation(Rot2::degrees(45.)),
                        "Rotate 45 degrees",
                        RED,
                    ),
                    (
                        UiTransform::from_scale(Vec2::new(2., 0.5)),
                        "Scale 2.x 0.5y",
                        GREEN,
                    ),
                    (
                        UiTransform::from_translation(Val2::px(-50., 50.)),
                        "Translate -50px x +50px y",
                        BLUE,
                    ),
                    (
                        UiTransform {
                            translation: Val2::px(50., 0.),
                            scale: Vec2::new(-1., 1.),
                            rotation: Rot2::degrees(30.),
                        },
                        "T 50px x\nS -1.x (refl)\nR 30deg",
                        DARK_CYAN,
                    ),
                ] {
                    parent
                        .spawn((Node {
                            width: percent(100),
                            margin: UiRect {
                                top: px(50),
                                bottom: px(50),
                                ..default()
                            },
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceAround,
                            ..default()
                        },))
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Before Tf"),
                                Node {
                                    width: px(100),
                                    height: px(100),
                                    border_radius: BorderRadius::bottom_right(px(25.)),
                                    ..default()
                                },
                                BackgroundColor(background.into()),
                                TextFont::default(),
                            ));
                            row.spawn((
                                Text::new(label),
                                Node {
                                    width: px(100),
                                    height: px(100),
                                    border_radius: BorderRadius::bottom_right(px(25.)),
                                    ..default()
                                },
                                BackgroundColor(background.into()),
                                transformation,
                                TextFont::default(),
                            ));
                        });
                }
            });
    }
}

#[cfg(feature = "bevy_ui_debug")]
mod debug_outlines {
    use bevy::{
        color::palettes::css::{BLUE, GRAY, RED},
        prelude::*,
        ui_render::UiDebugOptions,
    };

    pub fn setup(mut commands: Commands, mut debug_options: ResMut<UiDebugOptions>) {
        debug_options.enabled = true;
        debug_options.line_width = 5.;
        debug_options.line_color_override = Some(LinearRgba::GREEN);
        debug_options.show_hidden = true;
        debug_options.show_clipped = true;
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::DebugOutlines)));
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(50),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceAround,
                    ..default()
                },
                DespawnOnExit(super::Scene::DebugOutlines),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Node {
                        width: px(100),
                        height: px(100),
                        ..default()
                    },
                    BackgroundColor(GRAY.into()),
                    UiTransform::from_rotation(Rot2::degrees(45.)),
                ));

                parent.spawn((Text::new("Regular Text"), TextFont::default()));

                parent.spawn((
                    Node {
                        width: px(100),
                        height: px(100),
                        ..default()
                    },
                    Text::new("Invisible"),
                    BackgroundColor(GRAY.into()),
                    TextFont::default(),
                    Visibility::Hidden,
                ));

                parent
                    .spawn((
                        Node {
                            width: px(100),
                            height: px(100),
                            padding: UiRect {
                                left: px(25),
                                top: px(25),
                                ..Default::default()
                            },
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(RED.into()),
                    ))
                    .with_children(|child| {
                        child.spawn((
                            Node {
                                min_width: px(100),
                                min_height: px(100),
                                ..default()
                            },
                            BackgroundColor(BLUE.into()),
                        ));
                    });
            });

        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(50),
                    top: percent(50),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceAround,
                    ..default()
                },
                DespawnOnExit(super::Scene::DebugOutlines),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Node {
                        width: px(200),
                        height: px(200),
                        border: UiRect {
                            top: px(10),
                            bottom: px(20),
                            left: px(30),
                            right: px(40),
                        },
                        border_radius: BorderRadius::bottom_right(px(10)),
                        padding: UiRect {
                            top: px(40),
                            bottom: px(30),
                            left: px(20),
                            right: px(10),
                        },
                        ..default()
                    },
                    children![(
                        Text::new("border padding content outlines"),
                        TextFont::default(),
                        UiDebugOptions {
                            enabled: false,
                            ..default()
                        }
                    )],
                    UiDebugOptions {
                        outline_border_box: true,
                        outline_padding_box: true,
                        outline_content_box: true,
                        ignore_border_radius: false,
                        ..*debug_options
                    },
                ));

                // Vertical scrollbar (non-functional)
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: px(100),
                        height: px(100),
                        overflow: Overflow::scroll_y(),
                        scrollbar_width: 20.,
                        ..default()
                    },
                    UiDebugOptions {
                        line_width: 3.,
                        outline_scrollbars: true,
                        show_hidden: false,
                        show_clipped: false,
                        ..*debug_options
                    },
                    Children::spawn(SpawnIter((0..6).map(move |i| {
                        (
                            Node::default(),
                            children![(
                                Text(format!("Item {i}")),
                                UiDebugOptions {
                                    enabled: false,
                                    ..default()
                                }
                            )],
                            UiDebugOptions {
                                enabled: false,
                                ..default()
                            },
                        )
                    }))),
                ));

                // Horizontal scrollbar (non-functional)
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        width: px(100),
                        height: px(100),
                        overflow: Overflow::scroll_x(),
                        scrollbar_width: 20.,
                        ..default()
                    },
                    UiDebugOptions {
                        line_width: 3.,
                        outline_scrollbars: true,
                        show_hidden: false,
                        show_clipped: false,
                        ..*debug_options
                    },
                    Children::spawn(SpawnIter((0..6).map(move |i| {
                        (
                            Node::default(),
                            children![(
                                Text(format!("Item {i}")),
                                UiDebugOptions {
                                    enabled: false,
                                    ..default()
                                }
                            )],
                            UiDebugOptions {
                                enabled: false,
                                ..default()
                            },
                        )
                    }))),
                ));

                // bi-directional scrollbar (non-functional)
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: px(100),
                        height: px(100),
                        overflow: Overflow::scroll(),
                        scrollbar_width: 20.,
                        ..default()
                    },
                    UiDebugOptions {
                        line_width: 3.,
                        outline_scrollbars: true,
                        show_hidden: false,
                        show_clipped: false,
                        ..*debug_options
                    },
                    Children::spawn(SpawnIter((0..6).map(move |i| {
                        (
                            Node {
                                flex_direction: FlexDirection::Row,
                                ..default()
                            },
                            Children::spawn(SpawnIter((0..6).map({
                                move |j| {
                                    (
                                        Text(format!("Item {}", (i * 5) + j)),
                                        UiDebugOptions {
                                            enabled: false,
                                            ..default()
                                        },
                                    )
                                }
                            }))),
                            UiDebugOptions {
                                enabled: false,
                                ..default()
                            },
                        )
                    }))),
                ));
            });
    }

    pub fn teardown(mut debug_options: ResMut<UiDebugOptions>) {
        *debug_options = UiDebugOptions::default();
    }
}

mod viewport_coords {
    use bevy::{color::palettes::css::*, prelude::*};

    const PALETTE: [Srgba; 9] = [RED, WHITE, BEIGE, AQUA, CRIMSON, NAVY, AZURE, LIME, BLACK];

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::ViewportCoords)));
        commands
            .spawn((
                Node {
                    width: vw(100),
                    height: vh(100),
                    border: UiRect::axes(vw(5), vh(5)),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                BorderColor::all(PALETTE[0]),
                DespawnOnExit(super::Scene::ViewportCoords),
            ))
            .with_children(|builder| {
                builder.spawn((
                    Node {
                        width: vw(30),
                        height: vh(30),
                        border: UiRect::all(vmin(5)),
                        ..default()
                    },
                    BackgroundColor(PALETTE[1].into()),
                    BorderColor::all(PALETTE[8]),
                ));

                builder.spawn((
                    Node {
                        width: vw(60),
                        height: vh(30),
                        ..default()
                    },
                    BackgroundColor(PALETTE[2].into()),
                ));

                builder.spawn((
                    Node {
                        width: vw(45),
                        height: vh(30),
                        border: UiRect::left(vmax(45. / 2.)),
                        ..default()
                    },
                    BackgroundColor(PALETTE[3].into()),
                    BorderColor::all(PALETTE[7]),
                ));

                builder.spawn((
                    Node {
                        width: vw(45),
                        height: vh(30),
                        border: UiRect::right(vmax(45. / 2.)),
                        ..default()
                    },
                    BackgroundColor(PALETTE[4].into()),
                    BorderColor::all(PALETTE[7]),
                ));

                builder.spawn((
                    Node {
                        width: vw(60),
                        height: vh(30),
                        ..default()
                    },
                    BackgroundColor(PALETTE[5].into()),
                ));

                builder.spawn((
                    Node {
                        width: vw(30),
                        height: vh(30),
                        border: UiRect::all(vmin(5)),
                        ..default()
                    },
                    BackgroundColor(PALETTE[6].into()),
                    BorderColor::all(PALETTE[8]),
                ));
            });
    }
}
