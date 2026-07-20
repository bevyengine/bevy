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
    .add_systems(OnEnter(Scene::ImageMeasure), image_measure::setup)
    .add_systems(OnEnter(Scene::Text), text::setup)
    .add_systems(OnEnter(Scene::FontLists), font_lists::setup)
    .add_systems(OnEnter(Scene::TextMeasurement), text_measurement::setup)
    .add_systems(OnEnter(Scene::Grid), grid::setup)
    .add_systems(OnEnter(Scene::Borders), borders::setup)
    .add_systems(
        OnEnter(Scene::EllipticalBorderRadius),
        elliptical_border_radius::setup,
    )
    .add_systems(OnEnter(Scene::BoxShadow), box_shadow::setup)
    .add_systems(OnEnter(Scene::TextWrap), text_wrap::setup)
    .add_systems(OnEnter(Scene::Overflow), overflow::setup)
    .add_systems(OnEnter(Scene::Slice), slice::setup)
    .add_systems(OnEnter(Scene::LayoutRounding), layout_rounding::setup)
    .add_systems(OnEnter(Scene::LinearGradient), linear_gradient::setup)
    .add_systems(OnEnter(Scene::RadialGradient), radial_gradient::setup)
    .add_systems(OnEnter(Scene::Transformations), transformations::setup)
    .add_systems(OnEnter(Scene::ViewportCoords), viewport_coords::setup)
    .add_systems(OnEnter(Scene::OuterColor), outer_color::setup)
    .add_systems(OnEnter(Scene::BoxedContent), boxed_content::setup)
    .add_systems(OnEnter(Scene::EditableText), editable_text::setup)
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, States, Default)]
#[states(scoped_entities)]
enum Scene {
    #[default]
    Image,
    ImageMeasure,
    Text,
    FontLists,
    TextMeasurement,
    Grid,
    Borders,
    EllipticalBorderRadius,
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
    OuterColor,
    BoxedContent,
    EditableText,
}

impl Scene {
    const ALL_ORDERED: &'static [Scene] = &[
        Scene::Image,
        Scene::ImageMeasure,
        Scene::Text,
        Scene::FontLists,
        Scene::TextMeasurement,
        Scene::Grid,
        Scene::Borders,
        Scene::EllipticalBorderRadius,
        Scene::BoxShadow,
        Scene::TextWrap,
        Scene::Overflow,
        Scene::Slice,
        Scene::LayoutRounding,
        Scene::LinearGradient,
        Scene::RadialGradient,
        Scene::Transformations,
        #[cfg(feature = "bevy_ui_debug")]
        Scene::DebugOutlines,
        Scene::ViewportCoords,
        Scene::OuterColor,
        Scene::BoxedContent,
        Scene::EditableText,
    ];
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
        Scene::ALL_ORDERED[(Scene::ALL_ORDERED
            .iter()
            .position(|scene| scene == self)
            .unwrap()
            + 1)
            % Scene::ALL_ORDERED.len()]
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
    use bevy::color::palettes::css::DARK_GREY;
    use bevy::prelude::*;

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Image)));
        commands
            .spawn(Node {
                width: percent(100.),
                height: percent(100.),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceAround,
                align_items: AlignItems::Stretch,
                ..default()
            })
            .with_children(|parent| {
                for [b, p] in [[0, 0], [10, 0], [0, 10], [10, 10]] {
                    for image_path in ["branding/icon.png", "branding/bevy_logo_dark.png"] {
                        parent
                            .spawn(Node {
                                justify_content: JustifyContent::SpaceAround,
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|parent| {
                                for visual_box in [
                                    VisualBox::BorderBox,
                                    VisualBox::PaddingBox,
                                    VisualBox::ContentBox,
                                ] {
                                    parent.spawn((
                                        ImageNode {
                                            image: asset_server.load(image_path),
                                            visual_box,
                                            ..default()
                                        },
                                        Node {
                                            border: px(b).all(),
                                            padding: px(p).all(),
                                            width: px(100.),
                                            ..default()
                                        },
                                        DespawnOnExit(super::Scene::Image),
                                        Outline {
                                            color: DARK_GREY.into(),
                                            width: px(2.),
                                            ..default()
                                        },
                                    ));
                                }
                            });
                    }
                }
            });
    }
}

