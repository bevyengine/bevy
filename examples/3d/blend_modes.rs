//! This example showcases different blend modes.
//!
//! ## Controls
//!
//! | Key Binding        | Action                              |
//! |:-------------------|:------------------------------------|
//! | `Up` / `Down`      | Increase / Decrease Alpha           |
//! | `Left` / `Right`   | Rotate Camera                       |
//! | `H`                | Toggle HDR                          |
//! | `Spacebar`         | Toggle Unlit                        |
//! | `C`                | Randomize Colors                    |

use bevy::prelude::*;
use rand::random;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(example_control_system);

    // Unfortunately, MSAA and HDR are not supported simultaneously under WebGL.
    // Since this example uses HDR, we must disable MSAA for WASM builds, at least
    // until WebGPU is ready and no longer behind a feature flag in Web browsers.
    #[cfg(target_arch = "wasm32")]
    app.insert_resource(Msaa { samples: 1 }); // Default is 4 samples (MSAA on)

    app.run();
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
    let opaque = commands
        .spawn((
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
        ))
        .id();

    // Blend
    let blend = commands
        .spawn((
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
        ))
        .id();

    // Premultiplied
    let premultiplied = commands
        .spawn((
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
        ))
        .id();

    // Add
    let add = commands
        .spawn((
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
        ))
        .id();

    // Multiply
    let multiply = commands
        .spawn((
            PbrBundle {
                mesh: icosphere_mesh,
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
        ))
        .id();

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
        transform: Transform::from_xyz(0.0, 2.5, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Controls Text
    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 18.0,
        color: Color::BLACK,
    };

    let label_text_style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 25.0,
        color: Color::ORANGE,
    };

    commands.spawn(
        TextBundle::from_section(
            "Up / Down — Increase / Decrease Alpha\nLeft / Right — Rotate Camera\nH - Toggle HDR\nSpacebar — Toggle Unlit\nC — Randomize Colors",
            text_style.clone(),
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

    commands.spawn((
        TextBundle::from_section("", text_style).with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                ..default()
            },
            ..default()
        }),
        ExampleDisplay,
    ));

    commands.spawn((
        TextBundle::from_section("┌─ Opaque\n│\n│\n│\n│", label_text_style.clone()).with_style(
            Style {
                position_type: PositionType::Absolute,
                ..default()
            },
        ),
        ExampleLabel { entity: opaque },
    ));

    commands.spawn((
        TextBundle::from_section("┌─ Blend\n│\n│\n│", label_text_style.clone()).with_style(Style {
            position_type: PositionType::Absolute,
            ..default()
        }),
        ExampleLabel { entity: blend },
    ));

    commands.spawn((
        TextBundle::from_section("┌─ Premultiplied\n│\n│", label_text_style.clone()).with_style(
            Style {
                position_type: PositionType::Absolute,
                ..default()
            },
        ),
        ExampleLabel {
            entity: premultiplied,
        },
    ));

    commands.spawn((
        TextBundle::from_section("┌─ Add\n│", label_text_style.clone()).with_style(Style {
            position_type: PositionType::Absolute,
            ..default()
        }),
        ExampleLabel { entity: add },
    ));

    commands.spawn((
        TextBundle::from_section("┌─ Multiply", label_text_style).with_style(Style {
            position_type: PositionType::Absolute,
            ..default()
        }),
        ExampleLabel { entity: multiply },
    ));
}

#[derive(Component)]
struct ExampleControls {
    unlit: bool,
    color: bool,
}

#[derive(Component)]
struct ExampleLabel {
    entity: Entity,
}

struct ExampleState {
    alpha: f32,
    unlit: bool,
}

#[derive(Component)]
struct ExampleDisplay;

impl Default for ExampleState {
    fn default() -> Self {
        ExampleState {
            alpha: 0.9,
            unlit: false,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn example_control_system(
    mut materials: ResMut<Assets<StandardMaterial>>,
    controllable: Query<(&Handle<StandardMaterial>, &ExampleControls)>,
    mut camera: Query<(&mut Camera, &mut Transform, &GlobalTransform), With<Camera3d>>,
    mut labels: Query<(&mut Style, &ExampleLabel)>,
    mut display: Query<&mut Text, With<ExampleDisplay>>,
    labelled: Query<&GlobalTransform>,
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

        if controls.color && randomize_colors {
            material.base_color.set_r(random());
            material.base_color.set_g(random());
            material.base_color.set_b(random());
        }
        if controls.unlit {
            material.unlit = state.unlit;
        }
    }

    let (mut camera, mut camera_transform, camera_global_transform) = camera.single_mut();

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

    for (mut style, label) in &mut labels {
        let world_position =
            labelled.get(label.entity).unwrap().translation() + Vec3::new(0.0, 1.0, 0.0);

        let viewport_position = camera
            .world_to_viewport(camera_global_transform, world_position)
            .unwrap();

        style.position.bottom = Val::Px(viewport_position.y);
        style.position.left = Val::Px(viewport_position.x);
    }

    let mut display = display.single_mut();
    display.sections[0].value = format!(
        "  HDR: {}\nAlpha: {:.2}",
        if camera.hdr { "ON " } else { "OFF" },
        state.alpha
    );
}
