//! UI testbed
//!
//! You can switch scene by pressing the spacebar

mod helpers;

use bevy::prelude::*;
use helpers::Next;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins,))
        .init_state::<Scene>()
        .add_systems(OnEnter(Scene::Image), image::setup)
        .add_systems(OnEnter(Scene::Text), text::setup)
        .add_systems(OnEnter(Scene::Grid), grid::setup)
        .add_systems(OnEnter(Scene::Borders), borders::setup)
        .add_systems(OnEnter(Scene::BoxShadow), box_shadow::setup)
        .add_systems(OnEnter(Scene::TextWrap), text_wrap::setup)
        .add_systems(OnEnter(Scene::Overflow), overflow::setup)
        .add_systems(OnEnter(Scene::Slice), slice::setup)
        .add_systems(OnEnter(Scene::LayoutRounding), layout_rounding::setup)
        .add_systems(Update, switch_scene);

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
            Scene::LayoutRounding => Scene::Image,
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
        commands.spawn((Camera2d, StateScoped(super::Scene::Image)));
        commands.spawn((
            ImageNode::new(asset_server.load("branding/bevy_logo_dark.png")),
            StateScoped(super::Scene::Image),
        ));
    }
}

mod text {
    use bevy::prelude::*;

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, StateScoped(super::Scene::Text)));
        commands.spawn((
            Text::new("Hello World."),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 200.,
                ..default()
            },
            StateScoped(super::Scene::Text),
        ));
    }
}

mod grid {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, StateScoped(super::Scene::Grid)));
        // Top-level grid (app frame)
        commands
            .spawn((
                Node {
                    display: Display::Grid,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    grid_template_columns: vec![GridTrack::min_content(), GridTrack::flex(1.0)],
                    grid_template_rows: vec![
                        GridTrack::auto(),
                        GridTrack::flex(1.0),
                        GridTrack::px(40.),
                    ],
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                StateScoped(super::Scene::Grid),
            ))
            .with_children(|builder| {
                // Header
                builder.spawn((
                    Node {
                        display: Display::Grid,
                        grid_column: GridPlacement::span(2),
                        padding: UiRect::all(Val::Px(40.0)),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                ));

                // Main content grid (auto placed in row 2, column 1)
                builder
                    .spawn((
                        Node {
                            height: Val::Percent(100.0),
                            aspect_ratio: Some(1.0),
                            display: Display::Grid,
                            grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
                            grid_template_rows: RepeatedGridTrack::flex(2, 1.0),
                            row_gap: Val::Px(12.0),
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
                    ))
                    .with_children(|builder| {
                        builder.spawn((Node::default(), BackgroundColor(ORANGE.into())));
                        builder.spawn((Node::default(), BackgroundColor(BISQUE.into())));
                        builder.spawn((Node::default(), BackgroundColor(BLUE.into())));
                        builder.spawn((Node::default(), BackgroundColor(CRIMSON.into())));
                        builder.spawn((Node::default(), BackgroundColor(AQUA.into())));
                    });

                // Right side bar (auto placed in row 2, column 2)
                builder.spawn((Node::DEFAULT, BackgroundColor(BLACK.into())));
            });
    }
}

mod borders {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, StateScoped(super::Scene::Borders)));
        let root = commands
            .spawn((
                Node {
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                StateScoped(super::Scene::Borders),
            ))
            .id();

        // all the different combinations of border edges
        let borders = [
            UiRect::default(),
            UiRect::all(Val::Px(20.)),
            UiRect::left(Val::Px(20.)),
            UiRect::vertical(Val::Px(20.)),
            UiRect {
                left: Val::Px(40.),
                top: Val::Px(20.),
                ..Default::default()
            },
            UiRect {
                right: Val::Px(20.),
                bottom: Val::Px(30.),
                ..Default::default()
            },
            UiRect {
                right: Val::Px(20.),
                top: Val::Px(40.),
                bottom: Val::Px(20.),
                ..Default::default()
            },
            UiRect {
                left: Val::Px(20.),
                top: Val::Px(20.),
                bottom: Val::Px(20.),
                ..Default::default()
            },
            UiRect {
                left: Val::Px(20.),
                right: Val::Px(20.),
                bottom: Val::Px(40.),
                ..Default::default()
            },
        ];

        let non_zero = |x, y| x != Val::Px(0.) && y != Val::Px(0.);
        let border_size = |x, y| if non_zero(x, y) { f32::MAX } else { 0. };

        for border in borders {
            for rounded in [true, false] {
                let border_node = commands
                    .spawn((
                        Node {
                            width: Val::Px(100.),
                            height: Val::Px(100.),
                            border,
                            margin: UiRect::all(Val::Px(30.)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(MAROON.into()),
                        BorderColor(RED.into()),
                        Outline {
                            width: Val::Px(10.),
                            offset: Val::Px(10.),
                            color: Color::WHITE,
                        },
                    ))
                    .id();

                if rounded {
                    let border_radius = BorderRadius::px(
                        border_size(border.left, border.top),
                        border_size(border.right, border.top),
                        border_size(border.right, border.bottom),
                        border_size(border.left, border.bottom),
                    );
                    commands.entity(border_node).insert(border_radius);
                }

                commands.entity(root).add_child(border_node);
            }
        }
    }
}

mod box_shadow {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, StateScoped(super::Scene::BoxShadow)));

        commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(30.)),
                    column_gap: Val::Px(200.),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                BackgroundColor(GREEN.into()),
                StateScoped(super::Scene::BoxShadow),
            ))
            .with_children(|commands| {
                let example_nodes = [
                    (
                        Vec2::splat(100.),
                        Vec2::ZERO,
                        10.,
                        0.,
                        BorderRadius::bottom_right(Val::Px(10.)),
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
                        BorderRadius::bottom_right(Val::Px(10.)),
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
                            width: Val::Px(size.x),
                            height: Val::Px(size.y),
                            border: UiRect::all(Val::Px(2.)),
                            ..default()
                        },
                        BorderColor(WHITE.into()),
                        border_radius,
                        BackgroundColor(BLUE.into()),
                        BoxShadow::new(
                            Color::BLACK.with_alpha(0.9),
                            Val::Percent(offset.x),
                            Val::Percent(offset.y),
                            Val::Percent(spread),
                            Val::Px(blur),
                        ),
                    ));
                }
            });
    }
}