mod image_measure {
    use bevy::{
        color::palettes::css::{GREEN, RED},
        prelude::*,
    };

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::ImageMeasure)));
        commands.spawn((
            Node {
                margin: auto().all(),
                column_gap: px(5.),
                ..Default::default()
            },
            DespawnOnExit(super::Scene::ImageMeasure),
            children![
                (
                    Node {
                        width: vmin(20.),
                        ..default()
                    },
                    children![(
                        Node {
                            position_type: PositionType::Absolute,
                            width: vmin(20.),
                            ..default()
                        },
                        BackgroundColor(GREEN.into()),
                        ImageNode::new(asset_server.load("branding/icon.png")),
                    )],
                ),
                (
                    Node {
                        width: vmin(20.),
                        ..default()
                    },
                    children![(
                        Node {
                            position_type: PositionType::Absolute,
                            width: vmin(20.),
                            border: px(8.).all(),
                            ..default()
                        },
                        BorderColor::all(RED),
                        BackgroundColor(GREEN.into()),
                        ImageNode::new(asset_server.load("branding/icon.png")),
                    )],
                ),
                (
                    Node {
                        width: vmin(20.),
                        ..default()
                    },
                    children![(
                        Node {
                            position_type: PositionType::Absolute,
                            width: vmin(20.),
                            border: px(8.).all(),
                            padding: px(4.).all(),
                            ..default()
                        },
                        BorderColor::all(RED),
                        BackgroundColor(GREEN.into()),
                        ImageNode::new(asset_server.load("branding/icon.png")),
                    )],
                ),
                (
                    Node {
                        width: vmin(20.),
                        ..default()
                    },
                    children![(
                        Node {
                            position_type: PositionType::Absolute,
                            width: vmin(20.),
                            border: UiRect::px(4.0, 12.0, 8.0, 16.0),
                            ..default()
                        },
                        BorderColor::all(RED),
                        BackgroundColor(GREEN.into()),
                        ImageNode::new(asset_server.load("branding/icon.png")),
                    )],
                ),
                (
                    Node {
                        width: vmin(20.),
                        ..default()
                    },
                    children![(
                        Node {
                            position_type: PositionType::Absolute,
                            width: vmin(20.),
                            border: UiRect::px(4.0, 12.0, 8.0, 16.0),
                            padding: UiRect::axes(px(10.), px(0.)),
                            ..default()
                        },
                        BorderColor::all(RED),
                        BackgroundColor(GREEN.into()),
                        ImageNode::new(asset_server.load("branding/icon.png")),
                    )],
                ),
                (
                    Node {
                        width: vmin(20.),
                        ..default()
                    },
                    children![(
                        Node {
                            position_type: PositionType::Absolute,
                            width: vmin(20.),
                            border: UiRect::px(4.0, 12.0, 8.0, 16.0),
                            padding: UiRect::axes(px(0.), px(10.)),
                            ..default()
                        },
                        BorderColor::all(RED),
                        BackgroundColor(GREEN.into()),
                        ImageNode::new(asset_server.load("branding/icon.png")),
                    )],
                ),
            ],
        ));
    }
}

