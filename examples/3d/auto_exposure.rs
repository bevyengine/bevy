//! This example showcases auto exposure,
//! which automatically (but not instantly) adjusts the brightness of the scene in a way that mimics the function of the human eye.
//! Auto exposure requires compute shader capabilities, so it's not available on WebGL.
//!
//! ## Controls
//!
//! | Key Binding        | Action                                 |
//! |:-------------------|:---------------------------------------|
//! | `Left` / `Right`   | Rotate Camera                          |
//! | `C`                | Toggle Compensation Curve              |
//! | `M`                | Toggle Metering Mask                   |
//! | `V`                | Visualize Metering Mask                |

use bevy::{
    core_pipeline::{
        auto_exposure::{AutoExposure, AutoExposureCompensationCurve, AutoExposurePlugin},
        Skybox,
    },
    math::{cubic_splines::LinearSpline, primitives::Plane3d, vec2},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AutoExposurePlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, example_control_system)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut compensation_curves: ResMut<Assets<AutoExposureCompensationCurve>>,
    asset_server: Res<AssetServer>,
) {
    let metering_mask = asset_server.load("textures/basic_metering_mask.png");

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        AutoExposure {
            metering_mask: metering_mask.clone(),
            ..default()
        },
        Skybox {
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            brightness: light_consts::lux::DIRECT_SUNLIGHT,
            ..default()
        },
    ));

    commands.insert_resource(ExampleResources {
        basic_compensation_curve: compensation_curves.add(
            AutoExposureCompensationCurve::from_curve(LinearSpline::new([
                vec2(-4.0, -2.0),
                vec2(0.0, 0.0),
                vec2(2.0, 0.0),
                vec2(4.0, 2.0),
            ]))
            .unwrap(),
        ),
        basic_metering_mask: metering_mask.clone(),
    });

    let plane = meshes.add(Mesh::from(
        Plane3d {
            normal: -Dir3::Z,
            half_size: Vec2::new(2.0, 0.5),
        }
        .mesh(),
    ));

    // Build a dimly lit box around the camera, with a slot to see the bright skybox.
    for level in -1..=1 {
        for side in [-Vec3::X, Vec3::X, -Vec3::Z, Vec3::Z] {
            if level == 0 && Vec3::Z == side {
                continue;
            }

            let height = Vec3::Y * level as f32;

            commands.spawn((
                Mesh3d(plane.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(
                        0.5 + side.x * 0.5,
                        0.75 - level as f32 * 0.25,
                        0.5 + side.z * 0.5,
                    ),
                    ..default()
                })),
                Transform::from_translation(side * 2.0 + height).looking_at(height, Vec3::Y),
            ));
        }
    }

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.0,
        ..default()
    });

    commands.spawn((
        PointLight {
            intensity: 2000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        ImageNode {
            image: metering_mask,
            ..default()
        },
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
    ));

    let text_font = TextFont::default();

    commands.spawn((Text::new("Left / Right - Rotate Camera\nC - Toggle Compensation Curve\nM - Toggle Metering Mask\nV - Visualize Metering Mask"),
            text_font.clone(), Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        })
    );

    commands.spawn((
        Text::default(),
        text_font,
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            right: px(12),
            ..default()
        },
        ExampleDisplay,
    ));
}

#[derive(Component)]
struct ExampleDisplay;

#[derive(Resource)]
struct ExampleResources {
    basic_compensation_curve: Handle<AutoExposureCompensationCurve>,
    basic_metering_mask: Handle<Image>,
}

fn example_control_system(
    camera: Single<(&mut Transform, &mut AutoExposure), With<Camera3d>>,
    mut display: Single<&mut Text, With<ExampleDisplay>>,
    mut mask_image: Single<&mut Node, With<ImageNode>>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    resources: Res<ExampleResources>,
) {
    let (mut camera_transform, mut auto_exposure) = camera.into_inner();

    let rotation = if input.pressed(KeyCode::ArrowLeft) {
        time.delta_secs()
    } else if input.pressed(KeyCode::ArrowRight) {
        -time.delta_secs()
    } else {
        0.0
    };

    camera_transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(rotation));

    if input.just_pressed(KeyCode::KeyC) {
        auto_exposure.compensation_curve =
            if auto_exposure.compensation_curve == resources.basic_compensation_curve {
                Handle::default()
            } else {
                resources.basic_compensation_curve.clone()
            };
    }

    if input.just_pressed(KeyCode::KeyM) {
        auto_exposure.metering_mask =
            if auto_exposure.metering_mask == resources.basic_metering_mask {
                Handle::default()
            } else {
                resources.basic_metering_mask.clone()
            };
    }

    mask_image.display = if input.pressed(KeyCode::KeyV) {
        Display::Flex
    } else {
        Display::None
    };

    display.0 = format!(
        "Compensation Curve: {}\nMetering Mask: {}",
        if auto_exposure.compensation_curve == resources.basic_compensation_curve {
            "Enabled"
        } else {
            "Disabled"
        },
        if auto_exposure.metering_mask == resources.basic_metering_mask {
            "Enabled"
        } else {
            "Disabled"
        },
    );
}
