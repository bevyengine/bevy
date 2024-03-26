//! This example showcases auto exposure
//! Auto exposure requires compute shader capabilities, so it's not available on WebGL.
//!
//! ## Controls
//!
//! | Key Binding        | Action                                 |
//! |:-------------------|:---------------------------------------|
//! | `Left` / `Right`   | Rotate Camera                          |
//! | `E`                | Cycle Environment Maps                 |
//! | `C`                | Toggle Compensation Curve              |
//! | `M`                | Toggle Metering Mask                   |

use bevy::{
    core_pipeline::{
        auto_exposure::{AutoExposureCompensationCurve, AutoExposurePlugin, AutoExposureSettings},
        Skybox,
    },
    math::{cubic_splines::LinearSpline, vec2},
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
    // let diffuse = asset_server.load("environment_maps/ennis_diffuse.ktx2");
    // let specular = asset_server.load("environment_maps/ennis_specular.ktx2");

    let diffuse = asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2");
    let specular = asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2");

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 6.0),
            ..Default::default()
        },
        AutoExposureSettings::default(),
        Skybox {
            image: specular.clone(),
            brightness: 5000.0,
        },
        EnvironmentMapLight {
            diffuse_map: diffuse,
            specular_map: specular,
            intensity: 5000.0,
        },
    ));

    commands.insert_resource(ExampleResources {
        basic_compensation_curve: compensation_curves.add(
            LinearSpline::new([
                vec2(-4.0, -2.0),
                vec2(0.0, 0.0),
                vec2(2.0, 0.0),
                vec2(4.0, 2.0),
            ])
            .to_curve(),
        ),
        basic_metering_mask: asset_server.load("textures/basic_metering_mask.png"),
    });

    let ball = meshes.add(Sphere::default());

    commands.spawn(PbrBundle {
        mesh: ball.clone(),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.1, 1.0),
            ..default()
        }),
        transform: Transform::from_xyz(1.0, 0.0, 0.0),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: ball.clone(),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.1, 1.0),
            ..default()
        }),
        transform: Transform::from_xyz(-1.0, 0.0, 0.0),
        ..default()
    });

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 18.0,
        ..default()
    };

    commands.spawn(
        TextBundle::from_section(
            "Left / Right — Rotate Camera\nL — Cycle Environment Maps\nC — Toggle Compensation Curve\nM — Toggle Metering Mask",
            text_style.clone(),
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
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