mod text {
    use bevy::{color::palettes::css::*, prelude::*, text::FontSmoothing};

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Text)));

        let mut container = commands.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                ..default()
            },
            DespawnOnExit(super::Scene::Text),
        ));

        container.with_child((
            Text::new("Hello World."),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: FontSize::Px(100.),
                ..default()
            },
        ));

        container.with_children(|builder| {
            let mut grid = builder.spawn(Node {
                display: Display::Grid,
                grid_template_columns: vec![GridTrack::flex(1.0), GridTrack::flex(1.0)],
                padding: UiRect::horizontal(px(5.)),
                ..default()
            });

            grid.with_children(|grid| {
                for hinting in [FontHinting::Enabled, FontHinting::Disabled] {
                    let mut content = grid.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(5.),
                        ..default()
                    });

                    content.with_child((
                        Text::new(format!("FontHinting::{:?}", hinting)),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            ..default()
                        },
                        hinting,
                    ));

                    content.with_child((
                        Text::new("Font from css font list"),
                        TextFont {
                            font: FontSource::families(
                                "'Comic Sans', Arial, 'Noto Sans', sans-serif",
                            ),
                            ..Default::default()
                        },
                    ));

                    content.with_child((
                        Text::new("white "),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            ..default()
                        },
                        hinting,
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

                    content.with_child((
                        Text::new(""),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            ..default()
                        },
                        hinting,
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

                    content.with_child((
                        Text::new(""),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            ..default()
                        },
                        hinting,
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

                    content.with_child((
                        hinting,
                        Text::new("FiraSans_"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(25.),
                            ..default()
                        },
                        children![
                            (
                                TextSpan::new("MonaSans_"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/MonaSans-VariableFont.ttf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                }
                            ),
                            (
                                TextSpan::new("EBGaramond_"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/EBGaramond12-Regular.otf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                            (
                                TextSpan::new("FiraMono"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                        ],
                    ));

                    content.with_child((
                        hinting,
                        Text::new("FiraSans "),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(25.),
                            ..default()
                        },
                        children![
                            (
                                TextSpan::new("MonaSans "),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/MonaSans-VariableFont.ttf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                }
                            ),
                            (
                                TextSpan::new("EBGaramond "),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/EBGaramond12-Regular.otf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                            (
                                TextSpan::new("FiraMono"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                        ],
                    ));

                    content.with_child((
                        hinting,
                        Text::new("FiraSans "),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(25.),
                            ..default()
                        },
                        children![
                            (
                                TextSpan::new("MonaSans_"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/MonaSans-VariableFont.ttf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                }
                            ),
                            (
                                TextSpan::new("EBGaramond "),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/EBGaramond12-Regular.otf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                            (
                                TextSpan::new("FiraMono"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                        ],
                    ));

                    content.with_child((
                        hinting,
                        Text::new("FiraSans"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(25.),
                            ..default()
                        },
                        children![
                            TextSpan::new(" "),
                            (
                                TextSpan::new("MonaSans"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/MonaSans-VariableFont.ttf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                }
                            ),
                            TextSpan::new(" "),
                            (
                                TextSpan::new("EBGaramond"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/EBGaramond12-Regular.otf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                            TextSpan::new(" "),
                            (
                                TextSpan::new("FiraMono"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                        ],
                    ));

                    content.with_child((
                        hinting,
                        Text::new("Fira Sans_"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(25.),
                            ..default()
                        },
                        children![
                            (
                                TextSpan::new("Mona Sans_"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/MonaSans-VariableFont.ttf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                }
                            ),
                            (
                                TextSpan::new("EB Garamond_"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/EBGaramond12-Regular.otf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                            (
                                TextSpan::new("Fira Mono"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                        ],
                    ));

                    content.with_child((
                        hinting,
                        Text::new("FontWeight(100)_"),
                        TextFont {
                            font: "Mona Sans".into(),
                            font_size: FontSize::Px(25.),
                            weight: FontWeight(100),
                            ..default()
                        },
                        children![
                            (
                                TextSpan::new("FontWeight(500)_"),
                                TextFont {
                                    font: "Mona Sans".into(),
                                    font_size: FontSize::Px(25.),
                                    weight: FontWeight(500),
                                    ..default()
                                }
                            ),
                            (
                                TextSpan::new("FontWeight(900)"),
                                TextFont {
                                    font: "Mona Sans".into(),
                                    font_size: FontSize::Px(25.),
                                    weight: FontWeight(900),
                                    ..default()
                                },
                            ),
                        ],
                    ));

                    content.with_child((
                        hinting,
                        Text::new("FiraSans_"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(25.),
                            weight: FontWeight(900),
                            ..default()
                        },
                        children![
                            (
                                TextSpan::new("MonaSans_"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/MonaSans-VariableFont.ttf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    weight: FontWeight(700),
                                    ..default()
                                }
                            ),
                            (
                                TextSpan::new("EBGaramond_"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/EBGaramond12-Regular.otf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    weight: FontWeight(500),
                                    ..default()
                                },
                            ),
                            (
                                TextSpan::new("FiraMono"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                    font_size: FontSize::Px(25.),
                                    weight: FontWeight(300),
                                    ..default()
                                },
                            ),
                        ],
                    ));

                    content.with_child((
                        hinting,
                        Text::new("FiraSans\t"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(25.),
                            ..default()
                        },
                        children![
                            (
                                TextSpan::new("MonaSans\t"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/MonaSans-VariableFont.ttf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                }
                            ),
                            (
                                TextSpan::new("EBGaramond\t"),
                                TextFont {
                                    font: asset_server
                                        .load("fonts/EBGaramond12-Regular.otf")
                                        .into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                            (
                                TextSpan::new("FiraMono"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                    font_size: FontSize::Px(25.),
                                    ..default()
                                },
                            ),
                        ],
                    ));

                    for font_smoothing in [FontSmoothing::AntiAliased, FontSmoothing::None] {
                        content.with_child((
                            Text::new(format!("FontSmoothing::{:?}", font_smoothing)),
                            TextFont {
                                font: asset_server.load("fonts/MonaSans-VariableFont.ttf").into(),
                                font_size: FontSize::Px(25.),
                                font_smoothing,
                                ..default()
                            },
                        ));
                    }
                }
            });
        });
    }
}

mod font_lists {
    use bevy::prelude::*;

    const FONT_ASSETS: &[&str] = &[
        "fonts/FiraSans-Bold.ttf",
        "fonts/FiraMono-Medium.ttf",
        "fonts/MonaSans-VariableFont.ttf",
        "fonts/EBGaramond12-Regular.otf",
    ];

    const FONT_NAMES: &[&str] = &[
        "Gabriola",
        "Fira Sans",
        "Fira Mono",
        "Mona Sans",
        "EB Garamond",
    ];

    #[derive(Resource)]
    struct LoadedFontAssets {
        _handles: Vec<Handle<Font>>,
    }

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::FontLists)));
        commands.insert_resource(LoadedFontAssets {
            _handles: FONT_ASSETS
                .iter()
                .map(|font_asset| asset_server.load(*font_asset))
                .collect(),
        });
        commands.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                row_gap: px(25),
                ..default()
            },
            DespawnOnExit(super::Scene::FontLists),
            children![
                (
                    Text::new("Font Lists"),
                    TextFont::from_font_size(FontSize::Px(32.)),
                    Underline,
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(6),
                        ..default()
                    },
                    children![
                        Text::new("FontSource::Families"),
                        (
                            Node {
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                padding: px(16).left(),
                                column_gap: px(30),
                                row_gap: px(30),
                                ..default()
                            },
                            Children::spawn(SpawnIter(
                                (0..FONT_NAMES.len())
                                    .map(|start| {
                                        FONT_NAMES
                                            .iter()
                                            .copied()
                                            .cycle()
                                            .skip(start)
                                            .take(FONT_NAMES.len())
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    })
                                    .map(|list| {
                                        (
                                            Text::new(list.replace(", ", "\n")),
                                            TextFont {
                                                font: FontSource::families(list),
                                                font_size: FontSize::Px(16.),
                                                ..default()
                                            },
                                            Node {
                                                padding: px(4.).all(),
                                                ..default()
                                            },
                                            TextLayout::no_wrap(),
                                            Outline::default(),
                                        )
                                    }),
                            )),
                        )
                    ]
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(6),
                        ..default()
                    },
                    children![
                        Text::new("FontSource::List"),
                        (
                            Node {
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                padding: px(16).left(),
                                column_gap: px(30),
                                row_gap: px(30),
                                ..default()
                            },
                            Children::spawn(SpawnIter(
                                (0..FONT_NAMES.len())
                                    .map(|start| {
                                        FONT_NAMES
                                            .iter()
                                            .copied()
                                            .cycle()
                                            .skip(start)
                                            .take(FONT_NAMES.len())
                                            .collect::<Vec<_>>()
                                    })
                                    .map(|list| {
                                        (
                                            Text::new(list.join("\n")),
                                            TextFont {
                                                font: FontSource::list(list.iter().copied()),
                                                font_size: FontSize::Px(16.),
                                                ..default()
                                            },
                                            Node {
                                                padding: px(4.).all(),
                                                ..default()
                                            },
                                            TextLayout::no_wrap(),
                                            Outline::default(),
                                        )
                                    }),
                            )),
                        )
                    ]
                ),
            ],
        ));
    }
}
mod text_measurement {
    use bevy::prelude::*;

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::TextMeasurement)));

        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8),
                    padding: px(8).horizontal(),
                    ..default()
                },
                DespawnOnExit(super::Scene::TextMeasurement),
            ))
            .with_children(|parent| {
                let width = px(102);
                for (flex_direction, boxed) in [
                    (FlexDirection::Row, true),
                    (FlexDirection::Row, false),
                    (FlexDirection::Column, true),
                    (FlexDirection::Column, false),
                ] {
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            ..default()
                        })
                        .with_children(|parent| {
                            for align_items in [
                                AlignItems::Baseline,
                                AlignItems::Center,
                                AlignItems::Stretch,
                                AlignItems::FlexStart,
                                AlignItems::FlexEnd,
                            ] {
                                parent.spawn((
                                    Node {
                                        margin: px(8).top(),
                                        ..default()
                                    },
                                    Text::new(format!("AlignItems::{align_items:?}")),
                                    TextFont::from_font_size(10.),
                                ));
                                if boxed {
                                    parent.spawn((
                                        Node {
                                            align_items,
                                            border: px(2).all(),
                                            padding: px(2).all(),
                                            flex_direction,
                                            ..default()
                                        },
                                        BorderColor::all(Color::WHITE),
                                        children![
                                            (
                                                Node {
                                                    width: px(32),
                                                    height: px(32),
                                                    ..default()
                                                },
                                                BackgroundColor(Color::WHITE),
                                            ),
                                            (
                                                Node { width, ..default() },
                                                children![(
                                                    Text::new("+300 Some Long Item Title"),
                                                    TextFont {
                                                        font_size: FontSize::Px(10.),
                                                        ..default()
                                                    },
                                                    BackgroundColor(Color::srgba(
                                                        0.95, 0.85, 0.2, 0.35
                                                    )),
                                                )]
                                            )
                                        ],
                                    ));
                                } else {
                                    parent.spawn((
                                        Node {
                                            align_items,
                                            border: px(2).all(),
                                            padding: px(2).all(),
                                            flex_direction,
                                            ..default()
                                        },
                                        BorderColor::all(Color::WHITE),
                                        children![
                                            (
                                                Node {
                                                    width: px(32),
                                                    height: px(32),
                                                    ..default()
                                                },
                                                BackgroundColor(Color::WHITE),
                                            ),
                                            (
                                                Node { width, ..default() },
                                                Text::new("+300 Some Long Item Title"),
                                                TextFont {
                                                    font_size: FontSize::Px(10.),
                                                    ..default()
                                                },
                                                BackgroundColor(Color::srgba(
                                                    0.95, 0.85, 0.2, 0.35
                                                )),
                                            )
                                        ],
                                    ));
                                }
                            }
                        });
                }
            });
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

