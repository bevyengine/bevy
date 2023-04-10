//! This example showcases light transmission
//!
//! ## Controls
//!
//! | Key Binding        | Action                                    |
//! |:-------------------|:------------------------------------------|
//! | `Q` / `W`          | Decrease / Increase Transmission          |
//! | `A` / `S`          | Decrease / Increase Thickness             |
//! | `Z` / `X`          | Decrease / Increase IOR                   |
//! | `Down` / `Up`      | Decrease / Increase Perceptual Roughness  |
//! | `Left` / `Right`   | Rotate Camera                             |
//! | `H`                | Toggle HDR                                |
//! | `C`                | Randomize Colors                          |

use bevy::prelude::*;
use rand::random;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, example_control_system);

    // Unfortunately, MSAA and HDR are not supported simultaneously under WebGL.
    // Since this example uses HDR, we must disable MSAA for WASM builds, at least
    // until WebGPU is ready and no longer behind a feature flag in Web browsers.
    #[cfg(target_arch = "wasm32")]
    app.insert_resource(Msaa::Off);

    app.run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let icosphere_mesh = meshes.add(
        Mesh::try_from(shape::Icosphere {
            radius: 0.9,
            subdivisions: 7,
        })
        .unwrap(),
    );

    // Opaque
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::rgba(0.9, 0.2, 0.3, 1.0),
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            transform: Transform::from_xyz(-1.0, 0.0, 0.0),
            ..default()
        },
        ExampleControls {
            color: true,
            transmission: false,
        },
    ));

    // Transmissive
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                transmission: 1.0,
                thickness: 1.0,
                ior: 1.5,
                perceptual_roughness: 0.12,
                ..default()
            }),
            transform: Transform::from_xyz(1.0, 0.0, 0.0),
            ..default()
        },
        ExampleControls {
            color: true,
            transmission: true,
        },
    ));

    // Chessboard Plane
    let black_material = materials.add(Color::BLACK.into());
    let white_material = materials.add(Color::WHITE.into());

    let plane_mesh = meshes.add(shape::Plane::from_size(2.0).into());

    for x in -3..4 {
        for z in -3..4 {
            commands.spawn((
                PbrBundle {
                    mesh: plane_mesh.clone(),
                    material: if (x + z) % 2 == 0 {
                        black_material.clone()
                    } else {
                        white_material.clone()
                    },
                    transform: Transform::from_xyz(x as f32 * 2.0, -1.0, z as f32 * 2.0),
                    ..default()
                },
                ExampleControls {
                    color: true,
                    transmission: false,
                },
            ));
        }
    }

    // Light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 1.5, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Controls Text
    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 18.0,
        color: Color::BLACK,
    };

    commands.spawn(
        TextBundle::from_section(
            "Q / W - Decrease / Increase Transmission\nA / S - Decrease / Increase Thickness\nZ / X - Decrease / Increase IOR\nDown / Up - Decrease / Increase Perceptual Roughness\nLeft / Right - Rotate Camera\nH - Toggle HDR\nC - Randomize Colors",
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
struct ExampleControls {
    transmission: bool,
    color: bool,
}

struct ExampleState {
    transmission: f32,
    thickness: f32,
    ior: f32,
    perceptual_roughness: f32,
}

#[derive(Component)]
struct ExampleDisplay;

impl Default for ExampleState {
    fn default() -> Self {
        ExampleState {
            transmission: 1.0,
            thickness: 1.0,
            ior: 1.5,
            perceptual_roughness: 0.12,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn example_control_system(
    mut materials: ResMut<Assets<StandardMaterial>>,
    controllable: Query<(&Handle<StandardMaterial>, &ExampleControls)>,
    mut camera: Query<(&mut Camera, &mut Transform), With<Camera3d>>,
    mut display: Query<&mut Text, With<ExampleDisplay>>,
    mut state: Local<ExampleState>,
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
) {
    if input.pressed(KeyCode::W) {
        state.transmission = (state.transmission + time.delta_seconds()).min(1.0);
    } else if input.pressed(KeyCode::Q) {
        state.transmission = (state.transmission - time.delta_seconds()).max(0.0);
    }

    if input.pressed(KeyCode::S) {
        state.thickness = (state.thickness + time.delta_seconds()).min(1.0);
    } else if input.pressed(KeyCode::A) {
        state.thickness = (state.thickness - time.delta_seconds()).max(0.0);
    }

    if input.pressed(KeyCode::X) {
        state.ior = (state.ior + time.delta_seconds()).min(3.0);
    } else if input.pressed(KeyCode::Z) {
        state.ior = (state.ior - time.delta_seconds()).max(1.0);
    }

    if input.pressed(KeyCode::Up) {
        state.perceptual_roughness = (state.perceptual_roughness + time.delta_seconds()).min(1.0);
    } else if input.pressed(KeyCode::Down) {
        state.perceptual_roughness = (state.perceptual_roughness - time.delta_seconds()).max(0.0);
    }

    let randomize_colors = input.just_pressed(KeyCode::C);

    for (material_handle, controls) in &controllable {
        let mut material = materials.get_mut(material_handle).unwrap();
        if controls.transmission {
            material.transmission = state.transmission;
            material.thickness = state.thickness;
            material.ior = state.ior;
            material.perceptual_roughness = state.perceptual_roughness;
        }

        if controls.color && randomize_colors {
            material.base_color.set_r(random());
            material.base_color.set_g(random());
            material.base_color.set_b(random());
        }
    }

    let (mut camera, mut camera_transform) = camera.single_mut();

    if input.just_pressed(KeyCode::H) {
        camera.hdr = !camera.hdr;
    }

    let rotation = if input.pressed(KeyCode::Left) {
        time.delta_seconds()
    } else if input.pressed(KeyCode::Right) {
        -time.delta_seconds()
    } else {
        0.0
    };

    camera_transform.rotate_around(
        Vec3::ZERO,
        Quat::from_euler(EulerRot::XYZ, 0.0, rotation, 0.0),
    );

    let mut display = display.single_mut();
    display.sections[0].value = format!(
        "HDR: {}\nTransmission: {:.2}\nThickness: {:.2}\nIOR: {:.2}\nPerceptual Roughness: {:.2}",
        if camera.hdr { "ON " } else { "OFF" },
        state.transmission,
        state.thickness,
        state.ior,
        state.perceptual_roughness,
    );
}
