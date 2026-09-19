//! Uses two windows to test bunch of things for 2 cameras.

use bevy::color::palettes::css::{DARK_BLUE, DEEP_SKY_BLUE, LIGHT_SKY_BLUE, YELLOW};
use bevy::{
    prelude::*, reflect::TypePath, render::camera::RenderTarget, render::render_resource::*,
    window::WindowRef,
};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/custom_ui_material.wgsl";

fn main() {
    App::new()
        // By default, a primary window gets spawned by `WindowPlugin`, contained in `DefaultPlugins`
        .add_plugins(DefaultPlugins)
        .add_plugins(UiMaterialPlugin::<CustomUiMaterial>::default())
        .add_plugins(UiMaterialPlugin::<CustomUiMaterial2>::default())
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut ui_materials: ResMut<Assets<CustomUiMaterial>>,
    mut ui_materials2: ResMut<Assets<CustomUiMaterial2>>,
    asset_server: Res<AssetServer>,
) {
    let image = asset_server.load("textures/fantasy_ui_borders/panel-border-010.png");
    let slicer = TextureSlicer {
        border: BorderRect::all(22.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };

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

    // First window
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
                left: Val::ZERO,
                top: Val::ZERO,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                ..default()
            },
            UiTargetCamera(first_window_camera),
        ))
        .with_children(|parent| {
            let banner_scale_factor = 0.2;
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(100.0),
                    top: Val::Px(100.0),
                    width: Val::Px(905.0 * banner_scale_factor),
                    height: Val::Px(363.0 * banner_scale_factor),
                    border: UiRect::all(Val::Px(10.)),
                    ..default()
                },
                MaterialNode(ui_materials.add(CustomUiMaterial {
                    color: LinearRgba::BLUE.to_f32_array().into(),
                    slider: Vec4::splat(1.0),
                    color_texture: asset_server.load("branding/banner.png"),
                    border_color: LinearRgba::WHITE.to_f32_array().into(),
                })),
                BorderRadius::all(Val::Px(10.)),
                // UI material nodes can have outlines and shadows like any other UI node
                Outline {
                    width: Val::Px(2.),
                    offset: Val::Px(10.),
                    color: DARK_BLUE.into(),
                },
            ));
            parent
                .spawn((
                    Button,
                    ImageNode {
                        image: image.clone(),
                        image_mode: NodeImageMode::Sliced(slicer.clone()),
                        ..default()
                    },
                    Node {
                        left: Val::Px(100.0),
                        top: Val::Px(200.0),
                        width: Val::Px(150.0),
                        height: Val::Px(200.0),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        margin: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },
                ))
                .with_child((
                    Text::new("Button"),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 33.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));
        });

    // Second window
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

    commands
        .spawn((
            Node {
                left: Val::ZERO,
                top: Val::ZERO,
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                ..default()
            },
            UiTargetCamera(second_window_camera),
        ))
        .with_children(|parent| {
            let banner_scale_factor = 0.2;
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(1000.0),
                    top: Val::Px(500.0),
                    width: Val::Px(905.0 * banner_scale_factor),
                    height: Val::Px(363.0 * banner_scale_factor),
                    border: UiRect::all(Val::Px(10.)),
                    ..default()
                },
                MaterialNode(ui_materials2.add(CustomUiMaterial2 {
                    color: LinearRgba::RED.to_f32_array().into(),
                    slider: Vec4::splat(1.0),
                    color_texture: asset_server.load("branding/banner.png"),
                    border_color: LinearRgba::WHITE.to_f32_array().into(),
                })),
                BorderRadius::all(Val::Px(10.)),
                // UI material nodes can have outlines and shadows like any other UI node
                Outline {
                    width: Val::Px(2.),
                    offset: Val::Px(10.),
                    color: DEEP_SKY_BLUE.into(),
                },
            ));
            parent
                .spawn((
                    Button,
                    ImageNode {
                        image: image.clone(),
                        image_mode: NodeImageMode::Sliced(slicer.clone()),
                        ..default()
                    },
                    Node {
                        left: Val::Px(1000.0),
                        top: Val::Px(200.0),
                        width: Val::Px(150.0),
                        height: Val::Px(200.0),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        margin: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },
                ))
                .with_child((
                    Text::new("Button"),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 33.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));
        });
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct CustomUiMaterial {
    /// Color multiplied with the image
    #[uniform(0)]
    color: Vec4,
    /// Represents how much of the image is visible
    /// Goes from 0 to 1
    /// A `Vec4` is used here because Bevy with webgl2 requires that uniforms are 16-byte aligned but only the first component is read.
    #[uniform(1)]
    slider: Vec4,
    /// Image used to represent the slider
    #[texture(2)]
    #[sampler(3)]
    color_texture: Handle<Image>,
    /// Color of the image's border
    #[uniform(4)]
    border_color: Vec4,
}

impl UiMaterial for CustomUiMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct CustomUiMaterial2 {
    /// Color multiplied with the image
    #[uniform(0)]
    color: Vec4,
    /// Represents how much of the image is visible
    /// Goes from 0 to 1
    /// A `Vec4` is used here because Bevy with webgl2 requires that uniforms are 16-byte aligned but only the first component is read.
    #[uniform(1)]
    slider: Vec4,
    /// Image used to represent the slider
    #[texture(2)]
    #[sampler(3)]
    color_texture: Handle<Image>,
    /// Color of the image's border
    #[uniform(4)]
    border_color: Vec4,
}

impl UiMaterial for CustomUiMaterial2 {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
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
