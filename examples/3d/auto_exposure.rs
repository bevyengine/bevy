//! This example showcases auto exposure
//!
//! ## Controls
//!
//! | Key Binding        | Action                                 |
//! |:-------------------|:---------------------------------------|
//! | `Left` / `Right`   | Rotate Camera                          |
//! | `L`                | Toggle Light Source Illuminance        |

use bevy::{
    core_pipeline::auto_exposure::{AutoExposurePlugin, AutoExposureSettings},
    math::vec2,
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
    asset_server: Res<AssetServer>,
) {
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

    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(10.0, 10.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.3, 0.1),
            ..default()
        }),
        transform: Transform::from_xyz(0.0, -1.0, 0.0),
        ..default()
    });

    let wall_mesh = meshes.add(Plane3d { normal: -Dir3::Z }.mesh().size(10.0, 10.0));
    for (x, z, base_color) in [
        (-1, 0, Color::WHITE),
        (0, -1, Color::srgb(1.0, 0.0, 0.0)),
        (1, 0, Color::srgb(0.0, 1.0, 0.0)),
        (0, 1, Color::srgb(0.05, 0.05, 0.05)),
    ] {
        let x = x as f32 * 5.0;
        let z = z as f32 * 5.0;
        commands.spawn(PbrBundle {
            mesh: wall_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color,
                double_sided: true,
                ..default()
            }),
            transform: Transform::from_xyz(x, 4.0, z).looking_at(Vec3::new(0.0, 4.0, 0.0), Vec3::Y),
            ..default()
        });
    }

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-8.0, 10.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 6.0),
            ..Default::default()
        },
        AutoExposureSettings {
            min: -16.0,
            max: 16.0,
            compensation_curve: vec![vec2(-8.0, -2.0), vec2(0.0, 0.0), vec2(8.0, 2.0)],
            ..default()
        },
    ));

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 18.0,
        ..default()
    };

    commands.spawn(
        TextBundle::from_section(
            "Left / Right — Rotate Camera\nL — Toggle Light Source Illuminance",
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

fn example_control_system(
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut light: Query<&mut DirectionalLight>,
    mut display: Query<&mut Text, With<ExampleDisplay>>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let mut camera_transform = camera.single_mut();

    let rotation = if input.pressed(KeyCode::ArrowLeft) {
        time.delta_seconds()
    } else if input.pressed(KeyCode::ArrowRight) {
        -time.delta_seconds()
    } else {
        0.0
    };

    camera_transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(rotation));

    let mut light = light.single_mut();

    if input.just_pressed(KeyCode::KeyL) {
        light.illuminance = if light.illuminance == light_consts::lux::AMBIENT_DAYLIGHT {
            light_consts::lux::FULL_MOON_NIGHT
        } else {
            light_consts::lux::AMBIENT_DAYLIGHT
        };
    }

    let mut display = display.single_mut();
    display.sections[0].value = format!(
        "Exposure: {}\nLight Source Illuminance {}",
        0.0, light.illuminance
    );
}