mod elliptical_border_radius {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands, assets: Res<AssetServer>) {
        commands.spawn((
            Camera2d,
            DespawnOnExit(super::Scene::EllipticalBorderRadius),
        ));
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(40),
                    row_gap: px(40),
                    margin: auto().all(),
                    padding: px(30).all(),
                    ..default()
                },
                BackgroundColor(DARK_GRAY.into()),
                DespawnOnExit(super::Scene::EllipticalBorderRadius),
            ))
            .with_children(|builder| {
                builder.spawn((
                    Node {
                        width: px(200),
                        height: px(100),
                        border: UiRect::all(px(8)),
                        border_radius: BorderRadius::elliptical(
                            Val2::new(px(90), px(24)),
                            Val2::new(px(18), px(70)),
                            Val2::new(px(110), px(32)),
                            Val2::new(px(28), px(58)),
                        ),
                        ..default()
                    },
                    BackgroundColor(ORANGE.into()),
                    BackgroundGradient::from(LinearGradient {
                        stops: vec![
                            RED.into(),
                            Color::BLACK.into(),
                            BLUE.into(),
                            WHEAT.into(),
                            GREEN.into(),
                        ],
                        ..default()
                    }),
                    BorderColor::all(RED),
                    Outline {
                        width: px(4),
                        offset: px(8),
                        color: WHITE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));

                builder.spawn((
                    Node {
                        width: px(150),
                        height: px(150),
                        border: UiRect {
                            left: px(16),
                            right: px(4),
                            top: px(24),
                            bottom: px(8),
                        },
                        border_radius: BorderRadius::elliptical(
                            Val2::new(percent(65), percent(20)),
                            Val2::new(percent(20), percent(65)),
                            Val2::new(percent(65), percent(20)),
                            Val2::new(percent(20), percent(65)),
                        ),
                        ..default()
                    },
                    BackgroundColor(MEDIUM_SEA_GREEN.into()),
                    BorderColor::all(DARK_GREEN),
                    Outline {
                        width: px(3),
                        offset: px(10),
                        color: LIME.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));

                builder.spawn((
                    Node {
                        width: px(210),
                        height: px(75),
                        border: UiRect::axes(px(12), px(4)),
                        border_radius: BorderRadius::elliptical(
                            Val2::new(px(140), px(18)),
                            Val2::new(px(140), px(18)),
                            Val2::new(px(42), px(54)),
                            Val2::new(px(42), px(54)),
                        ),
                        ..default()
                    },
                    BackgroundColor(DODGER_BLUE.into()),
                    BorderColor::all(NAVY),
                    Outline {
                        width: px(5),
                        offset: px(6),
                        color: SKY_BLUE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));
                builder.spawn((
                    Node {
                        width: px(160),
                        height: px(120),
                        border: UiRect::axes(px(20), px(20)),
                        border_radius: BorderRadius::elliptical(
                            Val2::new(px(50), px(10)),
                            Val2::new(px(50), px(10)),
                            Val2::new(px(50), px(10)),
                            Val2::new(px(50), px(10)),
                        ),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                    BorderColor::all(WHITE),
                    Outline {
                        width: px(3),
                        offset: px(5),
                        color: SKY_BLUE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));

                builder.spawn((
                    Node {
                        width: px(160),
                        height: px(120),
                        border: UiRect::axes(px(20), px(20)),
                        border_radius: BorderRadius::all(px(30)),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                    BorderColor::all(WHITE),
                    Outline {
                        width: px(3),
                        offset: px(5),
                        color: SKY_BLUE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));

                builder.spawn((
                    Node {
                        width: px(160),
                        height: px(120),
                        border: UiRect::axes(px(20), px(20)),
                        border_radius: BorderRadius::elliptical(
                            Val2::new(px(25), px(20)),
                            Val2::new(px(20), px(25)),
                            Val2::new(px(20), px(25)),
                            Val2::new(px(20), px(25)),
                        ),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                    ImageNode::from(assets.load("branding/icon.png")),
                    BorderColor::all(WHITE),
                    Outline {
                        width: px(3),
                        offset: px(5),
                        color: SKY_BLUE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));

                builder.spawn((
                    Node {
                        width: px(160),
                        height: px(120),
                        border: UiRect::axes(px(10), px(10)),
                        border_radius: BorderRadius::elliptical(
                            Val2::new(px(40), px(30)),
                            Val2::new(px(40), px(30)),
                            Val2::new(px(40), px(30)),
                            Val2::new(px(40), px(30)),
                        ),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                    BorderColor::all(WHITE),
                    Outline {
                        width: px(3),
                        offset: px(5),
                        color: SKY_BLUE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));

                builder.spawn((
                    Node {
                        width: px(160),
                        height: px(80),
                        border: UiRect::axes(px(10), px(10)),
                        border_radius: BorderRadius::elliptical(
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                        ),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                    BorderColor::all(WHITE),
                    ImageNode::from(assets.load("branding/icon.png")),
                    Outline {
                        width: px(3),
                        offset: px(5),
                        color: SKY_BLUE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));

                builder.spawn((
                    Node {
                        width: px(80),
                        height: px(160),
                        border: UiRect::axes(px(10), px(10)),
                        border_radius: BorderRadius::elliptical(
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                        ),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                    BorderColor::all(WHITE),
                    ImageNode::from(assets.load("branding/icon.png")),
                    Outline {
                        width: px(3),
                        offset: px(5),
                        color: SKY_BLUE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));

                builder.spawn((
                    Node {
                        width: px(160),
                        height: px(80),
                        border: UiRect::axes(px(20), px(10)),
                        border_radius: BorderRadius::elliptical(
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                        ),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                    BorderColor::all(WHITE),
                    ImageNode::from(assets.load("branding/icon.png")),
                    Outline {
                        width: px(3),
                        offset: px(5),
                        color: SKY_BLUE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));

                builder.spawn((
                    Node {
                        width: px(80),
                        height: px(160),
                        border: UiRect::all(px(10)).with_right(px(25)),
                        border_radius: BorderRadius::elliptical(
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                        ),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                    BorderColor::all(WHITE),
                    Outline {
                        width: px(3),
                        offset: px(5),
                        color: SKY_BLUE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));

                builder.spawn((
                    Node {
                        width: px(160),
                        height: px(80),
                        border: UiRect::all(px(5)),
                        border_radius: BorderRadius::elliptical(
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                            Val2::new(percent(50), percent(50)),
                            Val2::new(px(20), px(20)),
                        ),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                    BorderColor::all(WHITE),
                    ImageNode::from(assets.load("branding/icon.png")),
                    Outline {
                        width: px(3),
                        offset: px(5),
                        color: SKY_BLUE.into(),
                    },
                    BoxShadow::from(ShadowStyle {
                        blur_radius: px(5),
                        ..default()
                    }),
                ));
            });
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
                        BorderRadius::bottom_right(Val2::all(px(10))),
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
                        BorderRadius::bottom_right(Val2::all(px(10))),
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
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceAround,
                    align_content: AlignContent::Center,
                    ..default()
                },
                DespawnOnExit(super::Scene::Slice),
            ))
            .with_children(|parent| {
                for visual_box in [
                    VisualBox::BorderBox,
                    VisualBox::PaddingBox,
                    VisualBox::ContentBox,
                ] {
                    parent
                        .spawn(Node {
                            justify_content: JustifyContent::SpaceAround,
                            ..default()
                        })
                        .with_children(|parent| {
                            for [w, h] in [[200.0, 200.0], [300.0, 200.0], [150., 200.0]] {
                                parent.spawn((
                                    ImageNode {
                                        image: image.clone(),
                                        image_mode: NodeImageMode::Sliced(slicer.clone()),
                                        visual_box,
                                        ..default()
                                    },
                                    Node {
                                        width: px(w),
                                        height: px(h),
                                        border: px(20.).all(),
                                        padding: px(20.).all(),
                                        ..default()
                                    },
                                    Outline {
                                        width: px(2.),
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
                                    visual_box,
                                    ..Default::default()
                                },
                                Node {
                                    width: px(200),
                                    height: px(200),
                                    border: px(20.).all(),
                                    padding: px(20.).all(),
                                    ..default()
                                },
                                Outline {
                                    color: bevy::color::palettes::css::DARK_CYAN.into(),
                                    width: px(2.),
                                    ..default()
                                },
                                BackgroundColor(bevy::color::palettes::css::NAVY.into()),
                            ));
                        });
                }
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
    use bevy::ui::widget::Text;
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
                                            Text(format!("{color_space:?}")),
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
                                    border_radius: BorderRadius::bottom_right(Val2::all(px(25.))),
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
                                    border_radius: BorderRadius::bottom_right(Val2::all(px(25.))),
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

    pub fn setup(mut commands: Commands, mut debug_options: ResMut<GlobalUiDebugOptions>) {
        debug_options.enabled = true;
        debug_options.line_width = 5.;
        debug_options.line_color_override = Some(LinearRgba::GREEN);
        debug_options.show_hidden = true;
        debug_options.show_clipped = true;

        let debug_options: UiDebugOptions = (*debug_options.as_ref()).into();

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
                        ..debug_options
                    },
                ));

                // Vertical scrollbar (non-functional)
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: px(90),
                        height: px(230),
                        overflow: Overflow::scroll_y(),
                        scrollbar_width: 20.,
                        ..default()
                    },
                    ScrollPosition(Vec2::new(180., 180.)),
                    UiDebugOptions {
                        line_width: 3.,
                        outline_scrollbars: true,
                        show_hidden: false,
                        show_clipped: false,
                        ..debug_options
                    },
                    Children::spawn(SpawnIter((0..20).map(move |i| {
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
                        width: px(156),
                        height: px(70),
                        overflow: Overflow::scroll_x(),
                        scrollbar_width: 10.,
                        ..default()
                    },
                    UiDebugOptions {
                        line_width: 3.,
                        outline_scrollbars: true,
                        show_hidden: false,
                        show_clipped: false,
                        ..debug_options
                    },
                    Children::spawn(SpawnIter((0..20).map(move |i| {
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
                        width: px(230),
                        height: px(125),
                        overflow: Overflow::scroll(),
                        scrollbar_width: 20.,
                        ..default()
                    },
                    ScrollPosition(Vec2::new(300., 0.)),
                    UiDebugOptions {
                        line_width: 3.,
                        outline_scrollbars: true,
                        show_hidden: false,
                        show_clipped: false,
                        ..debug_options
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

    pub fn teardown(mut debug_options: ResMut<GlobalUiDebugOptions>) {
        *debug_options = GlobalUiDebugOptions::default();
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

mod outer_color {
    use bevy::prelude::*;

    pub fn setup(mut commands: Commands) {
        let radius = Val2::all(percent(33.));
        let width = px(10.);

        commands.spawn((Camera2d, DespawnOnExit(super::Scene::OuterColor)));
        commands
            .spawn((
                Node {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::px(3, 200.),
                    grid_template_rows: RepeatedGridTrack::px(3, 200.),
                    margin: UiRect::AUTO,
                    ..default()
                },
                DespawnOnExit(super::Scene::OuterColor),
            ))
            .with_children(|builder| {
                for (border, border_radius, invert) in [
                    (UiRect::ZERO, BorderRadius::bottom_right(radius), true),
                    (UiRect::top(width), BorderRadius::top(radius), false),
                    (UiRect::ZERO, BorderRadius::bottom_left(radius), true),
                    (UiRect::left(width), BorderRadius::left(radius), false),
                    (UiRect::all(width), BorderRadius::all(radius.x), true),
                    (UiRect::right(width), BorderRadius::right(radius), false),
                    (UiRect::ZERO, BorderRadius::top_right(radius), true),
                    (UiRect::bottom(width), BorderRadius::bottom(radius), false),
                    (UiRect::ZERO, BorderRadius::top_left(radius), true),
                ] {
                    builder
                        .spawn((
                            Node {
                                width: px(200.),
                                height: px(200.),
                                border_radius,
                                border,
                                ..default()
                            },
                            BorderColor::all(bevy::color::palettes::css::RED),
                        ))
                        .insert_if(BackgroundColor(Color::WHITE), || !invert)
                        .insert_if(OuterColor(Color::WHITE), || invert);
                }
            });
    }
}

mod boxed_content {
    use bevy::color::palettes::css::RED;
    use bevy::prelude::*;

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::BoxedContent)));
        commands
            .spawn((
                Node {
                    margin: auto().all(),
                    column_gap: px(30),
                    ..default()
                },
                DespawnOnExit(super::Scene::BoxedContent),
            ))
            .with_children(|builder| {
                for (heading, text_justify) in [
                    ("Left", Justify::Left),
                    ("Center", Justify::Center),
                    ("Right", Justify::Right),
                ] {
                    builder
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Start,
                            row_gap: px(20),
                            ..default()
                        })
                        .with_children(|builder| {
                            builder.spawn((
                                Node::default(),
                                Text::new(format!("{heading} justify")),
                                TextFont::from_font_size(FontSize::Px(14.)),
                                TextLayout::justify(Justify::Center),
                            ));

                            builder.spawn((
                                Node::default(),
                                Text::new("This text has\nno border or padding."),
                                TextFont::from_font_size(FontSize::Px(10.)),
                                TextLayout::justify(text_justify),
                                Outline {
                                    width: px(2),
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                            ));

                            builder.spawn((
                                Node {
                                    border: px(10).all(),
                                    ..default()
                                },
                                Text::new("This text has\na border but no padding."),
                                TextFont::from_font_size(FontSize::Px(10.)),
                                TextLayout::justify(text_justify),
                                BorderColor::all(RED),
                                Outline {
                                    width: px(2),
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                            ));

                            builder.spawn((
                                Node {
                                    padding: px(20).all(),
                                    ..default()
                                },
                                Text::new("This text has\npadding but no border."),
                                TextFont::from_font_size(FontSize::Px(10.)),
                                TextLayout::justify(text_justify),
                                Outline {
                                    width: px(2),
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                            ));

                            builder.spawn((
                                Node {
                                    border: px(10).all(),
                                    padding: px(20).all(),
                                    ..default()
                                },
                                Text::new("This text has\nborder and padding."),
                                TextFont::from_font_size(FontSize::Px(10.)),
                                TextLayout::justify(text_justify),
                                BorderColor::all(RED),
                                Outline {
                                    width: px(2),
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                            ));

                            builder.spawn((
                                Node {
                                    border: px(10).left(),
                                    ..default()
                                },
                                Text::new("This text has\na left border and no padding."),
                                TextFont::from_font_size(FontSize::Px(10.)),
                                TextLayout::justify(text_justify),
                                BorderColor::all(RED),
                                Outline {
                                    width: px(2),
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                            ));

                            builder.spawn((
                                Node {
                                    border: px(10).right(),
                                    ..default()
                                },
                                Text::new("This text has\na right border and no padding."),
                                TextFont::from_font_size(FontSize::Px(10.)),
                                TextLayout::justify(text_justify),
                                BorderColor::all(RED),
                                Outline {
                                    width: px(2),
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                            ));

                            builder.spawn((
                                Node {
                                    padding: px(20).top().with_right(px(20)),
                                    ..default()
                                },
                                Text::new("This text has\npadding on its top and right."),
                                TextFont::from_font_size(FontSize::Px(10.)),
                                TextLayout::justify(text_justify),
                                BorderColor::all(RED),
                                Outline {
                                    width: px(2),
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                            ));

                            builder.spawn((
                                Node {
                                    padding: px(20).bottom().with_left(px(20)),
                                    ..default()
                                },
                                Text::new("This text has\npadding on its bottom and left."),
                                TextFont::from_font_size(FontSize::Px(10.)),
                                TextLayout::justify(text_justify),
                                BorderColor::all(RED),
                                Outline {
                                    width: px(2),
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                            ));

                            builder.spawn((
                                Node {
                                    padding: px(20).top().with_left(px(20)),
                                    border: px(10).bottom().with_right(px(10)),
                                    ..default()
                                },
                                Text::new(
                                    "This text has\npadding on its top and left\nand a border on its bottom and right.",
                                ),
                                TextFont::from_font_size(FontSize::Px(10.)),
                                TextLayout::justify(text_justify),
                                BorderColor::all(RED),
                                Outline {
                                    width: px(2),
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                            ));
                        });
                }
            });
    }
}

mod editable_text {
    use bevy::color::palettes::css::YELLOW;
    use bevy::prelude::*;
    use bevy::text::EditableText;
    use bevy::text::TextCursorStyle;
    use bevy::text::TextEdit;

    const DUMMY_TEXT: &str = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten";
    const LOREM_TEXT: &str = concat!(
        "Lorem ipsum dolor sit amet, consectetuer adipiscing elit. ",
        "Aenean commodo ligula eget dolor. Aenean massa. ",
        "Cum sociis natoque penatibus et magnis dis parturient montes, nascetur reprehenderit mus. ",
        "Donec quam felis, ultricies nec, pellentesque eu, pretium quis, sem. ",
        "Nulla consequat massa quis enim. Donec pede justo, fringilla vel, aliquet nec, vulputate eget, arcu. ",
        "In enim justo, rhoncus ut, imperdiet a, venenatis vitae, justo. ", 
        "Nullam dictum felis eu pede mollis pretium. Integer tincidunt. ", 
        "Cras dapibus. Vivamus elementum semper nisi. Aenean vulputate eleifend tellus. ",
        "Aenean leo ligula, porttitor eu, consequat vitae, eleifend ac, enim. ",
        "Aliquam lorem ante, dapibus in, viverra quis, feugiat a, tellus. ",
        "Phasellus viverra nulla ut metus officia laoreet. Quisque rutrum. ",
        "Aenean imperdiet. Etiam ultricies nisi vel augue. Curabitur ullamcorper ultricies nisi.", 
        " Qui eget dui. Etiam rhoncus. Maecenas tempus, tellus eget condimentum rhoncus, ", 
        "sem quam semper libero, sit amet adipiscing sem neque sed ipsum. ",
        "Qui quam nunc, blandit vel, luctus pulvinar, hendrerit id, lorem. ",
        "Maecenas nec odio et ante tincidunt tempus. Donec vitae sapien ut libero venenatis faucibus. ",
        "Nullam quis ante. Etiam sit amet orci eget eros faucibus tincidunt. Duis leo. ",
        "Sed fringilla mauris sit amet nibh. Donec sodales sagittis magna. ",
        "Sed consequat, leo eget bibendum sodales, augue velit cursus nunc,"
    );

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::EditableText)));
        commands.spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Start,
                margin: px(10.).all(),
                width: vw(100),
                height: vh(100),
                row_gap: px(10),
                column_gap: px(20),
                ..default()
            },
            DespawnOnExit(super::Scene::EditableText),
            children![
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("Single line"),
                        (
                            EditableText {
                                pending_edits: vec![TextEdit::Insert(
                                    "Single line EditableText".into(),
                                )],
                                ..default()
                            },
                            TextLayout::no_wrap(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            Node {
                                width: px(200.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                        Text::new("Initial end"),
                        (
                            EditableText::new(LOREM_TEXT),
                            TextLayout::no_wrap(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            Node {
                                width: px(200.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                        Text::new("Insert end"),
                        (
                            EditableText {
                                pending_edits: vec![TextEdit::Insert(LOREM_TEXT.into())],
                                ..default()
                            },
                            TextLayout::no_wrap(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            Node {
                                width: px(200.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                        Text::new("Select line start"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(LOREM_TEXT.into()),
                                    TextEdit::LineStart(true),
                                ],
                                ..default()
                            },
                            TextLayout::no_wrap(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            Node {
                                width: px(200.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("Wrapped start"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(LOREM_TEXT.into()),
                                    TextEdit::TextStart(false),
                                ],
                                visible_lines: Some(8.),
                                ..default()
                            },
                            Node {
                                width: px(200.),
                                border: px(2).all(),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("Wrapped selection"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(LOREM_TEXT.into()),
                                    TextEdit::TextStart(false),
                                    TextEdit::Down(false),
                                    TextEdit::TextEnd(true),
                                ],
                                visible_lines: Some(8.),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            Node {
                                width: px(200.),
                                border: px(2).all(),
                                ..default()
                            },
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("Clamp top"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(DUMMY_TEXT.into()),
                                    TextEdit::ScrollByLines(-10.0),
                                ],
                                visible_lines: Some(5.5),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            Node {
                                width: px(100.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("Home, Scroll 1"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(DUMMY_TEXT.into()),
                                    TextEdit::ScrollTo(Vec2::ZERO),
                                    TextEdit::ScrollByLines(1.0),
                                ],
                                visible_lines: Some(5.5),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            Node {
                                width: px(100.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("Home, Scroll 2"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(DUMMY_TEXT.into()),
                                    TextEdit::ScrollTo(Vec2::ZERO),
                                    TextEdit::ScrollByLines(2.0),
                                ],
                                visible_lines: Some(5.5),
                                ..default()
                            },
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            Node {
                                width: px(100.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("Clamp bottom"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(DUMMY_TEXT.into()),
                                    TextEdit::ScrollByLines(-1000.0),
                                    TextEdit::ScrollByLines(1000.0),
                                ],
                                visible_lines: Some(5.5),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            Node {
                                width: px(100.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("Bottom -1"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(DUMMY_TEXT.into()),
                                    TextEdit::ScrollByLines(-1000.0),
                                    TextEdit::ScrollByLines(1000.0),
                                    TextEdit::ScrollByLines(-1.0),
                                ],
                                visible_lines: Some(5.5),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            Node {
                                width: px(100.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("Top +3"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(DUMMY_TEXT.into()),
                                    TextEdit::ScrollByLines(-1000.0),
                                    TextEdit::ScrollByLines(3.0),
                                ],
                                visible_lines: Some(5.5),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            Node {
                                width: px(100.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("Select down 3"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(DUMMY_TEXT.into()),
                                    TextEdit::TextStart(false),
                                    TextEdit::Down(false),
                                    TextEdit::Down(true),
                                    TextEdit::Down(true),
                                    TextEdit::Down(true),
                                ],
                                visible_lines: Some(5.5),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            Node {
                                width: px(100.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("End, Scroll 1"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(DUMMY_TEXT.into()),
                                    TextEdit::ScrollByLines(1.0),
                                ],
                                visible_lines: Some(5.5),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            Node {
                                width: px(100.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    children![
                        Text::new("End, Scroll -0.5"),
                        (
                            EditableText {
                                pending_edits: vec![
                                    TextEdit::Insert(DUMMY_TEXT.into()),
                                    TextEdit::ScrollByLines(-0.5),
                                ],
                                visible_lines: Some(5.5),
                                ..default()
                            },
                            TextCursorStyle::default(),
                            TextFont {
                                font_size: FontSize::Px(10.),
                                ..default()
                            },
                            Node {
                                width: px(100.),
                                border: px(2).all(),
                                ..default()
                            },
                            BorderColor::all(YELLOW),
                        ),
                    ],
                )
            ],
        ));
    }
}
