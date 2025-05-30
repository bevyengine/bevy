//! Uses two windows to visualize a 3D model from different angles.

use bevy::color::palettes::css::{DEEP_SKY_BLUE, LIGHT_SKY_BLUE, YELLOW};
use bevy::{prelude::*, render::camera::RenderTarget, window::WindowRef};

fn main() {
    App::new()
        // By default, a primary window gets spawned by `WindowPlugin`, contained in `DefaultPlugins`
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // add entities to the world
    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/torus/torus.gltf")),
    ));
    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let first_window_camera = commands
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();

    // Spawn a second window
    let second_window = commands
        .spawn(Window {
            title: "Second window".to_owned(),
            ..default()
        })
        .id();

    let second_window_camera = commands
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
            Camera {
                target: RenderTarget::Window(WindowRef::Entity(second_window)),
                ..default()
            },
        ))
        .id();

    let example_nodes = [
        (1., BorderRadius::all(Val::Px(20.))),
        (2., BorderRadius::MAX),
        (3., BorderRadius::all(Val::Px(20.))),
        (4., BorderRadius::MAX),
        (5., BorderRadius::all(Val::Px(20.))),
    ];

    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::RowReverse,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                width: Val::Percent(100.),
                height: Val::Percent(11.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
            // Since we are using multiple cameras, we need to specify which camera UI should be rendered to
            UiTargetCamera(first_window_camera),
        ))
        .with_children(|commands| {
            for (blur, border_radius) in example_nodes {
                commands.spawn(box_shadow_node_bundle(blur, border_radius));
            }
        });

    let first_font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::ZERO,
                left: Val::Px(200.),
                width: Val::Px(100.),
                ..default()
            },
            BackgroundColor(Color::from(DEEP_SKY_BLUE)),
            UiTargetCamera(first_window_camera),
        ))
        .with_children(|command| {
            command.spawn((
                Text::new("First window"),
                TextFont {
                    font: first_font_handle.clone(),
                    font_size: 50.0,
                    ..default()
                },
                TextColor(YELLOW.into()),
            ));
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::ZERO,
                left: Val::Px(500.),
                width: Val::Px(100.),
                overflow: Overflow::hidden(),
                ..default()
            },
            BackgroundColor(Color::from(DEEP_SKY_BLUE)),
            UiTargetCamera(first_window_camera),
        ))
        .with_children(|command| {
            command.spawn((
                Text::new("xxxxxxx"),
                TextFont {
                    font: first_font_handle.clone(),
                    font_size: 50.0,
                    ..default()
                },
                TextColor(YELLOW.into()),
            ));
        });

    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                width: Val::Percent(7.),
                height: Val::Percent(100.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
            // Since we are using multiple cameras, we need to specify which camera UI should be rendered to
            UiTargetCamera(second_window_camera),
        ))
        .with_children(|commands| {
            for (blur, border_radius) in example_nodes {
                commands.spawn(box_shadow_node_bundle(blur, border_radius));
            }
        });

    let second_font_handle = asset_server.load("fonts/FiraMono-Medium.ttf");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::ZERO,
                left: Val::Px(200.),
                width: Val::Px(100.),
                ..default()
            },
            BackgroundColor(Color::from(DEEP_SKY_BLUE)),
            UiTargetCamera(second_window_camera),
        ))
        .with_children(|command| {
            command.spawn((
                Text::new("Second window"),
                TextFont {
                    font: second_font_handle.clone(),
                    font_size: 50.0,
                    ..default()
                },
                TextColor(YELLOW.into()),
            ));
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::ZERO,
                left: Val::Px(500.),
                width: Val::Px(100.),
                overflow: Overflow::hidden(),
                ..default()
            },
            BackgroundColor(Color::from(DEEP_SKY_BLUE)),
            UiTargetCamera(second_window_camera),
        ))
        .with_children(|command| {
            command.spawn((
                Text::new("xxxxxxx"),
                TextFont {
                    font: second_font_handle.clone(),
                    font_size: 50.0,
                    ..default()
                },
                TextColor(YELLOW.into()),
            ));
        });
}

fn box_shadow_node_bundle(blur: f32, border_radius: BorderRadius) -> impl Bundle {
    (
        Node {
            width: Val::Px(50.),
            height: Val::Px(50.),
            border: UiRect::all(Val::Px(4.)),
            ..default()
        },
        BorderColor::all(LIGHT_SKY_BLUE.into()),
        border_radius,
        BackgroundColor(DEEP_SKY_BLUE.into()),
        BoxShadow::new(
            Color::BLACK.with_alpha(0.8),
            Val::Percent(10.),
            Val::Percent(10.),
            Val::Percent(10.),
            Val::Px(blur),
        ),
    )
}
