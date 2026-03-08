//! Demonstrates how render layers can control which lights affect which meshes.
//!
//! ## Controls
//!
//! | Key Binding | Action                                 |
//! |:------------|:---------------------------------------|
//! | `1`         | Toggle directional light on layer `1`  |
//! | `2`         | Toggle directional light on layer `0`  |
//! | `3`         | Toggle point light on layer `1`        |
//! | `4`         | Toggle point light on layer `0`        |

use bevy::{camera::visibility::RenderLayers, prelude::*};

const DIRECTIONAL_LAYER_1_ILLUMINANCE: f32 = 10_000.0;
const DIRECTIONAL_LAYER_0_ILLUMINANCE: f32 = 20_000.0;
const POINT_LAYER_1_INTENSITY: f32 = 3_500_000.0;
const POINT_LAYER_0_INTENSITY: f32 = 4_500_000.0;
const POINT_LIGHT_RANGE: f32 = 45.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GlobalAmbientLight {
            brightness: 0.0,
            ..default()
        })
        .init_resource::<LightStates>()
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_lights, update_help_text))
        .run();
}

#[derive(Resource)]
struct LightStates {
    directional_layer_1: bool,
    directional_layer_0: bool,
    point_layer_1: bool,
    point_layer_0: bool,
}

impl Default for LightStates {
    fn default() -> Self {
        Self {
            directional_layer_1: true,
            directional_layer_0: true,
            point_layer_1: true,
            point_layer_0: true,
        }
    }
}

#[derive(Component)]
struct DirectionalLayer1Light;

#[derive(Component)]
struct DirectionalLayer0Light;

#[derive(Component)]
struct PointLayer1Light;

#[derive(Component)]
struct PointLayer0Light;

#[derive(Component)]
struct ExampleText;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::from_layers(&[0, 1]),
    ));

    // Ground plane for extra lighting context.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(14.0, 14.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.08, 0.08),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    // Object 1: on layers 0 and 1.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        Transform::from_xyz(-2.5, 1.0, 0.0),
        RenderLayers::from_layers(&[0, 1]),
    ));

    // Object 2: only on layer 0.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.2, 0.8))),
        Transform::from_xyz(2.5, 1.0, 0.0),
        RenderLayers::layer(0),
    ));

    // Directional light A: layer 1 only.
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.82, 0.75),
            illuminance: DIRECTIONAL_LAYER_1_ILLUMINANCE,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, -1.0, 0.0)),
        RenderLayers::layer(1),
        DirectionalLayer1Light,
    ));

    // Directional light B: layer 0 only.
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.75, 0.8, 1.0),
            illuminance: DIRECTIONAL_LAYER_0_ILLUMINANCE,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.6, 0.8, 0.0)),
        RenderLayers::layer(0),
        DirectionalLayer0Light,
    ));

    // Point light C: layer 1 only.
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.45, 0.1),
            intensity: POINT_LAYER_1_INTENSITY,
            range: POINT_LIGHT_RANGE,
            ..default()
        },
        Transform::from_xyz(-4.0, 3.0, 3.0),
        RenderLayers::layer(1),
        PointLayer1Light,
    ));

    // Point light D: layer 0 only.
    commands.spawn((
        PointLight {
            color: Color::srgb(0.1, 0.55, 1.0),
            intensity: POINT_LAYER_0_INTENSITY,
            range: POINT_LIGHT_RANGE,
            ..default()
        },
        Transform::from_xyz(4.0, 3.0, 3.0),
        RenderLayers::layer(0),
        PointLayer0Light,
    ));

    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            padding: UiRect::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(0.7)),
        ExampleText,
    ));
}

fn toggle_lights(
    key_input: Res<ButtonInput<KeyCode>>,
    mut light_states: ResMut<LightStates>,
    mut directional_layer_1: Single<
        &mut DirectionalLight,
        (With<DirectionalLayer1Light>, Without<DirectionalLayer0Light>),
    >,
    mut directional_layer_0: Single<
        &mut DirectionalLight,
        (With<DirectionalLayer0Light>, Without<DirectionalLayer1Light>),
    >,
    mut point_layer_1:
        Single<&mut PointLight, (With<PointLayer1Light>, Without<PointLayer0Light>)>,
    mut point_layer_0:
        Single<&mut PointLight, (With<PointLayer0Light>, Without<PointLayer1Light>)>,
) {
    if key_input.just_pressed(KeyCode::Digit1) {
        light_states.directional_layer_1 = !light_states.directional_layer_1;
    }
    if key_input.just_pressed(KeyCode::Digit2) {
        light_states.directional_layer_0 = !light_states.directional_layer_0;
    }
    if key_input.just_pressed(KeyCode::Digit3) {
        light_states.point_layer_1 = !light_states.point_layer_1;
    }
    if key_input.just_pressed(KeyCode::Digit4) {
        light_states.point_layer_0 = !light_states.point_layer_0;
    }

    directional_layer_1.illuminance = if light_states.directional_layer_1 {
        DIRECTIONAL_LAYER_1_ILLUMINANCE
    } else {
        0.0
    };
    directional_layer_0.illuminance = if light_states.directional_layer_0 {
        DIRECTIONAL_LAYER_0_ILLUMINANCE
    } else {
        0.0
    };
    point_layer_1.intensity = if light_states.point_layer_1 {
        POINT_LAYER_1_INTENSITY
    } else {
        0.0
    };
    point_layer_0.intensity = if light_states.point_layer_0 {
        POINT_LAYER_0_INTENSITY
    } else {
        0.0
    };
}

fn update_help_text(mut text: Single<&mut Text, With<ExampleText>>, light_states: Res<LightStates>) {
    fn status(enabled: bool) -> &'static str {
        if enabled {
            "ON"
        } else {
            "OFF"
        }
    }

    text.clear();
    text.push_str(&format!(
        "Light Render Layers\n\
Left cube: layers [0, 1]\n\
Right cube: layer [0]\n\
\n\
1 - Directional light on layer 1 [{}]\n\
2 - Directional light on layer 0 [{}]\n\
3 - Point light on layer 1 [{}]\n\
4 - Point light on layer 0 [{}]",
        status(light_states.directional_layer_1),
        status(light_states.directional_layer_0),
        status(light_states.point_layer_1),
        status(light_states.point_layer_0),
    ));
}
