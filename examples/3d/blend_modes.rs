//! This example showcases different blend modes.
//!
//! ## Controls
//!
//! | Key Binding        | Action                              |
//! |:-------------------|:------------------------------------|
//! | `Up` / `Down`      | Increase / Decrease Alpha           |
//! | `Left` / `Right`   | Rotate Camera                       |
//! | `Spacebar`         | Toggle Unlit                        |
//! | `C`                | Randomize Colors                    |

use bevy::prelude::*;
use rand::random;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(example_control_system)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let base_color = Color::rgba(0.9, 0.2, 0.3, 1.0);
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
                base_color,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            transform: Transform::from_xyz(-4.0, 0.0, 0.0),
            ..default()
        },
        ExampleControls {
            unlit: true,
            color: true,
        },
    ));

    // Blend
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            transform: Transform::from_xyz(-2.0, 0.0, 0.0),
            ..default()
        },
        ExampleControls {
            unlit: true,
            color: true,
        },
    ));

    // Premultiplied
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color,
                alpha_mode: AlphaMode::Premultiplied,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        ExampleControls {
            unlit: true,
            color: true,
        },
    ));

    // Add
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color,
                alpha_mode: AlphaMode::Add,
                ..default()
            }),
            transform: Transform::from_xyz(2.0, 0.0, 0.0),
            ..default()
        },
        ExampleControls {
            unlit: true,
            color: true,
        },
    ));

    // Multiply
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color,
                alpha_mode: AlphaMode::Multiply,
                ..default()
            }),
            transform: Transform::from_xyz(4.0, 0.0, 0.0),
            ..default()
        },
        ExampleControls {
            unlit: true,
            color: true,
        },
    ));

    // Chessboard Plane
    let black_material = materials.add(Color::BLACK.into());
    let white_material = materials.add(Color::WHITE.into());
    let plane_mesh = meshes.add(shape::Plane { size: 2.0 }.into());

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
                    unlit: false,
                    color: true,
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
        transform: Transform::from_xyz(0.0, 2.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Controls Text
    commands.spawn(
        TextBundle::from_section(
            "Up / Down — Increase / Decrease Alpha\nLeft / Right — Rotate Camera\nSpacebar — Toggle Unlit\nC — Randomize Colors",
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 18.0,
                color: Color::BLACK,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            ..default()
        }),
    );
}

#[derive(Component)]
struct ExampleControls {
    unlit: bool,
    color: bool,
}

struct ExampleState {
    alpha: f32,
    unlit: bool,
}

impl Default for ExampleState {
    fn default() -> Self {
        ExampleState {
            alpha: 1.0,
            unlit: false,
        }
    }
}

fn example_control_system(
    mut materials: ResMut<Assets<StandardMaterial>>,
    controllable: Query<(&Handle<StandardMaterial>, &ExampleControls)>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut state: Local<ExampleState>,
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
) {
    if input.pressed(KeyCode::Up) {
        state.alpha = (state.alpha + time.delta_seconds()).min(1.0);
    } else if input.pressed(KeyCode::Down) {
        state.alpha = (state.alpha - time.delta_seconds()).max(0.0);
    }

    if input.just_pressed(KeyCode::Space) {
        state.unlit = !state.unlit;
    }

    let randomize_colors = input.just_pressed(KeyCode::C);

    for (material_handle, controls) in &controllable {
        let mut material = materials.get_mut(material_handle).unwrap();
        material.base_color.set_a(state.alpha);

        if controls.color {
            if randomize_colors {
                material.base_color.set_r(random());
                material.base_color.set_g(random());
                material.base_color.set_b(random());
            }
        }
        if controls.unlit {
            material.unlit = state.unlit;
        }
    }

    let mut camera_transform = camera.single_mut();

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
}
