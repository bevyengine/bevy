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
    color::palettes::css::*,
    pbr::wireframe::{
        NoWireframe, Wireframe, WireframeColor, WireframeConfig, WireframeLineWidth,
        WireframePlugin, WireframeTopology,
    },
    prelude::*,
    render::{render_resource::WgpuFeatures, settings::WgpuSettings, RenderPlugin},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: WgpuSettings {
                    // WARN this is a native only feature. It will not work with webgl or webgpu
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }
                .into(),
                ..default()
            }),
            // You need to add this plugin to enable wireframe rendering
            WireframePlugin::default(),
        ))
        // Wireframes can be configured with this resource. This can be changed at runtime.
        .insert_resource(WireframeConfig {
            // The global wireframe config enables drawing of wireframes on every mesh,
            // except those with `NoWireframe`. Meshes with `Wireframe` will always have a wireframe,
            // regardless of the global configuration.
            global: true,
            // Controls the default color of all wireframes. Used as the default color for global wireframes.
            // Can be changed per mesh using the `WireframeColor` component.
            default_color: WHITE.into(),
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, update_colors)
        .run();
}

#[derive(Component)]
struct ColorToggleCube;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Red cube: Never renders a wireframe
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::from(RED))),
        Transform::from_xyz(-1.5, 0.5, -1.5),
        NoWireframe,
    ));
    // Orange cube: Follows global wireframe setting
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::from(ORANGE))),
        Transform::from_xyz(-0.5, 0.5, -0.5),
    ));
    // Green cube: Always renders a wireframe with custom color
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::from(LIME))),
        Transform::from_xyz(0.5, 0.5, 0.5),
        Wireframe,
        // This lets you configure the wireframe color of this entity.
        // If not set, this will use the color in `WireframeConfig`
        WireframeColor { color: LIME.into() },
        ColorToggleCube,
    ));

    // Purple cube: wireframe with explicit Quads topology override
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::from(PURPLE))),
        Transform::from_xyz(1.5, 0.5, 1.5),
        Wireframe,
        WireframeColor {
            color: YELLOW.into(),
        },
        WireframeLineWidth { width: 3.0 },
        WireframeTopology::Quads,
    ));

    // plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::from(BLUE))),
        // You can insert this component without the `Wireframe` component
        // to override the color of the global wireframe for this mesh
        WireframeColor {
            color: BLACK.into(),
        },
    ));

    // light
    commands.spawn((PointLight::default(), Transform::from_xyz(2.0, 4.0, 2.0)));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Text used to show controls
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

/// This system let's you toggle various wireframe settings
fn update_colors(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<WireframeConfig>,
    mut wireframe_colors: Query<&mut WireframeColor, With<ColorToggleCube>>,
    mut wireframe_widths: Query<&mut WireframeLineWidth>,
    mut text: Single<&mut Text>,
) {
    let current_width = wireframe_widths
        .iter()
        .next()
        .map(|w| w.width)
        .unwrap_or(1.0);

    text.0 = format!(
        "Controls
---------------
Z - Toggle global
X - Change global color
C - Change color of the green cube wireframe
V - Line width (current: {current_width:.1}px)
B - Toggle topology (current: {:?})

WireframeConfig
-------------
Global: {}
Color: {:?}",
        config.default_topology, config.global, config.default_color,
    );

    // Toggle showing a wireframe on all meshes
    if keyboard_input.just_pressed(KeyCode::KeyZ) {
        config.global = !config.global;
    }

    // Toggle the global wireframe color
    if keyboard_input.just_pressed(KeyCode::KeyX) {
        config.default_color = if config.default_color == WHITE.into() {
            DEEP_PINK.into()
        } else {
            WHITE.into()
        };
    }

    // Toggle the color of a wireframe using WireframeColor and not the global color
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        for mut color in &mut wireframe_colors {
            color.color = if color.color == LIME.into() {
                RED.into()
            } else {
                LIME.into()
            };
        }
    }

    if keyboard_input.just_pressed(KeyCode::KeyV) {
        for mut width in &mut wireframe_widths {
            width.width = match width.width as u32 {
                0..=2 => 3.0,
                3..=4 => 5.0,
                5..=7 => 10.0,
                _ => 2.0,
            };
        }
    }

    if keyboard_input.just_pressed(KeyCode::KeyB) {
        config.default_topology = match config.default_topology {
            WireframeTopology::Triangles => WireframeTopology::Quads,
            WireframeTopology::Quads => WireframeTopology::Triangles,
        };
    }
}