mod text_wrap {
    use bevy::prelude::*;

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, StateScoped(super::Scene::TextWrap)));

        let root = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Px(200.),
                    height: Val::Percent(100.),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                StateScoped(super::Scene::TextWrap),
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
                    TextLayout::new(JustifyText::Left, linebreak),
                    BackgroundColor(Color::srgb(0.8 - j as f32 * 0.3, 0., 0.)),
                ));
            }
        }
    }
}

mod overflow {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, StateScoped(super::Scene::Overflow)));
        let image = asset_server.load("branding/icon.png");

        commands
            .spawn((
                Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceAround,
                    ..Default::default()
                },
                BackgroundColor(BLUE.into()),
                StateScoped(super::Scene::Overflow),
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
                                width: Val::Px(100.),
                                height: Val::Px(100.),
                                padding: UiRect {
                                    left: Val::Px(25.),
                                    top: Val::Px(25.),
                                    ..Default::default()
                                },
                                border: UiRect::all(Val::Px(5.)),
                                overflow,
                                ..default()
                            },
                            BorderColor(RED.into()),
                            BackgroundColor(Color::WHITE),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                ImageNode::new(image.clone()),
                                Node {
                                    min_width: Val::Px(100.),
                                    min_height: Val::Px(100.),
                                    ..default()
                                },
                                Interaction::default(),
                                Outline {
                                    width: Val::Px(2.),
                                    offset: Val::Px(2.),
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
        commands.spawn((Camera2d, StateScoped(super::Scene::Slice)));
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
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceAround,
                    ..default()
                },
                StateScoped(super::Scene::Slice),
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
                            width: Val::Px(w),
                            height: Val::Px(h),
                            ..default()
                        },
                    ));
                }
            });
    }
}

mod layout_rounding {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, StateScoped(super::Scene::LayoutRounding)));

        commands
            .spawn((
                Node {
                    display: Display::Grid,
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    grid_template_rows: vec![RepeatedGridTrack::fr(10, 1.)],
                    ..Default::default()
                },
                BackgroundColor(Color::WHITE),
                StateScoped(super::Scene::LayoutRounding),
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
                                        border: UiRect::all(Val::Px(5.)),
                                        ..Default::default()
                                    },
                                    BackgroundColor(MAROON.into()),
                                    BorderColor(DARK_BLUE.into()),
                                ));
                            }
                        });
                }
            });
    }
}
