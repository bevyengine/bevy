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
        auto_exposure::{AutoExposureCompensationCurve, AutoExposurePlugin, AutoExposureSettings},
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
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(1.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        AutoExposureSettings {
            metering_mask: metering_mask.clone(),
            ..default()
        },
        Skybox {
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            brightness: bevy::pbr::light_consts::lux::DIRECT_SUNLIGHT,
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

            commands.spawn(PbrBundle {
                mesh: plane.clone(),
                material: materials.add(StandardMaterial {
                    base_color: Color::srgb(
                        0.5 + side.x * 0.5,
                        0.75 - level as f32 * 0.25,
                        0.5 + side.z * 0.5,
                    ),
                    ..default()
                }),
                transform: Transform::from_translation(side * 2.0 + height)
                    .looking_at(height, Vec3::Y),
                ..default()
            });
        }
    }

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.0,
    });

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 2000.0,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });

    commands.spawn(ImageBundle {
        image: UiImage {
            texture: metering_mask,
            ..default()
        },
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        ..default()
    });

    let text_style = TextStyle::default();

    commands.spawn(
        TextBundle::from_section(
            "Left / Right - Rotate Camera\nC - Toggle Compensation Curve\nM - Toggle Metering Mask\nV - Visualize Metering Mask",
            text_style.clone(),
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );

    commands.spawn((
        TextBundle::from_section("", text_style).with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        }),
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
    mut camera: Query<(&mut Transform, &mut AutoExposureSettings), With<Camera3d>>,
    mut display: Query<&mut Text, With<ExampleDisplay>>,
    mut mask_image: Query<&mut Style, With<UiImage>>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    resources: Res<ExampleResources>,
) {
    let (mut camera_transform, mut auto_exposure) = camera.single_mut();

    let rotation = if input.pressed(KeyCode::ArrowLeft) {
        time.delta_seconds()
    } else if input.pressed(KeyCode::ArrowRight) {
        -time.delta_seconds()
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

    mask_image.single_mut().display = if input.pressed(KeyCode::KeyV) {
        Display::Flex
    } else {
        Display::None
    };

    let mut display = display.single_mut();
    display.sections[0].value = format!(
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
