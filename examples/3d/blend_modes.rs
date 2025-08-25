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

use bevy::{color::palettes::css::ORANGE, prelude::*, render::view::Hdr};
use rand::random;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, example_control_system);

    app.run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let base_color = Color::srgb(0.9, 0.2, 0.3);
    let icosphere_mesh = meshes.add(Sphere::new(0.9).mesh().ico(7).unwrap());

    // Opaque
    let opaque = commands
        .spawn((
            Mesh3d(icosphere_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            })),
            Transform::from_xyz(-4.0, 0.0, 0.0),
            ExampleControls {
                unlit: true,
                color: true,
            },
        ))
        .id();

    // Blend
    let blend = commands
        .spawn((
            Mesh3d(icosphere_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(-2.0, 0.0, 0.0),
            ExampleControls {
                unlit: true,
                color: true,
            },
        ))
        .id();

    // Premultiplied
    let premultiplied = commands
        .spawn((
            Mesh3d(icosphere_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color,
                alpha_mode: AlphaMode::Premultiplied,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, 0.0),
            ExampleControls {
                unlit: true,
                color: true,
            },
        ))
        .id();

    // Add
    let add = commands
        .spawn((
            Mesh3d(icosphere_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color,
                alpha_mode: AlphaMode::Add,
                ..default()
            })),
            Transform::from_xyz(2.0, 0.0, 0.0),
            ExampleControls {
                unlit: true,
                color: true,
            },
        ))
        .id();

    // Multiply
    let multiply = commands
        .spawn((
            Mesh3d(icosphere_mesh),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color,
                alpha_mode: AlphaMode::Multiply,
                ..default()
            })),
            Transform::from_xyz(4.0, 0.0, 0.0),
            ExampleControls {
                unlit: true,
                color: true,
            },
        ))
        .id();

    // Chessboard Plane
    let black_material = materials.add(Color::BLACK);
    let white_material = materials.add(Color::WHITE);

    let plane_mesh = meshes.add(Plane3d::default().mesh().size(2.0, 2.0));

    for x in -3..4 {
        for z in -3..4 {
            commands.spawn((
                Mesh3d(plane_mesh.clone()),
                MeshMaterial3d(if (x + z) % 2 == 0 {
                    black_material.clone()
                } else {
                    white_material.clone()
                }),
                Transform::from_xyz(x as f32 * 2.0, -1.0, z as f32 * 2.0),
                ExampleControls {
                    unlit: false,
                    color: true,
                },
            ));
        }
    }

    // Light
    commands.spawn((PointLight::default(), Transform::from_xyz(4.0, 8.0, 4.0)));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.5, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        Hdr,
        // Unfortunately, MSAA and HDR are not supported simultaneously under WebGL.
        // Since this example uses HDR, we must disable MSAA for Wasm builds, at least
        // until WebGPU is ready and no longer behind a feature flag in Web browsers.
        #[cfg(target_arch = "wasm32")]
        Msaa::Off,
    ));

    // Controls Text

    // We need the full version of this font so we can use box drawing characters.
    let text_style = TextFont {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        ..default()
    };

    let label_text_style = (text_style.clone(), TextColor(ORANGE.into()));

    commands.spawn((Text::new("Up / Down — Increase / Decrease Alpha\nLeft / Right — Rotate Camera\nH - Toggle HDR\nSpacebar — Toggle Unlit\nC — Randomize Colors"),
            text_style.clone(),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        })
    );

    commands.spawn((
        Text::default(),
        text_style,
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            right: px(12),
            ..default()
        },
        ExampleDisplay,
    ));

    let mut label = |entity: Entity, label: &str| {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ExampleLabel { entity },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(label),
                    label_text_style.clone(),
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::ZERO,
                        ..default()
                    },
                    TextLayout::default().with_no_wrap(),
                ));
            });
    };

    label(opaque, "┌─ Opaque\n│\n│\n│\n│");
    label(blend, "┌─ Blend\n│\n│\n│");
    label(premultiplied, "┌─ Premultiplied\n│\n│");
    label(add, "┌─ Add\n│");
    label(multiply, "┌─ Multiply");
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

fn example_control_system(
    mut materials: ResMut<Assets<StandardMaterial>>,
    controllable: Query<(&MeshMaterial3d<StandardMaterial>, &ExampleControls)>,
    camera: Single<
        (
            Entity,
            &mut Camera,
            &mut Transform,
            &GlobalTransform,
            Has<Hdr>,
        ),
        With<Camera3d>,
    >,
    mut labels: Query<(&mut Node, &ExampleLabel)>,
    mut display: Single<&mut Text, With<ExampleDisplay>>,
    labeled: Query<&GlobalTransform>,
    mut state: Local<ExampleState>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if input.pressed(KeyCode::ArrowUp) {
        state.alpha = (state.alpha + time.delta_secs()).min(1.0);
    } else if input.pressed(KeyCode::ArrowDown) {
        state.alpha = (state.alpha - time.delta_secs()).max(0.0);
    }

    if input.just_pressed(KeyCode::Space) {
        state.unlit = !state.unlit;
    }

    let randomize_colors = input.just_pressed(KeyCode::KeyC);

    for (material_handle, controls) in &controllable {
        let material = materials.get_mut(material_handle).unwrap();

        if controls.color && randomize_colors {
            material.base_color = Srgba {
                red: random(),
                green: random(),
                blue: random(),
                alpha: state.alpha,
            }
            .into();
        } else {
            material.base_color.set_alpha(state.alpha);
        }

        if controls.unlit {
            material.unlit = state.unlit;
        }
    }

    let (entity, camera, mut camera_transform, camera_global_transform, hdr) = camera.into_inner();

    if input.just_pressed(KeyCode::KeyH) {
        if hdr {
            commands.entity(entity).remove::<Hdr>();
        } else {
            commands.entity(entity).insert(Hdr);
        }
    }

    let rotation = if input.pressed(KeyCode::ArrowLeft) {
        time.delta_secs()
    } else if input.pressed(KeyCode::ArrowRight) {
        -time.delta_secs()
    } else {
        0.0
    };

    camera_transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(rotation));

    for (mut node, label) in &mut labels {
        let world_position = labeled.get(label.entity).unwrap().translation() + Vec3::Y;

        let viewport_position = camera
            .world_to_viewport(camera_global_transform, world_position)
            .unwrap();

        node.top = px(viewport_position.y);
        node.left = px(viewport_position.x);
    }

    display.0 = format!(
        "  HDR: {}\nAlpha: {:.2}",
        if hdr { "ON " } else { "OFF" },
        state.alpha
    );
}
