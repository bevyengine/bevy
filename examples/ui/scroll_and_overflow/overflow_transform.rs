//! Demonstrates nested transformed UI clipping.

use bevy::color::palettes::css::LIGHT_BLUE;
use bevy::color::palettes::css::NAVY;
use bevy::math::ops::sin;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate_nodes, scale_inner))
        .run();
}

#[derive(Component)]
struct RotatingClipLayer {
    base_rotation: f32,
    speed: f32,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(rotating_node(
                    400.,
                    0.35,
                    0.18,
                    Color::srgb(0.12, 0.17, 0.22),
                ))
                .with_children(|parent| {
                    parent
                        .spawn(rotating_node(
                            350.,
                            -0.5,
                            -0.4,
                            Color::srgb(0.24, 0.18, 0.32),
                        ))
                        .with_children(|parent| {
                            parent
                                .spawn(rotating_node(
                                    300.,
                                    0.65,
                                    0.55,
                                    Color::srgb(0.15, 0.30, 0.25),
                                ))
                                .with_children(|parent| {
                                    parent.spawn((
                                        Node {
                                            flex_direction: FlexDirection::Column,
                                            position_type: PositionType::Absolute,
                                            margin: auto().all(),
                                            border_radius: BorderRadius::all(percent(20.)),
                                            align_items: AlignItems::Center,
                                            border: px(5.).all(),
                                            padding: px(5.).all(),
                                            ..default()
                                        },
                                        BoxShadow::new(
                                            Color::srgb(0.15, 0.30, 0.25).darker(0.05),
                                            px(30),
                                            px(30),
                                            px(0),
                                            px(4),
                                        ),
                                        InnerNode,
                                        BackgroundColor(NAVY.into()),
                                        BorderColor::all(LIGHT_BLUE),
                                        UiTransform::from_rotation(Rot2::degrees(45.)),
                                        children![
                                            (
                                                ImageNode::new(
                                                    asset_server
                                                        .load("branding/bevy_logo_dark_big.png"),
                                                ),
                                                Node {
                                                    width: px(400),
                                                    ..default()
                                                },
                                            ),
                                            (
                                                Text::new("transform + overflow"),
                                                TextFont {
                                                    font: asset_server
                                                        .load("fonts/FiraSans-Bold.ttf")
                                                        .into(),
                                                    font_size: FontSize::Px(34.),
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            )
                                        ],
                                    ));
                                });
                        });
                });
        });
}

fn rotate_nodes(time: Res<Time>, mut query: Query<(&RotatingClipLayer, &mut UiTransform)>) {
    for (layer, mut transform) in &mut query {
        transform.rotation = Rot2::radians(layer.base_rotation + time.elapsed_secs() * layer.speed);
    }
}

fn rotating_node(size: f32, rotation: f32, speed: f32, color: Color) -> impl Bundle {
    (
        Node {
            width: px(size),
            height: px(size),
            overflow: Overflow::clip(),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(4)),
            ..default()
        },
        BackgroundColor(color),
        BorderColor::all(Color::WHITE.with_alpha(0.7)),
        UiTransform::from_rotation(Rot2::radians(rotation)),
        RotatingClipLayer {
            base_rotation: rotation,
            speed,
        },
    )
}

#[derive(Component)]
struct InnerNode;

fn scale_inner(time: Res<Time>, mut query: Query<&mut UiTransform, With<InnerNode>>) {
    for mut transform in query.iter_mut() {
        transform.scale = Vec2::splat(1. + 0.75 * sin(0.4 * time.elapsed_secs()));
    }
}
