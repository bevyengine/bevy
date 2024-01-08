//! Showcases wireframe rendering.
//!
//! Wireframes currently do not work when using webgl or webgpu.
//! Supported platforms:
//! - DX12
//! - Vulkan
//! - Metal
//!
//! This is a native only feature.

use bevy::{
    pbr::wireframe::{NoWireframe, Wireframe, WireframeColor, WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        render_resource::WgpuFeatures,
        settings::{RenderCreation, WgpuSettings},
        RenderPlugin,
    },
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    // WARN this is a native only feature. It will not work with webgl or webgpu
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
            }),
            // You need to add this plugin to enable wireframe rendering
            WireframePlugin,
        ))
        // Wireframes can be configured with this resource. This can be changed at runtime.
        .insert_resource(WireframeConfig {
            // The global wireframe config enables drawing of wireframes on every mesh,
            // except those with `NoWireframe`. Meshes with `Wireframe` will always have a wireframe,
            // regardless of the global configuration.
            global: true,
            // Controls the default color of all wireframes. Used as the default color for global wireframes.
            // Can be changed per mesh using the `WireframeColor` component.
            default_color: Color::WHITE,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, update_colors)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0)),
        material: materials.add(Color::BLUE),
        ..default()
    });

    // Red cube: Never renders a wireframe
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube { size: 1.0 }),
            material: materials.add(Color::RED),
            transform: Transform::from_xyz(-1.0, 0.5, -1.0),
            ..default()
        },
        NoWireframe,
    ));
    // Orange cube: Follows global wireframe setting
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Cube { size: 1.0 }),
        material: materials.add(Color::ORANGE),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // Green cube: Always renders a wireframe
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube { size: 1.0 }),
            material: materials.add(Color::GREEN),
            transform: Transform::from_xyz(1.0, 0.5, 1.0),
            ..default()
        },
        Wireframe,
        // This lets you configure the wireframe color of this entity.
        // If not set, this will use the color in `WireframeConfig`
        WireframeColor {
            color: Color::GREEN,
        },
    ));

    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Text used to show controls
    commands.spawn(
        TextBundle::from_section("", TextStyle::default()).with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
    );
}

/// This system let's you toggle various wireframe settings
fn update_colors(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<WireframeConfig>,
    mut wireframe_colors: Query<&mut WireframeColor>,
    mut text: Query<&mut Text>,
) {
    text.single_mut().sections[0].value = format!(
        "
Controls
---------------
Z - Toggle global
X - Change global color
C - Change color of the green cube wireframe

WireframeConfig
-------------
Global: {}
Color: {:?}
",
        config.global, config.default_color,
    );

    // Toggle showing a wireframe on all meshes
    if keyboard_input.just_pressed(KeyCode::KeyZ) {
        config.global = !config.global;
    }

    // Toggle the global wireframe color
    if keyboard_input.just_pressed(KeyCode::KeyX) {
        config.default_color = if config.default_color == Color::WHITE {
            Color::PINK
        } else {
            Color::WHITE
        };
    }

    // Toggle the color of a wireframe using WireframeColor and not the global color
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        for mut color in &mut wireframe_colors {
            color.color = if color.color == Color::GREEN {
                Color::RED
            } else {
                Color::GREEN
            };
        }
    }
}
